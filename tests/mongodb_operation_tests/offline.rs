//! What needs no MongoDB server: the error every operation fails with, the read of a row once the
//! driver has handed it over, a row refused before it is sent, and what a read holds before it is
//! awaited. No check here opens a connection.

use core::cell::Cell;
use core::error::Error;

use bson::{Bson, Document, doc};
use mongodb::options::{ClientOptions, Collation, Hint};
use mongodb::{Client, Collection};
use serde::{Deserialize, Serialize};
use tixschema::model_schema;

use super::invoice_schema::{Field, Filter, MongoPath, OperationError, Resolver};
use super::{
    Customer, Invoice, InvoiceStatus, SEEDED_ID, customer_schema, invoice, invoice_status_schema,
    names_the_unreadable_row, readable, seeded_id, unreadable, whole_number_as_text,
};

/// A row whose total is of whatever type fills `T`: an unsigned one can hold more than BSON has
/// a number for.
#[model_schema(decode_with, default_types(T = u32))]
#[derive(Debug, Deserialize, Serialize)]
struct Ledger<T> {
    total: T,
}

/// A row that crosses threads and that no two threads share: what it has seen is kept in a
/// `Cell`, off the wire.
#[model_schema(decode_with)]
#[derive(Debug, Default, Deserialize, Serialize)]
struct Tally {
    #[serde(skip)]
    seen: Cell<u32>,
    total: u32,
}

/// Compiles only where what awaits an operation on a row no two threads share can be moved to
/// another thread: the future of an insert holds no borrow of the row.
fn a_row_no_two_threads_share_is_sendable(held: &Collection<Document>) {
    let every = || tally_schema::Filter::<Tally>::raw(doc! {});
    sendable(&Tally::default().insert_one(held));
    sendable(&Tally::find(held, every()).into_future());
    sendable(&Tally::find_one(held, every()).into_future());
    sendable(&Tally::count(held, every()).into_future());
}

/// Compiles only where a row with a type parameter gets the operations at what fills it, and
/// what awaits each can be moved to another thread.
fn a_row_with_a_type_parameter_is_sendable(held: &Collection<Document>) {
    let every = || ledger_schema::Filter::<Ledger<u64>>::raw(doc! {});
    sendable(&Ledger { total: 7_u64 }.insert_one(held));
    sendable(&Ledger::<u64>::find(held, every()).into_future());
    sendable(&Ledger::<u64>::count(held, every()).into_future());
    sendable(&Ledger::<u64>::delete_many(held, every()));
}

/// Compiles only where `checked` does. It is never called, so no collection is asked for.
const fn compiles<C>(_checked: &C)
where
    C: Fn(&Collection<Document>),
{
}

/// Compiles only where the error crosses threads and borrows nothing.
const fn crosses_threads<T>()
where
    T: Error + Send + Sync + 'static,
{
}

