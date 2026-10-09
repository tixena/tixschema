//! What the README shows of the operations: every operation bound to what it answers, a read
//! under options, rows a resolver repairs, what each failure is told as, and a query run on the
//! driver itself.

use core::iter::once;
use std::collections::HashMap;

use bson::oid::ObjectId;
use bson::{Bson, Document, doc};
use mongodb::Collection;
use mongodb::options::{ClientOptions, Collation, Hint};
use mongodb::results::{DeleteResult, InsertOneResult, UpdateResult};
use serde::{Deserialize, Serialize};
use tixschema::model_schema;
use tokio::task::JoinHandle;

use super::super::live::{collection, say};
use super::{
    Address, Customer, Invoice, InvoiceStatus, assert_declared_and_documented, assert_documented,
    customer_schema, invoice, invoice_schema,
};
use crate::BSON_MAJOR;

use invoice_schema::{Expected, Filter, Issue, OperationError, Read, Resolution, Resolver};

/// The type the README reads the rows of an aggregation as, and the function that runs it on the
/// driver, character for character.
const DECLARED_DIRECT: [&str; 2] = [
    r#"/// What one customer was billed: no row of `Invoice`, so no read of `Invoice` answers it.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, Serialize)]
pub struct Billed {
    #[serde(rename = "_id")]
    pub customer: String,
    pub total: f64,
}"#,
    r#"/// What each customer was billed, by an aggregation run on the driver itself. Each row it
/// answers is read as the operations read one, and fails as they fail.
async fn billed(
    invoices: &Collection<Document>,
) -> Result<Vec<Billed>, billed_schema::OperationError> {
    use billed_schema::OperationError;

    let pipeline = [
        doc! { "$group": { "_id": "$customer.name", "total": { "$sum": "$total" } } },
        doc! { "$sort": { "_id": 1_i32 } },
    ];
    let mut cursor = invoices
        .aggregate(pipeline)
        .await
        .map_err(OperationError::Database)?;
    let mut rows = Vec::new();
    while cursor.advance().await.map_err(OperationError::Database)? {
        let row = cursor
            .deserialize_current()
            .map_err(OperationError::Database)?;
        let id = row
            .get("_id")
            .map_or_else(|| "without an _id".to_owned(), ToString::to_string);
        let read =
            Billed::from_bson_piped(row, &[]).map_err(|refused| OperationError::Unreadable {
                row: id,
                issues: refused.issues,
            })?;
        rows.push(read);
    }
    Ok(rows)
}"#,
];

/// The line the README brings the operations' types into scope with.
const DECLARED_IMPORT: &str =
    "use invoice_schema::{Expected, Filter, Issue, OperationError, Read, Resolution, Resolver};";

/// The functions the README calls the operations in, character for character.
const DECLARED_OPERATIONS: [&str; 4] = [
    "/// Every operation, each bound to what it answers.
async fn every_operation(
    invoices: &Collection<Document>,
    invoice: &Invoice,
) -> Result<(), OperationError> {
    let paths = Invoice::MONGO_FIELDS;
    let drafts = || paths.status.eq(InvoiceStatus::Draft);
    let voided = || paths.total.set(0.0_f64);
    let resolvers: [Resolver<'_, Document, Bson>; 1] = [&whole_number_as_text];

    let _stored: InsertOneResult = invoice.insert_one(invoices).await?;
    let _first: Option<Invoice> = Invoice::find_one(invoices, drafts()?).await?;
    let _repaired: Option<Invoice> =
        Invoice::find_one_with(invoices, drafts()?, &resolvers).await?;
    let _every: Vec<Invoice> = Invoice::find(invoices, drafts()?).await?;
    let _every_repaired: Vec<Invoice> = Invoice::find_with(invoices, drafts()?, &resolvers).await?;
    let _how_many: u64 = Invoice::count(invoices, drafts()?).await?;
    let _changed: UpdateResult = Invoice::update_one(invoices, drafts()?, voided()?).await?;
    let _all_changed: UpdateResult = Invoice::update_many(invoices, drafts()?, voided()?).await?;
    let _deleted: DeleteResult = Invoice::delete_one(invoices, drafts()?).await?;
    let _all_deleted: DeleteResult = Invoice::delete_many(invoices, drafts()?).await?;
    Ok(())
}",
    r#"/// The largest unpaid invoices of one customer, largest first, and how many it has unpaid.
async fn largest_unpaid(
    invoices: &Collection<Document>,
    customer: &str,
    at_most: i64,
) -> Result<(Vec<Invoice>, u64), OperationError> {
    let paths = Invoice::MONGO_FIELDS;
    let unpaid = || {
        paths
            .customer
            .name
            .eq(customer.to_owned())
            .map(|named| named.and(paths.paid_at.exists(false)))
    };
    let largest = Invoice::find_with(invoices, unpaid()?, &[&whole_number_as_text])
        .sort(doc! { "total": -1_i32 })
        .limit(at_most)
        .await?;
    let how_many = Invoice::count(invoices, unpaid()?).await?;
    Ok((largest, how_many))
}"#,
    r#"/// One page of invoices in the order of their numbers: a read under every option it takes,
/// which asks nothing of MongoDB until it is awaited.
fn page(invoices: &Collection<Document>, at: u64) -> Read<'_, Invoice, Vec<Invoice>> {
    Invoice::find(invoices, Filter::raw(doc! {}))
        .sort(doc! { "number": 1_i32 })
        .skip(at * 20)
        .limit(20)
        .hint(Hint::Name("number_1".to_owned()))
        .collation(Collation::builder().locale("en").build())
}"#,
    "/// A count handed to something that takes a future, which a read is not until it is turned
/// into one.
fn counted_elsewhere(
    invoices: &'static Collection<Document>,
) -> JoinHandle<Result<u64, OperationError>> {
    tokio::spawn(Invoice::count(invoices, Filter::raw(doc! {})).into_future())
}",
];

