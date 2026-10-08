//! The operations against a real collection.
//!
//! These need a MongoDB server, named in `TIXSCHEMA_MONGODB_URI`. Without the variable every check
//! stands down, saying so on the process's own stderr; `just test-mongodb` refuses to. With the
//! variable set, a value that is no address or a server that does not answer is a failure. The
//! address is never printed: it may hold a password.
//!
//! Each check works in a collection of its own in the database `tixschema_live`, named after the
//! check, the `bson` major and the process, so the two majors can run at once against one server.

use core::time::Duration;
use std::env;
use std::io::Write as _;
use std::io::stderr;
use std::process::id;
use std::sync::Once;

use bson::{Bson, Document, doc};
use mongodb::options::{ClientOptions, Collation, CollationStrength, Hint};
use mongodb::{Client, Collection};

use super::invoice_schema::OperationError;
use super::{
    Invoice, InvoiceStatus, every_row, invoice, names_the_unreadable_row, numbered, readable,
    seeded_id, stored, unreadable, whole_number_as_text,
};
use crate::BSON_MAJOR;

const DATABASE: &str = "tixschema_live";

/// When the seeded invoice was paid, as the checks that pay it store it.
const PAID_AT: &str = "2026-10-07T17:40:00Z";

/// How long a named server is given to answer before the check fails.
const PATIENCE: Duration = Duration::from_secs(5);

const SERVER_VAR: &str = "TIXSCHEMA_MONGODB_URI";

static STOOD_DOWN: Once = Once::new();

/// An empty collection of its own for the check `named`, or `None` where no server is named.
pub(super) async fn collection(named: &str) -> Option<Collection<Document>> {
    let address = named_server()?;
    let parsed = ClientOptions::parse(&address).await;
    assert!(
        parsed.is_ok(),
        "{SERVER_VAR} is set, and what it holds is no MongoDB address: {}",
        parsed.unwrap_err()
    );
    let mut options = parsed.unwrap();
    options.server_selection_timeout = Some(PATIENCE);
    let held = Client::with_options(options)
        .unwrap()
        .database(DATABASE)
        .collection::<Document>(&format!("{named}_bson{BSON_MAJOR}_{run}", run = id()));
    let emptied = held.drop().await;
    assert!(
        emptied.is_ok(),
        "{SERVER_VAR} is set, and no server answered at the address it holds: {}",
        emptied.unwrap_err()
    );
    Some(held)
}

/// The address `TIXSCHEMA_MONGODB_URI` holds, or `None`, standing down, where it is not set.
fn named_server() -> Option<String> {
    let named = env::var(SERVER_VAR).ok();
    if named.is_none() {
        stand_down();
    }
    named
}

pub(super) fn say(answered: &str) {
    eprintln!("[bson {BSON_MAJOR}, live] {answered}");
}

/// A collection of its own for the check `named` holding `rows`, stored by the driver as they
/// stand, or `None` where no server is named.
async fn seeded(named: &str, rows: Vec<Document>) -> Option<Collection<Document>> {
    let held = collection(named).await?;
    held.insert_many(rows).await.unwrap();
    Some(held)
}

/// Said on the process's own stderr, which `cargo test` does not capture: a stand-down is a pass
/// that proved nothing, so it has to show on a run where everything passed.
fn stand_down() {
    STOOD_DOWN.call_once(|| {
        let notice = format!(
            "\ntixschema: {SERVER_VAR} names no MongoDB server, so the operations were NOT run \
             against a collection (bson {BSON_MAJOR}).\n  Set {SERVER_VAR} to a server's address, \
             as in `mongodb://127.0.0.1:27017`, and run `just test-mongodb`, which refuses to \
             stand down.\n\n"
        );
        drop(stderr().write_all(notice.as_bytes()));
    });
}