/// Compiles only where each operation can be moved to another thread: a read as it is answered
/// and as it is awaited, and the future of each write.
fn every_operation_is_sendable(held: &Collection<Document>) {
    let resolvers: [Resolver<'_, Document, Bson>; 1] = [&whole_number_as_text];
    let paid = || {
        Invoice::MONGO_FIELDS
            .status
            .set(InvoiceStatus::Paid)
            .unwrap()
    };
    sendable(&Invoice::find_one(held, numbered()));
    sendable(&Invoice::find_one(held, numbered()).into_future());
    sendable(&Invoice::find_one_with(held, numbered(), &resolvers));
    sendable(&Invoice::find_one_with(held, numbered(), &resolvers).into_future());
    sendable(&Invoice::find(held, numbered()));
    sendable(&Invoice::find(held, numbered()).into_future());
    sendable(&Invoice::find_with(held, numbered(), &resolvers));
    sendable(&Invoice::find_with(held, numbered(), &resolvers).into_future());
    sendable(&Invoice::count(held, numbered()));
    sendable(&Invoice::count(held, numbered()).into_future());
    sendable(&invoice().insert_one(held));
    sendable(&Invoice::update_one(held, numbered(), paid()));
    sendable(&Invoice::update_many(held, numbered(), paid()));
    sendable(&Invoice::delete_one(held, numbered()));
    sendable(&Invoice::delete_many(held, numbered()));
}

/// A collection of a client that opens no connection until an operation asks it for one: a
/// load-balanced client watches no server.
async fn never_connected() -> Collection<Document> {
    let options = ClientOptions::parse("mongodb://127.0.0.1:1/?loadBalanced=true")
        .await
        .unwrap();
    Client::with_options(options)
        .unwrap()
        .database("tixschema_offline")
        .collection("never_asked")
}

fn numbered() -> Filter<Invoice> {
    Invoice::MONGO_FIELDS
        .number
        .eq("INV-0042".to_owned())
        .unwrap()
}

/// A filter over a `u64` at its largest, which BSON has no type for, refused through `?`.
fn over_the_limit() -> Result<Filter<Invoice>, OperationError> {
    let limit: Field<Invoice, u64> = Field::plain(MongoPath::under(MongoPath::ROOT, "limit"));
    Ok(limit.eq(u64::MAX)?)
}

const fn sendable<T>(_held: &T)
where
    T: Send,
{
}

/// The driver refuses a connection string with no scheme before it opens any connection.
#[tokio::test]
async fn a_driver_error_is_told_as_a_database_error_and_is_its_source() {
    let refused = ClientOptions::parse("not an address").await.unwrap_err();
    let told = refused.to_string();
    let failed = OperationError::Database(refused);
    assert_eq!(
        failed.to_string(),
        format!("MongoDB refused the operation or could not be reached: {told}")
    );
    assert_eq!(failed.source().map(ToString::to_string), Some(told));
}

/// The resolver that settles it takes the `_id` out of the row before the second read.
#[test]
fn a_type_with_no_id_refuses_a_stored_row_until_a_resolver_settles_it() {
    use customer_schema::{Issue as Found, Path, Resolution, Segment};

    let stored = doc! { "_id": seeded_id(), "name": "Acme", "openInvoices": 3_i64 };
    let refused = Customer::mongo_read_row(stored.clone(), &[]).unwrap_err();
    assert!(
        matches!(
            &refused,
            customer_schema::OperationError::Unreadable { row, issues }
                if *row == format!("ObjectId(\"{SEEDED_ID}\")")
                    && *issues == [Found::Unknown {
                        path: Path(vec![Segment::Key("_id".to_owned())]),
                        found: Bson::ObjectId(seeded_id()),
                    }]
        ),
        "got: {refused:?}"
    );

    let unkeyed = |raw: &mut Document, issue: &Found<Bson>| {
        let Found::Unknown {
            path,
            found: _found,
        } = issue
        else {
            return Resolution::NotTouched;
        };
        if path.remove_from_document(raw) {
            Resolution::Settled
        } else {
            Resolution::Rejected
        }
    };
    assert_eq!(
        Customer::mongo_read_row(stored, &[&unkeyed]).unwrap(),
        invoice().customer
    );
}

#[tokio::test]
async fn a_row_declared_inside_a_function_reads_and_writes_itself() {
    #[model_schema(decode_with)]
    #[derive(Debug, Deserialize, PartialEq, Eq, Serialize)]
    struct Note {
        text: String,
    }

    let kept = || Note {
        text: "kept".to_owned(),
    };
    assert_eq!(kept().mongo_written_row().unwrap(), doc! { "text": "kept" });
    assert_eq!(
        Note::mongo_read_row(doc! { "text": "kept" }, &[]).unwrap(),
        kept()
    );

    let never = never_connected().await;
    let filter = || Note::MONGO_FIELDS.text.eq("kept".to_owned()).unwrap();
    let read: note_schema::Read<'_, Note, Vec<Note>> = Note::find(&never, filter()).limit(1_i64);
    assert_eq!(
        format!("{read:?}"),
        format!(
            "Read {{ filter: {:?}, sort: None, limit: Some(1), skip: None, hint: None, \
             collation: None, resolvers: 0, .. }}",
            Document::from(filter())
        )
    );
}

#[test]
fn a_stored_row_reads_as_the_operations_read_it() {
    assert_eq!(
        invoice().serialize(bson::Serializer::new()).unwrap(),
        Bson::Document(readable()),
        "the row the type writes is the row the checks seed"
    );
    assert_eq!(Invoice::mongo_read_row(readable(), &[]).unwrap(), invoice());
    assert_eq!(
        Invoice::mongo_read_row(unreadable(), &[&whole_number_as_text]).unwrap(),
        invoice()
    );
    let refused = Invoice::mongo_read_row(unreadable(), &[]).unwrap_err();
    assert!(names_the_unreadable_row(&refused), "got: {refused:?}");
}

/// serde reads a name that breaks its bound.
#[test]
fn a_row_that_breaks_a_bound_is_unreadable_until_a_resolver_repairs_it() {
    let mut nameless = readable();
    nameless
        .get_document_mut("customer")
        .unwrap()
        .insert("name", "");
    let read = Invoice::mongo_read_row(nameless.clone(), &[]);
    #[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
    {
        use super::invoice_schema::{Expected, Issue, Path, Resolution, Segment};

        let refused = read.unwrap_err();
        assert!(
            matches!(
                &refused,
                OperationError::Unreadable { row, issues }
                    if *row == format!("ObjectId(\"{SEEDED_ID}\")")
                        && *issues == [Issue::Invalid {
                            path: Path(vec![
                                Segment::Key("customer".to_owned()),
                                Segment::Key("name".to_owned()),
                            ]),
                            expected: Expected::String,
                            found: Bson::String(String::new()),
                            reason: "too short: minimum length is 1, got 0".to_owned(),
                        }]
            ),
            "got: {refused:?}"
        );
        assert_eq!(
            refused.to_string(),
            format!(
                "the row ObjectId(\"{SEEDED_ID}\") does not read as expected: customer.name: \
                 invalid: expected String, found String(\"\"): too short: minimum length is 1, \
                 got 0"
            )
        );

        let unnamed = |raw: &mut Document, issue: &Issue<Bson>| {
            let Issue::Invalid {
                path,
                expected: Expected::String,
                found: Bson::String(held),
                reason: _reason,
            } = issue
            else {
                return Resolution::NotTouched;
            };
            let named =
                held.is_empty() && path.set_in_document(raw, Bson::String("unnamed".to_owned()));
            if named {
                Resolution::Settled
            } else {
                Resolution::NotTouched
            }
        };
        let repaired = Invoice::mongo_read_row(nameless, &[&unnamed]).unwrap();
        assert_eq!(repaired.customer.name, "unnamed");
    }
    #[cfg(not(any(feature = "typescript", feature = "zod", feature = "jsonschema")))]
    assert_eq!(read.unwrap().customer.name, "");
}

#[test]
fn an_unreadable_row_is_told_by_its_id_and_its_issues_and_has_no_source() {
    let refused = Invoice::mongo_read_row(unreadable(), &[]).unwrap_err();
    let told = refused.to_string();
    assert!(
        told.starts_with(
            "the row ObjectId(\"6a7cc592ca0574e6efdfe217\") does not read as expected: \
             customer.openInvoices: invalid: expected U32, found String(\"3\"): "
        ),
        "got: {told}"
    );
    let unrecovered = Invoice::from_bson_piped(unreadable(), &[]).unwrap_err();
    assert_eq!(
        told,
        format!("the row ObjectId(\"{SEEDED_ID}\") does not read as expected: {unrecovered}")
    );
    assert!(refused.source().is_none(), "got: {:?}", refused.source());
}

#[tokio::test]
async fn a_read_holds_the_options_it_is_given() {
    let never = never_connected().await;
    let filter = Document::from(numbered());
    let (sort, hint) = (doc! { "total": -1_i32 }, Hint::Name("total_-1".to_owned()));
    let collation = Collation::builder().locale("en").build();
    let resolvers: [Resolver<'_, Document, Bson>; 1] = [&whole_number_as_text];

    let plain = Invoice::count(&never, numbered());
    assert_eq!(
        format!("{plain:?}"),
        format!(
            "Read {{ filter: {filter:?}, sort: None, limit: None, skip: None, hint: None, \
             collation: None, resolvers: 0, .. }}"
        )
    );
    let under_options = Invoice::find_with(&never, numbered(), &resolvers)
        .sort(sort.clone())
        .limit(2_i64)
        .skip(1_u64)
        .hint(hint.clone())
        .collation(collation.clone());
    assert_eq!(
        format!("{under_options:?}"),
        format!(
            "Read {{ filter: {filter:?}, sort: Some({sort:?}), limit: Some(2), skip: Some(1), \
             hint: Some({hint:?}), collation: Some({collation:?}), resolvers: 1, .. }}"
        )
    );
}

#[tokio::test]
async fn a_value_not_written_as_a_document_is_refused_before_any_connection() {
    let never = never_connected().await;
    let refused = InvoiceStatus::Paid.insert_one(&never).await.unwrap_err();
    assert!(
        matches!(
            refused,
            invoice_status_schema::OperationError::Unwritable(_)
        ),
        "got: {refused:?}"
    );
    let told = refused.to_string();
    assert!(
        told.starts_with("a value could not be written as BSON: ")
            && told.contains("a row is stored as a document, and this value is not written as one"),
        "got: {told}"
    );
}

#[tokio::test]
async fn an_unwritable_row_is_refused_before_any_connection() {
    let never = never_connected().await;
    let told = u64::MAX
        .serialize(bson::Serializer::new())
        .unwrap_err()
        .to_string();
    let answered = {
        let over = Ledger { total: u64::MAX };
        over.insert_one(&never)
    };
    let refused = answered.await.unwrap_err();
    assert!(
        matches!(refused, ledger_schema::OperationError::Unwritable(_)),
        "got: {refused:?}"
    );
    assert_eq!(
        refused.to_string(),
        format!("a value could not be written as BSON: {told}")
    );
    assert_eq!(refused.source().map(ToString::to_string), Some(told));

    let stored = Ledger { total: 7_u64 }.mongo_written_row().unwrap();
    assert_eq!(stored, doc! { "total": 7_i64 });
}

#[test]
fn a_value_bson_cannot_hold_is_unwritable_and_its_source() {
    let told = u64::MAX
        .serialize(bson::Serializer::new())
        .unwrap_err()
        .to_string();
    let failed = over_the_limit().unwrap_err();
    assert!(
        matches!(failed, OperationError::Unwritable(_)),
        "got: {failed:?}"
    );
    assert_eq!(
        failed.to_string(),
        format!("a value could not be written as BSON: {told}")
    );
    assert_eq!(failed.source().map(ToString::to_string), Some(told));
}

#[test]
fn the_error_and_every_operation_cross_threads() {
    crosses_threads::<OperationError>();
    compiles(&every_operation_is_sendable);
    compiles(&a_row_no_two_threads_share_is_sendable);
    compiles(&a_row_with_a_type_parameter_is_sendable);
}

#[test]
fn what_a_row_keeps_off_the_wire_is_not_stored() {
    let tally = Tally::default();
    tally.seen.set(7_u32);
    assert_eq!(tally.mongo_written_row().unwrap(), doc! { "total": 0_i64 });
    assert_eq!(tally.seen.get(), 7_u32);
}

/// The text keeps the BSON type of the `_id`: the text `3` and the number `3` are two rows.
#[test]
fn the_id_of_a_refused_row_is_told_as_bson_displays_it() {
    for (stored, told) in [
        (
            doc! { "_id": seeded_id() },
            format!("ObjectId(\"{SEEDED_ID}\")"),
        ),
        (doc! { "_id": "abc" }, "\"abc\"".to_owned()),
        (doc! { "_id": "3" }, "\"3\"".to_owned()),
        (doc! { "_id": 3_i32 }, "3".to_owned()),
        (doc! { "number": "INV-0042" }, "without an _id".to_owned()),
    ] {
        let refused = Invoice::mongo_read_row(stored, &[]).unwrap_err();
        assert!(
            matches!(
                &refused,
                OperationError::Unreadable { row, issues: _issues } if *row == told
            ),
            "for {told}, got: {refused:?}"
        );
    }
}