/// The resolver the README settles the `_id` of a row with, and the read it is handed to,
/// character for character.
const DECLARED_UNDECLARED_ID: [&str; 2] = [
    r#"/// The `_id` MongoDB stores every row under, which `Customer` does not declare: taken out.
fn id_dropped(
    raw: &mut Document,
    issue: &customer_schema::Issue<Bson>,
) -> customer_schema::Resolution {
    use customer_schema::{Issue, Resolution};

    let Issue::Unknown {
        path,
        found: _found,
    } = issue
    else {
        return Resolution::NotTouched;
    };
    if path.to_string() == "_id" && path.remove_from_document(raw) {
        Resolution::Settled
    } else {
        Resolution::NotTouched
    }
}"#,
    "/// A customer by name, from a collection whose rows are customers.
async fn customer_named(
    customers: &Collection<Document>,
    name: &str,
) -> Result<Option<Customer>, customer_schema::OperationError> {
    let named = Customer::MONGO_FIELDS.name.eq(name.to_owned())?;
    Customer::find_one_with(customers, named, &[&id_dropped]).await
}",
];

/// The resolver the README's operations are handed: the one its "Resolvers" section declares,
/// written here against this section's `Invoice`.
const DECLARED_RESOLVER: &str =
    "/// A whole number an older writer stored as text; a negative count is refused.
fn whole_number_as_text(raw: &mut Document, issue: &Issue<Bson>) -> Resolution {
    let Issue::Invalid {
        path,
        expected: Expected::U32,
        found: Bson::String(text),
        reason: _reason,
    } = issue
    else {
        return Resolution::NotTouched;
    };
    let settled = text
        .trim()
        .parse::<i64>()
        .is_ok_and(|number| number >= 0_i64 && path.set_in_document(raw, Bson::Int64(number)));
    if settled {
        Resolution::Settled
    } else {
        Resolution::Rejected
    }
}";

/// The type the README declares with a map whose keys are numbers, character for character.
const DECLARED_ROSTER: &str = r#"#[model_schema(decode_with)]
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Roster {
    pub by_number: HashMap<u32, String>,
}"#;

/// How the README says each failure opens: MongoDB's, an unreadable row's, an unwritable value's.
const SHOWN_OPENINGS: [&str; 3] = [
    "MongoDB refused the operation or could not be reached: ",
    "the row ",
    "a value could not be written as BSON: ",
];

/// The JSON the README shows a roster written as.
const SHOWN_ROSTER_AS_JSON: &str = r#"{"byNumber":{"7":"seven"}}"#;

/// What the README shows a stored roster told as, under version 2 of the `bson` library and
/// under version 3.
const SHOWN_ROSTER_UNREADABLE: [&str; 2] = [
    r#"the row without an _id does not read as expected: undescribed: invalid type: string "7", expected u32"#,
    r#"the row without an _id does not read as expected: undescribed: BSON error. Kind: A deserialization-related error occurred. Message: invalid type: string "7", expected u32."#,
];