/// A read is run under the options it is given: the order, how many rows and how many passed
/// over, on every kind of read. A collation changes what a filter matches, and a hint reaches
/// MongoDB, which refuses one that names no index.
#[tokio::test]
async fn live_a_read_is_run_under_the_options_it_is_given() {
    let rows = (1_u32..=4_u32).map(|at| stored(&numbered(at))).collect();
    let Some(held) = seeded("read_options", rows).await else {
        return;
    };
    let largest_first = || doc! { "total": -1_i32 };

    let largest = Invoice::find(&held, every_row())
        .sort(largest_first())
        .limit(2_i64)
        .await
        .unwrap();
    say(&format!(
        "find sorted by total, limited to 2, answers {largest:?}"
    ));
    assert_eq!(
        largest,
        [numbered(4_u32), numbered(3_u32)],
        "the two largest, largest first"
    );
    let smallest = Invoice::find(&held, every_row())
        .sort(largest_first())
        .skip(3_u64)
        .await
        .unwrap();
    assert_eq!(smallest, [numbered(1_u32)], "three rows are passed over");
    let second = Invoice::find_one(&held, every_row())
        .sort(largest_first())
        .skip(1_u64)
        .await
        .unwrap();
    assert_eq!(second, Some(numbered(3_u32)), "the second largest");
    let counted = Invoice::count(&held, every_row())
        .skip(1_u64)
        .limit(2_i64)
        .await
        .unwrap();
    assert_eq!(counted, 2_u64, "a count is held to its limit");

    let lowercase = || {
        Invoice::MONGO_FIELDS
            .number
            .eq("inv-0001".to_owned())
            .unwrap()
    };
    let exact = Invoice::find(&held, lowercase()).await.unwrap();
    assert!(exact.is_empty(), "no number is written in lowercase");
    let folded = Invoice::find(&held, lowercase())
        .collation(
            Collation::builder()
                .locale("en")
                .strength(CollationStrength::Secondary)
                .build(),
        )
        .await
        .unwrap();
    assert_eq!(folded, [numbered(1_u32)], "the collation folds the case");

    let hinted = Invoice::count(&held, every_row())
        .hint(Hint::Name("_id_".to_owned()))
        .await
        .unwrap();
    assert_eq!(hinted, 4_u64, "the index every collection has");
    let refused = Invoice::find(&held, every_row())
        .hint(Hint::Name("no_such_index".to_owned()))
        .await
        .unwrap_err();
    say(&format!("a hint that names no index answers: {refused}"));
    assert!(
        matches!(refused, OperationError::Database(_)),
        "a hint MongoDB refuses was not reported as a database error: {refused}"
    );
    held.drop().await.unwrap();
}

/// A row an older writer left is refused by `find_one`, by its `_id` and its one issue, and read
/// by `find_one_with` once a resolver has repaired it.
#[tokio::test]
async fn live_a_row_a_resolver_repairs_reads_and_is_refused_without_one() {
    let Some(held) = seeded("find_one_with", vec![unreadable()]).await else {
        return;
    };
    let paths = Invoice::MONGO_FIELDS;
    let numbered_42 = || paths.number.eq("INV-0042".to_owned()).unwrap();

    let repaired = Invoice::find_one_with(&held, numbered_42(), &[&whole_number_as_text])
        .await
        .unwrap();
    say(&format!(
        "find_one_with and the resolver answers {repaired:?}"
    ));
    assert_eq!(
        repaired,
        Some(invoice()),
        "the resolver's repair is what is read"
    );

    let refused = Invoice::find_one(&held, numbered_42()).await.unwrap_err();
    say(&format!(
        "find_one on the unreadable row answers: {refused}"
    ));
    assert!(
        names_the_unreadable_row(&refused),
        "the unreadable row was not reported by its `_id` and its one issue: {refused:?}"
    );
    held.drop().await.unwrap();
}

/// A stored row reads as the type, by a filter over a path of its own and by one that starts from
/// a nested model's path, and a filter that matches no row answers `None`.
#[tokio::test]
async fn live_a_seeded_row_reads_and_no_row_is_none() {
    let Some(held) = seeded("find_one", vec![readable()]).await else {
        return;
    };
    let paths = Invoice::MONGO_FIELDS;

    let read = Invoice::find_one(&held, paths.number.eq("INV-0042".to_owned()).unwrap())
        .await
        .unwrap();
    say(&format!("find_one answers {read:?}"));
    assert_eq!(read, Some(invoice()), "the seeded row reads as the type");

    let by_customer = Invoice::find_one(&held, paths.customer.name.eq("Acme".to_owned()).unwrap())
        .await
        .unwrap();
    assert_eq!(
        by_customer,
        Some(invoice()),
        "a filter from a nested model's path finds the row"
    );

    let absent = Invoice::find_one(&held, paths.number.eq("INV-9999".to_owned()).unwrap())
        .await
        .unwrap();
    assert_eq!(absent, None, "a filter matching no row answers `None`");
    held.drop().await.unwrap();
}

/// An inserted value reads back as the value it was, under the `_id` it declares.
#[tokio::test]
async fn live_an_inserted_row_reads_back_as_the_value_it_was() {
    let Some(held) = collection("insert_one").await else {
        return;
    };
    let inserted = invoice().insert_one(&held).await.unwrap();
    say(&format!("insert_one answers {inserted:?}"));
    assert_eq!(
        inserted.inserted_id,
        Bson::ObjectId(seeded_id()),
        "the answer names the row's own `_id`"
    );
    let read = Invoice::find_one(&held, Invoice::MONGO_FIELDS.id.eq(seeded_id()).unwrap())
        .await
        .unwrap();
    assert_eq!(read, Some(invoice()), "the row read back is the row stored");
    held.drop().await.unwrap();
}

