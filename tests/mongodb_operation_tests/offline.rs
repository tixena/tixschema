//! What needs no MongoDB server: the error every operation fails with, and the read of a row once
//! the driver has handed it over.

use core::error::Error;

use bson::{Bson, Document, doc};
use mongodb::Collection;
use mongodb::options::ClientOptions;
use serde::Serialize as _;

use super::invoice_schema::{Expected, Field, Filter, Issue, MongoPath, OperationError};
use super::{
    Customer, Invoice, SEEDED_ID, customer_schema, invoice, open_invoices, readable, seeded_id,
    unreadable, whole_number_as_text,
};

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

/// Compiles only where the future of each read can be moved to another thread.
fn reads_are_sendable(held: &Collection<Document>) {
    sendable(&Invoice::find_one(held, numbered()));
    sendable(&Invoice::find_one_with(
        held,
        numbered(),
        &[&whole_number_as_text],
    ));
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

/// The read `find_one_with` makes of the row the driver hands it: as the type where the row
/// reads, as the type once a resolver has repaired it, and refused by its issue where none has.
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
        "got: {refused:?}"
    );
}

/// The issues are told as a refused `from_bson_piped` tells them, whose last words are the `bson`
/// library's own and differ between its major versions.
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
fn the_error_and_both_reads_cross_threads() {
    crosses_threads::<OperationError>();
    compiles(&reads_are_sendable);
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