/// What the README shows a roster that cannot be written told as, under each major version.
const SHOWN_ROSTER_UNWRITABLE: [&str; 2] = [
    "a value could not be written as BSON: Invalid map key type: 7",
    "a value could not be written as BSON: BSON error. Kind: A serialization error occurred. Message: invalid document key type: int64.",
];

/// The README's table of what a refused row is told by, for each `_id` a row may hold.
const SHOWN_ROW_IDS: [&str; 5] = [
    r#"| an `ObjectId` | `ObjectId("6a7cc592ca0574e6efdfe217")` |"#,
    r#"| the text `abc` | `"abc"` |"#,
    r#"| the text `3` | `"3"` |"#,
    "| the number `3` | `3` |",
    "| no `_id` | `without an _id` |",
];

/// What the README shows a stored customer told as: by the plain read, and read once the resolver
/// has taken its `_id` out.
const SHOWN_UNDECLARED_ID: [&str; 2] = [
    r#"the row ObjectId("6a7cc592ca0574e6efdfe217") does not read as expected: _id: unknown: found ObjectId("6a7cc592ca0574e6efdfe217")"#,
    r#"Customer { address: Address { city: "Moca", postal_code: "56000" }, name: "Acme", open_invoices: 3 }"#,
];

/// What the README shows a row an older writer left told as, under version 2 of the `bson`
/// library and under version 3.
const SHOWN_UNREADABLE: [&str; 2] = [
    r#"the row ObjectId("6a7cc592ca0574e6efdfe217") does not read as expected: customer.openInvoices: invalid: expected U32, found String("3"): invalid type: string "3", expected u32"#,
    r#"the row ObjectId("6a7cc592ca0574e6efdfe217") does not read as expected: customer.openInvoices: invalid: expected U32, found String("3"): BSON error. Kind: A deserialization-related error occurred. Message: invalid type: string "3", expected u32."#,
];

/// What one customer was billed: no row of `Invoice`, so no read of `Invoice` answers it.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, Serialize)]
pub struct Billed {
    #[serde(rename = "_id")]
    pub customer: String,
    pub total: f64,
}

#[model_schema(decode_with)]
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Roster {
    pub by_number: HashMap<u32, String>,
}

/// What each customer was billed, by an aggregation run on the driver itself. Each row it
/// answers is read as the operations read one, and fails as they fail.
async fn billed(
    invoices: &Collection<Document>,
) -> Result<Vec<Billed>, billed_schema::OperationError> {
    use billed_schema::OperationError;

    let pipeline = [
        doc! { "$group": { "_id": "$customer.name", "total": { "$sum": "$total" } } },
        doc! { "$sort": { "_id": 1_i32 } },
    ];
    let mut cursor = invoices
        .aggregate(pipeline)
        .await
        .map_err(OperationError::Database)?;
    let mut rows = Vec::new();
    while cursor.advance().await.map_err(OperationError::Database)? {
        let row = cursor
            .deserialize_current()
            .map_err(OperationError::Database)?;
        let id = row
            .get("_id")
            .map_or_else(|| "without an _id".to_owned(), ToString::to_string);
        let read =
            Billed::from_bson_piped(row, &[]).map_err(|refused| OperationError::Unreadable {
                row: id,
                issues: refused.issues,
            })?;
        rows.push(read);
    }
    Ok(rows)
}

/// A count handed to something that takes a future, which a read is not until it is turned
/// into one.
fn counted_elsewhere(
    invoices: &'static Collection<Document>,
) -> JoinHandle<Result<u64, OperationError>> {
    tokio::spawn(Invoice::count(invoices, Filter::raw(doc! {})).into_future())
}

/// A customer by name, from a collection whose rows are customers.
async fn customer_named(
    customers: &Collection<Document>,
    name: &str,
) -> Result<Option<Customer>, customer_schema::OperationError> {
    let named = Customer::MONGO_FIELDS.name.eq(name.to_owned())?;
    Customer::find_one_with(customers, named, &[&id_dropped]).await
}