/// Nothing listens on port 1 of this machine, and the driver is given a fifth of a second to find
/// that out, once per operation. No row is asked of the server `TIXSCHEMA_MONGODB_URI` names.
#[tokio::test]
async fn live_an_unreachable_server_is_a_database_error_for_every_operation() {
    if named_server().is_none() {
        return;
    }
    let mut options = ClientOptions::parse("mongodb://127.0.0.1:1").await.unwrap();
    options.server_selection_timeout = Some(Duration::from_millis(200));
    let closed = Client::with_options(options)
        .unwrap()
        .database(DATABASE)
        .collection::<Document>("unreachable");
    let paths = Invoice::MONGO_FIELDS;
    let matching = || paths.number.eq("INV-0042".to_owned()).unwrap();
    let paid = || paths.status.set(InvoiceStatus::Paid).unwrap();

    let refusals = [
        (
            "find_one",
            Invoice::find_one(&closed, matching()).await.err(),
        ),
        (
            "find_one_with",
            Invoice::find_one_with(&closed, matching(), &[&whole_number_as_text])
                .await
                .err(),
        ),
        ("find", Invoice::find(&closed, matching()).await.err()),
        (
            "find_with",
            Invoice::find_with(&closed, matching(), &[&whole_number_as_text])
                .await
                .err(),
        ),
        ("count", Invoice::count(&closed, matching()).await.err()),
        ("insert_one", invoice().insert_one(&closed).await.err()),
        (
            "update_one",
            Invoice::update_one(&closed, matching(), paid()).await.err(),
        ),
        (
            "update_many",
            Invoice::update_many(&closed, matching(), paid())
                .await
                .err(),
        ),
        (
            "delete_one",
            Invoice::delete_one(&closed, matching()).await.err(),
        ),
        (
            "delete_many",
            Invoice::delete_many(&closed, matching()).await.err(),
        ),
    ];
    for (operation, refused) in refusals {
        say(&format!(
            "{operation} on a closed port answers: {refused:?}"
        ));
        assert!(
            matches!(refused, Some(OperationError::Database(_))),
            "{operation} on a server that cannot be reached was not a database error: {refused:?}"
        );
    }
}

/// A count answers how many rows its filter matches, with no row read: two past due, three
/// unpaid, none under a number no row has.
#[tokio::test]
async fn live_count_answers_how_many_rows_a_filter_matches() {
    let draft = Invoice {
        status: InvoiceStatus::Draft,
        ..numbered(3_u32)
    };
    let paid = Invoice {
        paid_at: Some(PAID_AT.to_owned()),
        status: InvoiceStatus::Paid,
        ..numbered(4_u32)
    };
    let rows = [numbered(1_u32), numbered(2_u32), draft, paid]
        .iter()
        .map(stored)
        .collect();
    let Some(held) = seeded("count", rows).await else {
        return;
    };
    let paths = Invoice::MONGO_FIELDS;

    let past_due = Invoice::count(&held, paths.status.eq(InvoiceStatus::PastDue).unwrap())
        .await
        .unwrap();
    say(&format!("count answers {past_due:?}"));
    assert_eq!(past_due, 2_u64, "the two past due");
    let unpaid = Invoice::count(&held, paths.paid_at.exists(false))
        .await
        .unwrap();
    assert_eq!(unpaid, 3_u64, "every row but the paid one");
    let none = Invoice::count(&held, paths.number.eq("INV-9999".to_owned()).unwrap())
        .await
        .unwrap();
    assert_eq!(none, 0_u64, "a filter matching no row counts none");
    held.drop().await.unwrap();
}

/// 252 rows, so that the cursor has to ask the server for more wherever a first batch holds fewer.
#[tokio::test]
async fn live_find_reads_every_row_the_cursor_holds() {
    let every: Vec<Invoice> = (1_u32..=252_u32).map(numbered).collect();
    let Some(held) = seeded("find", every.iter().map(stored).collect()).await else {
        return;
    };
    let unordered = Invoice::find(&held, every_row()).await.unwrap();
    say(&format!("find answers {} rows", unordered.len()));
    assert_eq!(unordered.len(), 252_usize, "every row is read");

    let ordered = Invoice::find(&held, every_row())
        .sort(doc! { "number": 1_i32 })
        .await
        .unwrap();
    assert_eq!(ordered, every, "each row is read once, as the type");

    let none = Invoice::find(
        &held,
        Invoice::MONGO_FIELDS
            .number
            .eq("INV-9999".to_owned())
            .unwrap(),
    )
    .await
    .unwrap();
    assert!(none.is_empty(), "a filter matching no row answers no rows");
    held.drop().await.unwrap();
}

