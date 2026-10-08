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

use bson::{Bson, Document};
use mongodb::options::ClientOptions;
use mongodb::{Client, Collection};

use super::invoice_schema::{Expected, Issue, OperationError};
use super::{
    Invoice, SEEDED_ID, invoice, open_invoices, readable, unreadable, whole_number_as_text,
};
use crate::BSON_MAJOR;

const DATABASE: &str = "tixschema_live";

/// How long a named server is given to answer before the check fails.
const PATIENCE: Duration = Duration::from_secs(5);

const SERVER_VAR: &str = "TIXSCHEMA_MONGODB_URI";

static STOOD_DOWN: Once = Once::new();

/// An empty collection of its own for the check `named`, or `None` where no server is named.
async fn collection(named: &str) -> Option<Collection<Document>> {
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

fn say(answered: &str) {
    eprintln!("[bson {BSON_MAJOR}, live] {answered}");
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

/// A row an older writer left is refused by `find_one`, by its `_id` and its one issue, and read
/// by `find_one_with` once a resolver has repaired it.
#[tokio::test]
async fn live_a_row_a_resolver_repairs_reads_and_is_refused_without_one() {
    let Some(held) = collection("find_one_with").await else {
        return;
    };
    held.insert_one(unreadable()).await.unwrap();
    let paths = Invoice::MONGO_FIELDS;
    let numbered = || paths.number.eq("INV-0042".to_owned()).unwrap();

    let repaired = Invoice::find_one_with(&held, numbered(), &[&whole_number_as_text])
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

    let refused = Invoice::find_one(&held, numbered()).await.unwrap_err();
    say(&format!(
        "find_one on the unreadable row answers: {refused}"
    ));
    assert!(
        matches!(
            &refused,
            OperationError::Unreadable { row, issues }
                if *row == format!("ObjectId(\"{SEEDED_ID}\")")
                    && matches!(
                        issues.as_slice(),
                        [Issue::Invalid {
                            path,
                            expected: Expected::U32,
                            found: Bson::String(text),
                            reason: _reason,
                        }] if *path == open_invoices() && text == "3"
                    )
        ),
        "the unreadable row was not reported by its `_id` and its one issue: {refused:?}"
    );
    held.drop().await.unwrap();
}

/// A stored row reads as the type, by a filter over a path of its own and by one that starts from
/// a nested model's path, and a filter that matches no row answers `None`.
#[tokio::test]
async fn live_a_seeded_row_reads_and_no_row_is_none() {
    let Some(held) = collection("find_one").await else {
        return;
    };
    held.insert_one(readable()).await.unwrap();
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

/// Nothing listens on port 1 of this machine, and the driver is given a fifth of a second to find
/// that out. No row is asked of the server `TIXSCHEMA_MONGODB_URI` names.
#[tokio::test]
async fn live_an_unreachable_server_is_a_database_error() {
    if named_server().is_none() {
        return;
    }
    let mut options = ClientOptions::parse("mongodb://127.0.0.1:1").await.unwrap();
    options.server_selection_timeout = Some(Duration::from_millis(200));
    let closed = Client::with_options(options)
        .unwrap()
        .database(DATABASE)
        .collection::<Document>("unreachable");
    let filter = Invoice::MONGO_FIELDS
        .number
        .eq("INV-0042".to_owned())
        .unwrap();
    let refused = Invoice::find_one(&closed, filter).await.unwrap_err();
    say(&format!("find_one on a closed port answers: {refused}"));
    assert!(
        matches!(refused, OperationError::Database(_)),
        "a server that cannot be reached was not reported as a database error: {refused}"
    );
}