/// Every operation, each bound to what it answers.
async fn every_operation(
    invoices: &Collection<Document>,
    invoice: &Invoice,
) -> Result<(), OperationError> {
    let paths = Invoice::MONGO_FIELDS;
    let drafts = || paths.status.eq(InvoiceStatus::Draft);
    let voided = || paths.total.set(0.0_f64);
    let resolvers: [Resolver<'_, Document, Bson>; 1] = [&whole_number_as_text];

    let _stored: InsertOneResult = invoice.insert_one(invoices).await?;
    let _first: Option<Invoice> = Invoice::find_one(invoices, drafts()?).await?;
    let _repaired: Option<Invoice> =
        Invoice::find_one_with(invoices, drafts()?, &resolvers).await?;
    let _every: Vec<Invoice> = Invoice::find(invoices, drafts()?).await?;
    let _every_repaired: Vec<Invoice> = Invoice::find_with(invoices, drafts()?, &resolvers).await?;
    let _how_many: u64 = Invoice::count(invoices, drafts()?).await?;
    let _changed: UpdateResult = Invoice::update_one(invoices, drafts()?, voided()?).await?;
    let _all_changed: UpdateResult = Invoice::update_many(invoices, drafts()?, voided()?).await?;
    let _deleted: DeleteResult = Invoice::delete_one(invoices, drafts()?).await?;
    let _all_deleted: DeleteResult = Invoice::delete_many(invoices, drafts()?).await?;
    Ok(())
}

/// The `_id` MongoDB stores every row under, which `Customer` does not declare: taken out.
fn id_dropped(
    raw: &mut Document,
    issue: &customer_schema::Issue<Bson>,
) -> customer_schema::Resolution {
    use customer_schema::{Issue, Resolution};

    let Issue::Unknown {
        path,
        found: _found,
    } = issue
    else {
        return Resolution::NotTouched;
    };
    if path.to_string() == "_id" && path.remove_from_document(raw) {
        Resolution::Settled
    } else {
        Resolution::NotTouched
    }
}

/// The largest unpaid invoices of one customer, largest first, and how many it has unpaid.
async fn largest_unpaid(
    invoices: &Collection<Document>,
    customer: &str,
    at_most: i64,
) -> Result<(Vec<Invoice>, u64), OperationError> {
    let paths = Invoice::MONGO_FIELDS;
    let unpaid = || {
        paths
            .customer
            .name
            .eq(customer.to_owned())
            .map(|named| named.and(paths.paid_at.exists(false)))
    };
    let largest = Invoice::find_with(invoices, unpaid()?, &[&whole_number_as_text])
        .sort(doc! { "total": -1_i32 })
        .limit(at_most)
        .await?;
    let how_many = Invoice::count(invoices, unpaid()?).await?;
    Ok((largest, how_many))
}

/// One page of invoices in the order of their numbers: a read under every option it takes,
/// which asks nothing of MongoDB until it is awaited.
fn page(invoices: &Collection<Document>, at: u64) -> Read<'_, Invoice, Vec<Invoice>> {
    Invoice::find(invoices, Filter::raw(doc! {}))
        .sort(doc! { "number": 1_i32 })
        .skip(at * 20)
        .limit(20)
        .hint(Hint::Name("number_1".to_owned()))
        .collation(Collation::builder().locale("en").build())
}

/// The `_id` of the stored rows the README reads.
fn row_id() -> ObjectId {
    ObjectId::parse_str("6a7cc592ca0574e6efdfe217").unwrap()
}

/// The row MongoDB stores for `value`.
fn stored<T>(value: &T) -> Document
where
    T: Serialize,
{
    let written = value.serialize(bson::Serializer::new()).unwrap();
    written.as_document().unwrap().clone()
}