/// One row an older writer left, between two that read: `find` answers no row at all, and names
/// the one that does not read. With a resolver that repairs it, `find_with` answers all three.
#[tokio::test]
async fn live_one_unreadable_row_refuses_every_row_until_a_resolver_repairs_it() {
    let rows = vec![
        stored(&numbered(1_u32)),
        unreadable(),
        stored(&numbered(2_u32)),
    ];
    let Some(held) = seeded("find_with", rows).await else {
        return;
    };

    let refused = Invoice::find(&held, every_row()).await.unwrap_err();
    say(&format!(
        "find over a collection holding one unreadable row answers: {refused}"
    ));
    assert!(
        names_the_unreadable_row(&refused),
        "one unreadable row did not refuse the whole read by its `_id` and its issue: {refused:?}"
    );

    let repaired = Invoice::find_with(&held, every_row(), &[&whole_number_as_text])
        .sort(doc! { "number": 1_i32 })
        .await
        .unwrap();
    say(&format!(
        "find_with and the resolver answers {} rows",
        repaired.len()
    ));
    assert_eq!(
        repaired,
        [numbered(1_u32), numbered(2_u32), invoice()],
        "every row reads, the repaired one among them"
    );
    held.drop().await.unwrap();
}

/// `update_many` changes every row its filter matches, `delete_one` takes out one, and
/// `delete_many` every one: two drafts and two past due leave one row.
#[tokio::test]
async fn live_update_many_and_the_deletes_act_on_the_rows_their_filters_match() {
    let draft = |at: u32| Invoice {
        status: InvoiceStatus::Draft,
        ..numbered(at)
    };
    let rows = [draft(1_u32), draft(2_u32), numbered(3_u32), numbered(4_u32)]
        .iter()
        .map(stored)
        .collect();
    let Some(held) = seeded("update_many", rows).await else {
        return;
    };
    let paths = Invoice::MONGO_FIELDS;
    let of = |status: InvoiceStatus| paths.status.eq(status).unwrap();

    let voided = Invoice::update_many(
        &held,
        of(InvoiceStatus::Draft),
        paths.total.set(0_u32).unwrap(),
    )
    .await
    .unwrap();
    say(&format!("update_many answers {voided:?}"));
    assert_eq!(
        (
            voided.matched_count,
            voided.modified_count,
            voided.upserted_id
        ),
        (2_u64, 2_u64, None),
        "both drafts matched and changed"
    );
    let voided_rows = Invoice::count(&held, paths.total.eq(0_u32).unwrap())
        .await
        .unwrap();
    assert_eq!(voided_rows, 2_u64, "no other row was changed");

    let one = Invoice::delete_one(&held, of(InvoiceStatus::PastDue))
        .await
        .unwrap();
    say(&format!("delete_one answers {one:?}"));
    assert_eq!(one.deleted_count, 1_u64, "one of the two past due");
    let drafts = Invoice::delete_many(&held, of(InvoiceStatus::Draft))
        .await
        .unwrap();
    say(&format!("delete_many answers {drafts:?}"));
    assert_eq!(drafts.deleted_count, 2_u64, "both drafts");
    let left = Invoice::count(&held, every_row()).await.unwrap();
    assert_eq!(left, 1_u64, "one past due is left");
    held.drop().await.unwrap();
}

/// `update_one` changes the one row its filter matches, by an update built from the type's own
/// paths, and the row then reads with what the update set.
#[tokio::test]
async fn live_update_one_changes_the_row_its_filter_matches() {
    let rows = vec![stored(&invoice()), stored(&numbered(1_u32))];
    let Some(held) = seeded("update_one", rows).await else {
        return;
    };
    let paths = Invoice::MONGO_FIELDS;
    let numbered_42 = || paths.number.eq("INV-0042".to_owned()).unwrap();

    let paid = paths
        .status
        .set(InvoiceStatus::Paid)
        .unwrap()
        .and(paths.paid_at.set(PAID_AT.to_owned()).unwrap());
    let result = Invoice::update_one(&held, numbered_42(), paid)
        .await
        .unwrap();
    say(&format!("update_one answers {result:?}"));
    assert_eq!(
        (
            result.matched_count,
            result.modified_count,
            result.upserted_id
        ),
        (1_u64, 1_u64, None),
        "one row matched and changed"
    );
    let read = Invoice::find_one(&held, numbered_42()).await.unwrap();
    assert_eq!(
        read,
        Some(Invoice {
            paid_at: Some(PAID_AT.to_owned()),
            status: InvoiceStatus::Paid,
            ..invoice()
        }),
        "the row reads as the update left it"
    );
    let past_due = Invoice::count(&held, paths.status.eq(InvoiceStatus::PastDue).unwrap())
        .await
        .unwrap();
    assert_eq!(past_due, 1_u64, "the other row is as it was");
    held.drop().await.unwrap();
}