/// Of what the README shows under version 2 of the `bson` library and then under version 3,
/// the one this binary is built against.
fn under_this_major(shown: [&'static str; 2]) -> &'static str {
    let [under_2, under_3] = shown;
    if BSON_MAJOR == 2 { under_2 } else { under_3 }
}

/// A whole number an older writer stored as text; a negative count is refused.
fn whole_number_as_text(raw: &mut Document, issue: &Issue<Bson>) -> Resolution {
    let Issue::Invalid {
        path,
        expected: Expected::U32,
        found: Bson::String(text),
        reason: _reason,
    } = issue
    else {
        return Resolution::NotTouched;
    };
    let settled = text
        .trim()
        .parse::<i64>()
        .is_ok_and(|number| number >= 0_i64 && path.set_in_document(raw, Bson::Int64(number)));
    if settled {
        Resolution::Settled
    } else {
        Resolution::Rejected
    }
}

#[test]
fn the_readme_declares_the_operations_that_compile_here() {
    let source = include_str!("operations.rs");
    for pinned in DECLARED_DIRECT
        .into_iter()
        .chain(DECLARED_OPERATIONS)
        .chain(DECLARED_UNDECLARED_ID)
        .chain([DECLARED_IMPORT, DECLARED_RESOLVER, DECLARED_ROSTER])
    {
        assert_declared_and_documented(source, pinned);
    }
    // Compiled, and never called: neither asks MongoDB anything until it is awaited.
    let _: fn(&Collection<Document>, u64) -> Read<'_, Invoice, Vec<Invoice>> = page;
    let _: fn(&'static Collection<Document>) -> JoinHandle<Result<u64, OperationError>> =
        counted_elsewhere;
}

/// The stored customer the README reads: a row of `Customer` under the `_id` MongoDB gave it.
fn stored_customer() -> Document {
    doc! {
        "_id": row_id(),
        "address": { "city": "Moca", "postalCode": "56000" },
        "name": "Acme",
        "openInvoices": 3_i32,
    }
}

/// What a refused read of one stored row holds as the row's `_id`.
fn told_id(row: Document) -> String {
    match Invoice::mongo_read_row(row, &[]) {
        Err(OperationError::Unreadable {
            row: id,
            issues: _issues,
        }) => id,
        Err(OperationError::Database(_) | OperationError::Unwritable(_)) | Ok(_) => String::new(),
    }
}

#[test]
fn the_readme_shows_what_a_row_with_an_undeclared_id_is_told_as() {
    let refused = Customer::mongo_read_row(stored_customer(), &[]).unwrap_err();
    let repaired = Customer::mongo_read_row(stored_customer(), &[&id_dropped]).unwrap();
    let told = [refused.to_string(), format!("{repaired:?}")];
    assert_eq!(told, SHOWN_UNDECLARED_ID);
    for shown in SHOWN_UNDECLARED_ID {
        assert_documented(shown);
    }
}

#[test]
fn the_readme_shows_what_an_unreadable_row_is_told_as() {
    let mut older = stored(&invoice("INV-0042", "Acme", 1250.5));
    older.insert("_id", row_id());
    older
        .get_document_mut("customer")
        .unwrap()
        .insert("openInvoices", "3");
    let refused = Invoice::mongo_read_row(older.clone(), &[]).unwrap_err();
    assert_eq!(refused.to_string(), under_this_major(SHOWN_UNREADABLE));
    for shown in SHOWN_UNREADABLE {
        assert_documented(shown);
    }
    let read = Invoice::mongo_read_row(older, &[&whole_number_as_text]).unwrap();
    assert_eq!(read.customer.open_invoices, 3);
}

#[test]
fn the_readme_shows_the_id_a_refused_row_is_told_by() {
    let told = [
        ("an `ObjectId`", doc! { "_id": row_id() }),
        ("the text `abc`", doc! { "_id": "abc" }),
        ("the text `3`", doc! { "_id": "3" }),
        ("the number `3`", doc! { "_id": 3_i32 }),
        ("no `_id`", doc! { "number": "INV-0042" }),
    ]
    .map(|(held, row)| format!("| {held} | `{}` |", told_id(row)));
    assert_eq!(told, SHOWN_ROW_IDS);
    assert_documented(&SHOWN_ROW_IDS.join("\n"));
}

/// A map whose keys are numbers is written and read as JSON, and is neither as BSON.
#[test]
fn the_readme_shows_what_a_map_keyed_by_a_number_is_told_as() {
    let roster = Roster {
        by_number: HashMap::from([(7, "seven".to_owned())]),
    };
    let as_json = serde_json::to_value(&roster).unwrap();
    assert_eq!(as_json.to_string(), SHOWN_ROSTER_AS_JSON);
    let from_json = Roster::from_value_piped(as_json, &[]).unwrap();
    assert_eq!(from_json.by_number, roster.by_number);

    let unwritable = roster.mongo_written_row().unwrap_err();
    assert!(
        matches!(unwritable, roster_schema::OperationError::Unwritable(_)),
        "got: {unwritable:?}"
    );
    assert_eq!(
        unwritable.to_string(),
        under_this_major(SHOWN_ROSTER_UNWRITABLE)
    );
    let stored_roster = doc! { "byNumber": { "7": "seven" } };
    let refused = Roster::mongo_read_row(stored_roster, &[]).unwrap_err();
    assert_eq!(
        refused.to_string(),
        under_this_major(SHOWN_ROSTER_UNREADABLE)
    );
    for shown in once(SHOWN_ROSTER_AS_JSON)
        .chain(SHOWN_ROSTER_UNWRITABLE)
        .chain(SHOWN_ROSTER_UNREADABLE)
    {
        assert_documented(shown);
    }
}

#[tokio::test]
async fn the_readme_shows_how_each_failure_opens() {
    let unreachable = ClientOptions::parse("not an address").await.unwrap_err();
    let failures = [
        OperationError::Database(unreachable),
        Invoice::mongo_read_row(doc! { "_id": row_id() }, &[]).unwrap_err(),
        OperationError::from(u64::MAX.serialize(bson::Serializer::new()).unwrap_err()),
    ];
    for (failure, opens) in failures.iter().zip(SHOWN_OPENINGS) {
        let told = failure.to_string();
        assert!(told.starts_with(opens), "got: {told}");
        assert_documented(&format!("`{opens}`"));
    }
}

#[tokio::test]
async fn live_the_readme_examples_run_against_a_collection() {
    let Some(invoices) = collection("readme_invoices").await else {
        return;
    };
    let draft = Invoice {
        status: InvoiceStatus::Draft,
        ..invoice("INV-0000", "Acme", 50.5)
    };
    every_operation(&invoices, &draft).await.unwrap();
    let left = Invoice::count(&invoices, Filter::raw(doc! {}))
        .await
        .unwrap();
    assert_eq!(left, 0, "the one draft stored was changed and then deleted");

    // Four invoices: two unpaid ones of Acme, the larger of them with its customer's count
    // stored as text, a paid one of Acme, and one of another customer.
    let mut older = stored(&invoice("INV-0002", "Acme", 300.5));
    older
        .get_document_mut("customer")
        .unwrap()
        .insert("openInvoices", "1");
    let paid = Invoice {
        paid_at: Some("2026-10-07T17:40:00Z".to_owned()),
        status: InvoiceStatus::Paid,
        ..invoice("INV-0003", "Acme", 500.5)
    };
    let rows = vec![
        stored(&invoice("INV-0001", "Acme", 100.5)),
        older,
        stored(&paid),
        stored(&invoice("INV-0004", "Globex", 900.5)),
    ];
    invoices.insert_many(rows).await.unwrap();

    let (largest, how_many) = largest_unpaid(&invoices, "Acme", 1).await.unwrap();
    say(&format!(
        "the largest unpaid invoice of one customer: {largest:?}, of {how_many}"
    ));
    let numbers: Vec<&str> = largest.iter().map(|row| row.number.as_str()).collect();
    assert_eq!(numbers, ["INV-0002"], "the larger of the two unpaid");
    assert_eq!(how_many, 2, "the paid one is not counted");
    let unrepaired = Invoice::find(&invoices, Filter::raw(doc! {}))
        .await
        .unwrap_err();
    assert!(
        matches!(
            &unrepaired,
            OperationError::Unreadable {
                row: _row,
                issues: _issues
            }
        ),
        "a row no resolver repaired did not refuse the read: {unrepaired}"
    );

    let totals = billed(&invoices).await.unwrap();
    say(&format!("the aggregation answers {totals:?}"));
    assert_eq!(
        format!("{totals:?}"),
        r#"[Billed { customer: "Acme", total: 901.5 }, Billed { customer: "Globex", total: 900.5 }]"#,
        "one row per customer, in the order of their names"
    );
    invoices.drop().await.unwrap();

    let Some(customers) = collection("readme_customers").await else {
        return;
    };
    let acme = Customer {
        address: Address {
            city: "Moca".to_owned(),
            postal_code: "56000".to_owned(),
        },
        name: "Acme".to_owned(),
        open_invoices: 3,
    };
    acme.insert_one(&customers).await.unwrap();
    let named = || Customer::MONGO_FIELDS.name.eq("Acme".to_owned()).unwrap();
    let refused = Customer::find_one(&customers, named()).await.unwrap_err();
    say(&format!(
        "a row whose `_id` its type does not declare: {refused}"
    ));
    assert!(
        matches!(
            &refused,
            customer_schema::OperationError::Unreadable {
                row: _row,
                issues: _issues
            }
        ),
        "the `_id` MongoDB gave the row did not refuse the plain read: {refused}"
    );
    let read = customer_named(&customers, "Acme").await.unwrap();
    assert_eq!(
        read.map(|customer| customer.open_invoices),
        Some(3),
        "the resolver took the `_id` out and the row read"
    );
    customers.drop().await.unwrap();
}
