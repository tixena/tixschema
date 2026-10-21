//! What the README shows of the typed paths: the kind each function of `Invoice::MONGO_FIELDS`
//! answers, the document each operator writes, the filters and updates its examples build, the
//! paths `nest` reaches below a nested model, and the arms of an enum under a tag.
//!
//! A table row is read off the code: a call is turned into the row's first cell by `stringify!`,
//! and the document it writes into the second.

use core::error::Error;

use bson::oid::ObjectId;
use bson::{Document, doc};
use mongodb::results::UpdateResult;
use mongodb::{Collection, Database};
use serde::{Deserialize, Serialize};
use tixschema::model_schema;

use super::{
    Address, Customer, Invoice, InvoiceStatus, LineItem, Payment, assert_declared_and_documented,
    assert_documented, readme,
};
use super::{invoice_schema, payment_schema};

use invoice_schema::{Filter, Update, WriteError};

/// One row of the README's table of arm functions: what an enum is declared under, one of its
/// variants, and the function of that variant's arm, held by the compiler to being there. With
/// it, the name of the enum.
macro_rules! arm {
    ($of:ident . $function:ident () : $attribute:literal, $variant:literal) => {{
        held(&$of::MONGO_FIELDS.$function());
        (
            stringify!($of),
            [$attribute, $variant, stringify!($function())],
        )
    }};
}

/// One row of a README table whose call changes rows in MongoDB: the call as it is written over
/// `$collection`, held to compiling in a function nothing calls.
macro_rules! called {
    ($collection:ident: $($call:tt)+) => {{
        held(&|$collection: Collection<Document>| async move {
            Ok::<UpdateResult, Box<dyn Error + Send + Sync>>($($call)+)
        });
        stringify!($($call)+)
    }};
}

/// One row of the README's table of kinds: a function of `Invoice`'s paths, held by the compiler
/// to answering the kind the row names.
macro_rules! kind {
    ($paths:ident . $member:ident () : $($kind:tt)+) => {{
        held::<$($kind)+>(&$paths.$member());
        [stringify!($member()), stringify!($($kind)+)]
    }};
}

/// One row of a README table of documents: a call as it is written, and the document it writes.
macro_rules! written {
    ($($call:tt)+) => {
        (stringify!($($call)+), Document::from($($call)+).to_string())
    };
}

/// The models the README declares for the arms of an enum under a tag, the `const` that holds one
/// arm filter and the functions that nest it and its update, character for character.
const DECLARED_ARMS: [&str; 6] = [
    r#"#[model_schema(enable = ["decode_with", "mongodb"])]
#[derive(Debug, Deserialize, Serialize)]
pub struct Courier {
    pub name: String,
    pub region: String,
}"#,
    r#"#[model_schema(enable = ["decode_with", "mongodb"])]
#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "type")]
pub enum Transport {
    #[serde(rename = "by-courier")]
    ByCourier(Courier),
    Collected,
}"#,
    r#"#[model_schema(enable = ["decode_with", "mongodb"])]
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Booking {
    pub depot_code: String,
    #[serde(flatten)]
    pub transport: Transport,
}"#,
    "/// The bookings a courier carries, held in a `const`.
const BY_COURIER: transport_schema::ArmFilter<Transport, Courier> =
    Transport::MONGO_FIELDS.type__by_courier();",
    "/// The bookings one courier carries out of one depot.
fn couriered(depot: &str, courier: &str) -> Result<booking_schema::Filter<Booking>, WriteError> {
    let booking = Booking::MONGO_FIELDS;
    let by_name = BY_COURIER.and(Courier::MONGO_FIELDS.name().eq(courier.to_owned())?);
    Ok(booking
        .transport()
        .nest(by_name)
        .and(booking.depot_code().eq(depot.to_owned())?))
}",
    "/// A booking's courier renamed, with the arm's tag written where an upsert inserts the row.
fn recouriered(courier: &str) -> Result<booking_schema::Update<Booking>, WriteError> {
    let renamed = BY_COURIER
        .update(Courier::MONGO_FIELDS.name().set(courier.to_owned())?)
        .tag_on_insert();
    Ok(Booking::MONGO_FIELDS.transport().nest(renamed))
}",
];

/// The functions the README builds its filters and updates with, character for character.
const DECLARED_BUILDERS: [&str; 6] = [
    r#"/// The address the examples write.
fn moca() -> Address {
    Address {
        city: "Moca".to_owned(),
        postal_code: "56000".to_owned(),
    }
}"#,
    r#"/// The line item the examples write.
fn bolts() -> LineItem {
    LineItem {
        price: 4.5,
        quantity: 9,
        sku: "C-3".to_owned(),
    }
}"#,
    "/// Unpaid invoices of one city that are past due or above an amount.
fn to_chase(city: &str, above: f64) -> Result<Filter<Invoice>, WriteError> {
    let (paths, customer) = (Invoice::MONGO_FIELDS, Customer::MONGO_FIELDS);
    let in_city = Address::MONGO_FIELDS.city().eq(city.to_owned())?;
    Ok(paths
        .status()
        .eq(InvoiceStatus::PastDue)?
        .or(paths.total().gt(above)?)
        .and(paths.customer().nest(customer.address().nest(in_city)))
        .and(paths.paid_at().exists(false)))
}",
    r#"/// What paying an invoice changes.
fn paid(at: &str) -> Result<Update<Invoice>, WriteError> {
    let (paths, customer) = (Invoice::MONGO_FIELDS, Customer::MONGO_FIELDS);
    Ok(paths
        .status()
        .set(InvoiceStatus::Paid)?
        .and(paths.paid_at().set(at.to_owned())?)
        .and(paths.customer().nest(customer.open_invoices().set(0)?))
        .and(paths.tags().push("settled".to_owned())?))
}"#,
    r#"/// Invoices of one cost center that are no draft. `details` is a map: its keys are data.
fn of_cost_center(code: &str) -> Result<Filter<Invoice>, WriteError> {
    let paths = Invoice::MONGO_FIELDS;
    Ok(paths
        .status()
        .ne(InvoiceStatus::Draft)?
        .and(Filter::raw(doc! { "details.costCenter": code })))
}"#,
    r#"/// A surcharge: `$inc`, an operator the typed paths do not write, beside one they do.
fn surcharged(by: f64) -> Result<Update<Invoice>, WriteError> {
    let paths = Invoice::MONGO_FIELDS;
    Ok(paths
        .tags()
        .push("surcharged".to_owned())?
        .and(Update::raw(doc! { "$inc": { "total": by } })))
}"#,
];

/// The `const` item the README holds one path in, character for character.
const DECLARED_CONST: &str =
    "const TOTAL: invoice_schema::Field<Invoice, f64> = Invoice::MONGO_FIELDS.total();";

/// The call the README counts the rows of an arm with, which `counted` makes with no regard for
/// spacing.
const DECLARED_COUNT: &str = r#"Booking::count(&collection, couriered("SDQ-1", "Acme Freight").map_err(booking_schema::OperationError::Unwritable)?).await?"#;

/// The line the README brings the query types into scope with.
const DECLARED_IMPORT: &str = "use invoice_schema::{Filter, Update, WriteError};";

/// The two types the README declares one below the other, and the function that nests a filter
/// under the one tixschema has not seen, character for character.
const DECLARED_UNSEEN: [&str; 3] = [
    r#"#[model_schema(enable = ["decode_with", "mongodb"])]
#[derive(Debug, Deserialize, Serialize)]
pub struct Shipment {
    pub carrier: Carrier,
    pub code: String,
}"#,
    r#"#[model_schema(enable = ["decode_with", "mongodb"])]
#[derive(Debug, Deserialize, Serialize)]
pub struct Carrier {
    pub name: String,
}"#,
    r#"/// Shipments of one carrier. `Carrier` is declared below `Shipment`, so `carrier` is one whole
/// value there, and `nest` reaches the paths below it as it does below any model.
fn carried_by(name: &str) -> Result<shipment_schema::Filter<Shipment>, WriteError> {
    let shipment = Shipment::MONGO_FIELDS;
    let named = Carrier::MONGO_FIELDS.name().eq(name.to_owned())?;
    Ok(shipment
        .code()
        .regex("^S-", "")
        .and(shipment.carrier().nest(named)))
}"#,
];

/// The rows of the README's table of the model an arm is over, each one whole.
const SHOWN_ARM_MODELS: [&str; 2] = [
    "| One slot serde writes | The slot's type, read through `Option`, a list and the wrappers `nest` reads through: what `nest` reaches below a path over that slot. `ByCourier(Courier)` is over `Courier`, and an arm that holds a `Box<Courier>` is over `Courier` too. A map is `()` |",
    "| Fields written in the variant, several slots, nothing, or one slot serde never writes | `()` |",
];

/// The bookings a courier carries, held in a `const`.
const BY_COURIER: transport_schema::ArmFilter<Transport, Courier> =
    Transport::MONGO_FIELDS.type__by_courier();

const TOTAL: invoice_schema::Field<Invoice, f64> = Invoice::MONGO_FIELDS.total();

#[model_schema(enable = ["decode_with", "mongodb"])]
#[derive(Debug, Deserialize, Serialize)]
pub struct Courier {
    pub name: String,
    pub region: String,
}

#[model_schema(enable = ["decode_with", "mongodb"])]
#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "type")]
pub enum Transport {
    #[serde(rename = "by-courier")]
    ByCourier(Courier),
    Collected,
}

#[model_schema(enable = ["decode_with", "mongodb"])]
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Booking {
    pub depot_code: String,
    #[serde(flatten)]
    pub transport: Transport,
}

/// What an arm of [`Paper`] holds.
#[model_schema(enable = ["decode_with", "mongodb"])]
#[derive(Debug, Deserialize, Serialize)]
pub struct Notice {
    pub code: String,
}

/// What the README's tables of arms say beside its own example, held by the compiler: an enum
/// under a tag key of two words, and an arm whose model is behind a wrapper.
#[model_schema(enable = ["decode_with", "mongodb"])]
#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "docType", content = "body", rename_all = "kebab-case")]
pub enum Paper {
    Forwarded(Box<Courier>),
    PastDue(Notice),
}

#[model_schema(enable = ["decode_with", "mongodb"])]
#[derive(Debug, Deserialize, Serialize)]
pub struct Shipment {
    pub carrier: Carrier,
    pub code: String,
}

#[model_schema(enable = ["decode_with", "mongodb"])]
#[derive(Debug, Deserialize, Serialize)]
pub struct Carrier {
    pub name: String,
}

/// The line item the examples write.
fn bolts() -> LineItem {
    LineItem {
        price: 4.5,
        quantity: 9,
        sku: "C-3".to_owned(),
    }
}

/// What the README's examples build, each as a row of the table that follows them.
fn built() -> Result<Vec<(&'static str, String)>, WriteError> {
    Ok(vec![
        written!(to_chase("Moca", 1000.5)?),
        written!(paid("2026-10-07T17:40:00Z")?),
        written!(of_cost_center("CC-7")?),
        written!(surcharged(12.5)?),
        written!(carried_by("Acme Freight")?),
        written!(couriered("SDQ-1", "Acme Freight")?),
        written!(recouriered("Acme Freight")?),
    ])
}

/// Whether the README holds the table row of `call` and then `answered`, a cell of prose: the
/// call with no regard for spacing, and the prose character for character.
fn call_is_documented(call: &str, answered: &str) -> bool {
    let closing = format!("` | {answered} |");
    readme().lines().any(|line| {
        line.strip_prefix("| `")
            .and_then(|rest| rest.strip_suffix(closing.as_str()))
            .is_some_and(|shown| squeezed(shown) == squeezed(call))
    })
}

/// Shipments of one carrier. `Carrier` is declared below `Shipment`, so `carrier` is one whole
/// value there, and `nest` reaches the paths below it as it does below any model.
fn carried_by(name: &str) -> Result<shipment_schema::Filter<Shipment>, WriteError> {
    let shipment = Shipment::MONGO_FIELDS;
    let named = Carrier::MONGO_FIELDS.name().eq(name.to_owned())?;
    Ok(shipment
        .code()
        .regex("^S-", "")
        .and(shipment.carrier().nest(named)))
}

/// The cells of one row of a README table whose every cell is code.
fn cells(line: &str) -> Option<Vec<&str>> {
    let inner = line.strip_prefix("| `")?.strip_suffix("` |")?;
    Some(inner.split("` | `").collect())
}

/// The call the README counts the rows of an arm with, held to compiling. It is never run.
async fn counted(database: &Database) -> Result<u64, booking_schema::OperationError> {
    let collection = database.collection("bookings");
    let how_many = Booking::count(
        &collection,
        couriered("SDQ-1", "Acme Freight").map_err(booking_schema::OperationError::Unwritable)?,
    )
    .await?;
    Ok(how_many)
}

/// The bookings one courier carries out of one depot.
fn couriered(depot: &str, courier: &str) -> Result<booking_schema::Filter<Booking>, WriteError> {
    let booking = Booking::MONGO_FIELDS;
    let by_name = BY_COURIER.and(Courier::MONGO_FIELDS.name().eq(courier.to_owned())?);
    Ok(booking
        .transport()
        .nest(by_name)
        .and(booking.depot_code().eq(depot.to_owned())?))
}

/// Compiles only where `path` is of the kind `T`.
const fn held<T>(_path: &T) {}

/// The address the examples write.
fn moca() -> Address {
    Address {
        city: "Moca".to_owned(),
        postal_code: "56000".to_owned(),
    }
}

/// Invoices of one cost center that are no draft. `details` is a map: its keys are data.
fn of_cost_center(code: &str) -> Result<Filter<Invoice>, WriteError> {
    let paths = Invoice::MONGO_FIELDS;
    Ok(paths
        .status()
        .ne(InvoiceStatus::Draft)?
        .and(Filter::raw(doc! { "details.costCenter": code })))
}

/// What paying an invoice changes.
fn paid(at: &str) -> Result<Update<Invoice>, WriteError> {
    let (paths, customer) = (Invoice::MONGO_FIELDS, Customer::MONGO_FIELDS);
    Ok(paths
        .status()
        .set(InvoiceStatus::Paid)?
        .and(paths.paid_at().set(at.to_owned())?)
        .and(paths.customer().nest(customer.open_invoices().set(0)?))
        .and(paths.tags().push("settled".to_owned())?))
}

/// A booking's courier renamed, with the arm's tag written where an upsert inserts the row.
fn recouriered(courier: &str) -> Result<booking_schema::Update<Booking>, WriteError> {
    let renamed = BY_COURIER
        .update(Courier::MONGO_FIELDS.name().set(courier.to_owned())?)
        .tag_on_insert();
    Ok(Booking::MONGO_FIELDS.transport().nest(renamed))
}

/// Whether the README holds the table row `code` and then `document`: the code cells with no
/// regard for spacing, which `stringify!` writes its own way, and the document character for
/// character.
fn row_is_documented(code: &[&str], document: Option<&str>) -> bool {
    let wanted: Vec<String> = code.iter().copied().map(squeezed).collect();
    readme().lines().filter_map(cells).any(|shown| {
        let (shown_code, shown_document) = match document {
            Some(_) => match shown.split_last() {
                Some((last, before)) => (before, Some(*last)),
                None => return false,
            },
            None => (shown.as_slice(), None),
        };
        shown_document == document
            && shown_code.iter().copied().map(squeezed).collect::<Vec<_>>() == wanted
    })
}

/// `code` with its spacing taken out.
fn squeezed(code: &str) -> String {
    code.split_whitespace().collect()
}

/// A surcharge: `$inc`, an operator the typed paths do not write, beside one they do.
fn surcharged(by: f64) -> Result<Update<Invoice>, WriteError> {
    let paths = Invoice::MONGO_FIELDS;
    Ok(paths
        .tags()
        .push("surcharged".to_owned())?
        .and(Update::raw(doc! { "$inc": { "total": by } })))
}

/// Unpaid invoices of one city that are past due or above an amount.
fn to_chase(city: &str, above: f64) -> Result<Filter<Invoice>, WriteError> {
    let (paths, customer) = (Invoice::MONGO_FIELDS, Customer::MONGO_FIELDS);
    let in_city = Address::MONGO_FIELDS.city().eq(city.to_owned())?;
    Ok(paths
        .status()
        .eq(InvoiceStatus::PastDue)?
        .or(paths.total().gt(above)?)
        .and(paths.customer().nest(customer.address().nest(in_city)))
        .and(paths.paid_at().exists(false)))
}

/// The rows of the README's tables for the arms of an enum under a tag: what an arm filter
/// writes, and an arm that holds no model.
fn written_by_an_arm() -> Result<Vec<(&'static str, String)>, WriteError> {
    let (paths, payment) = (Invoice::MONGO_FIELDS, Payment::MONGO_FIELDS);
    Ok(vec![
        written!(BY_COURIER),
        written!(BY_COURIER.and(Courier::MONGO_FIELDS.name().eq("Acme".to_owned())?)),
        written!(transport_schema::Filter::from(BY_COURIER).negated()),
        written!(payment.kind__cash()),
        written!(paths.payment().nest(payment.kind__cash())),
        written!(
            payment
                .card()
                .last4()
                .eq("4242".to_owned())?
                .and(payment.kind__card())
        ),
    ])
}

/// The rows of the README's tables for an arm update: what `update`, `and` and `tag_on_insert`
/// write, under a content key too, and the tag of an arm that holds no model.
fn written_by_an_arm_update() -> Result<Vec<(&'static str, String)>, WriteError> {
    let (courier, notice) = (Courier::MONGO_FIELDS, Notice::MONGO_FIELDS);
    let (paper, payment) = (Paper::MONGO_FIELDS, Payment::MONGO_FIELDS);
    Ok(vec![
        written!(BY_COURIER.update(courier.name().set("Renamed".to_owned())?)),
        written!(
            BY_COURIER
                .update(courier.name().set("Renamed".to_owned())?)
                .and(courier.region().set_on_insert("north".to_owned())?)
                .tag_on_insert()
        ),
        written!(
            paper
                .doc_type__past_due()
                .update(notice.code().set("N-8".to_owned())?)
                .tag_on_insert()
        ),
        written!(transport_schema::Update::from(BY_COURIER.tag_on_insert())),
        written!(payment.kind__cash().tag_on_insert()),
        written!(
            payment
                .card()
                .last4()
                .set("4242".to_owned())?
                .and(payment.kind__card().tag_on_insert())
        ),
    ])
}

/// The rows of the README's table for a path of one value every row holds, and for one a row
/// may leave out.
fn written_by_a_value() -> Result<Vec<(&'static str, String)>, WriteError> {
    let paths = Invoice::MONGO_FIELDS;
    Ok(vec![
        written!(paths.number().eq("INV-0042".to_owned())?),
        written!(paths.number().ne("INV-0042".to_owned())?),
        written!(paths.total().gt(1000.5)?),
        written!(paths.total().gte(1000.5)?),
        written!(paths.total().lt(1000.5)?),
        written!(paths.total().lte(1000.5)?),
        written!(
            paths
                .number()
                .is_in(["INV-0042".to_owned(), "INV-0043".to_owned()])?
        ),
        written!(paths.number().not_in(["INV-0042".to_owned()])?),
        written!(paths.number().regex("^INV-", "i")),
        written!(paths.total().set(99.5)?),
        written!(paths.number().set_on_insert("INV-0099".to_owned())?),
        written!(paths.paid_at().exists(false)),
        written!(paths.paid_at().unset()),
        written!(paths.paid_at().lt("2026-10-01".to_owned())?),
        written!(paths.status().eq(InvoiceStatus::PastDue)?),
        written!(
            paths
                .status()
                .is_in([InvoiceStatus::Draft, InvoiceStatus::PastDue])?
        ),
        written!(paths.status().set(InvoiceStatus::Paid)?),
    ])
}

/// The rows of the README's table for a list of plain values.
fn written_by_a_list() -> Result<Vec<(&'static str, String)>, WriteError> {
    let paths = Invoice::MONGO_FIELDS;
    Ok(vec![
        written!(paths.tags().contains("export".to_owned())?),
        written!(
            paths
                .tags()
                .contains_any(["export".to_owned(), "priority".to_owned()])?
        ),
        written!(paths.tags().contains_none(["void".to_owned()])?),
        written!(paths.tags().size(0)),
        written!(
            paths.tags().elem_match(
                paths
                    .tags()
                    .element()
                    .gte("a".to_owned())?
                    .lt("n".to_owned())?
            )
        ),
        written!(paths.tags().push("reviewed".to_owned())?),
        written!(paths.tags().pull("draft".to_owned())?),
        written!(paths.tags().set(["new".to_owned()])?),
        written!(paths.tags().set_on_insert(["new".to_owned()])?),
    ])
}

/// The rows of the README's table for a nested model: held bare, in an `Option`, in a list, and
/// as an enum under a tag.
fn written_by_a_model() -> Result<Vec<(&'static str, String)>, WriteError> {
    let (paths, customer) = (Invoice::MONGO_FIELDS, Customer::MONGO_FIELDS);
    let (address, item, payment) = (
        Address::MONGO_FIELDS,
        LineItem::MONGO_FIELDS,
        Payment::MONGO_FIELDS,
    );
    Ok(vec![
        written!(
            paths
                .customer()
                .nest(customer.name().eq("Acme".to_owned())?)
        ),
        written!(
            paths.customer().nest(
                customer
                    .address()
                    .nest(address.city().eq("Moca".to_owned())?)
            )
        ),
        written!(
            paths
                .customer()
                .nest(customer.name().set("John".to_owned())?)
        ),
        written!(paths.customer().nest(customer.address().eq(moca())?)),
        written!(paths.customer().nest(customer.address().ne(moca())?)),
        written!(paths.customer().nest(customer.address().is_in([moca()])?)),
        written!(paths.customer().nest(customer.address().not_in([moca()])?)),
        written!(paths.customer().nest(customer.address().set(moca())?)),
        written!(
            paths
                .customer()
                .nest(customer.address().set_on_insert(moca())?)
        ),
        written!(paths.billing().nest(address.city().eq("Moca".to_owned())?)),
        written!(paths.billing().exists(true)),
        written!(paths.billing().unset()),
        written!(paths.billing().set(moca())?),
        written!(paths.items().nest(item.price().gt(100.5)?)),
        written!(
            paths.items().elem_match(
                item.quantity()
                    .gte(2)?
                    .and(item.sku().eq("B-7".to_owned())?)
            )
        ),
        written!(paths.items().size(2)),
        written!(paths.items().push(bolts())?),
        written!(paths.items().pull(item.quantity().lt(1)?)),
        written!(paths.items().set([bolts()])?),
        written!(paths.payment().nest(payment.is_card())),
        written!(paths.payment().nest(payment.is_cash())),
        written!(
            paths
                .payment()
                .nest(payment.card().last4().eq("4242".to_owned())?)
        ),
        written!(paths.payment().set(Payment::Cash)?),
    ])
}

/// The rows of the README's table for what joins filters and updates, and for a document
/// written by hand.
fn written_by_joining() -> Result<Vec<(&'static str, String)>, WriteError> {
    let paths = Invoice::MONGO_FIELDS;
    Ok(vec![
        written!(paths.total().gt(1000.5)?.and(paths.paid_at().exists(false))),
        written!(paths.total().gt(1000.5)?.or(paths.paid_at().exists(false))),
        written!(paths.paid_at().exists(true).negated()),
        written!(Filter::<Invoice>::raw(
            doc! { "details.costCenter": "CC-7" }
        )),
        written!(paths.total().set(99.5)?.and(paths.paid_at().unset())),
        written!(Update::<Invoice>::raw(doc! { "$inc": { "total": 12.5 } })),
    ])
}

#[test]
fn the_readme_declares_the_builders_that_compile_here() {
    let source = include_str!("paths.rs");
    for pinned in DECLARED_BUILDERS
        .into_iter()
        .chain(DECLARED_UNSEEN)
        .chain(DECLARED_ARMS)
        .chain([DECLARED_IMPORT])
    {
        assert_declared_and_documented(source, pinned);
    }
    // rustfmt lays the call in `counted` over several lines, with a comma after its last argument.
    assert_eq!(
        squeezed(source)
            .replace(",)", ")")
            .matches(&squeezed(DECLARED_COUNT))
            .count(),
        2,
        "`counted` no longer makes the call the README counts the rows of an arm with"
    );
    assert_documented(DECLARED_COUNT);
}

#[test]
fn the_readme_shows_the_document_each_call_writes() {
    let rows = [
        built(),
        written_by_a_value(),
        written_by_a_list(),
        written_by_a_model(),
        written_by_joining(),
        written_by_an_arm(),
        written_by_an_arm_update(),
    ];
    let missing: Vec<String> = rows
        .into_iter()
        .flat_map(Result::unwrap)
        .filter(|(call, document)| !row_is_documented(&[call], Some(document)))
        .map(|(call, document)| format!("| `{call}` | `{document}` |"))
        .collect();
    assert!(
        missing.is_empty(),
        "the README has no row for these calls and what they write:\n{}",
        missing.join("\n")
    );
}

#[test]
fn the_readme_names_the_kind_each_path_of_an_invoice_is() {
    use invoice_schema::{Field, ListField, Model, ModelList, OptionalField, OptionalModel};

    let paths = Invoice::MONGO_FIELDS;
    let rows = [
        kind!(paths.id(): Field<Invoice, ObjectId>),
        kind!(paths.billing(): OptionalModel<Invoice, Address>),
        kind!(paths.customer(): Model<Invoice, Customer>),
        kind!(paths.items(): ModelList<Invoice, LineItem>),
        kind!(paths.number(): Field<Invoice, String>),
        kind!(paths.paid_at(): OptionalField<Invoice, String>),
        kind!(paths.payment(): Model<Invoice, Payment>),
        kind!(paths.status(): Field<Invoice, InvoiceStatus>),
        kind!(paths.tags(): ListField<Invoice, String>),
        kind!(paths.total(): Field<Invoice, f64>),
    ];
    let missing: Vec<String> = rows
        .into_iter()
        .filter(|row| !row_is_documented(row, None))
        .map(|[member, kind]| format!("| `{member}` | `{kind}` |"))
        .collect();
    assert!(
        missing.is_empty(),
        "the README has no row for these paths and their kinds:\n{}",
        missing.join("\n")
    );
    assert_documented("| `details` | none: a map's keys are data |");
    // Declared below the type that holds it, so held whole: the README names this kind in prose.
    held::<shipment_schema::Field<Shipment, Carrier>>(&Shipment::MONGO_FIELDS.carrier());
    held::<Field<Invoice, f64>>(&TOTAL);
    assert_declared_and_documented(include_str!("paths.rs"), DECLARED_CONST);
}

#[test]
fn the_readme_names_the_function_and_the_model_of_each_arm() {
    let rows = [
        arm!(Transport.type__by_courier(): r#"#[serde(tag = "type")]"#, r#"#[serde(rename = "by-courier")] ByCourier(Courier)"#),
        arm!(Transport.type__collected(): r#"#[serde(tag = "type")]"#, "Collected"),
        arm!(Paper.doc_type__past_due(): r#"#[serde(tag = "docType", content = "body", rename_all = "kebab-case")]"#, "PastDue(Notice)"),
    ];
    let source = squeezed(include_str!("paths.rs"));
    for (named, row) in rows {
        let [attribute, variant, function] = row;
        assert!(
            row_is_documented(&row, None),
            "the README has no row for this arm:\n| `{attribute}` | `{variant}` | `{function}` |"
        );
        let declared = format!("{}pubenum{named}{{", squeezed(attribute));
        assert!(
            source.contains(&declared) && source.contains(&format!("{},", squeezed(variant))),
            "`{named}` is no longer declared under `{attribute}` with `{variant}`"
        );
    }
    for shown in SHOWN_ARM_MODELS {
        assert_documented(shown);
    }
    held::<transport_schema::ArmFilter<Transport, Courier>>(&BY_COURIER);
    held::<transport_schema::ArmFilter<Transport, ()>>(&Transport::MONGO_FIELDS.type__collected());
    held::<paper_schema::ArmFilter<Paper, Courier>>(&Paper::MONGO_FIELDS.doc_type__forwarded());
    held::<paper_schema::ArmFilter<Paper, Notice>>(&Paper::MONGO_FIELDS.doc_type__past_due());
    held::<payment_schema::ArmFilter<Payment, ()>>(&Payment::MONGO_FIELDS.kind__card());
    assert_eq!(
        format!("{:?}", transport_schema::Arm::ByCourier),
        "ByCourier"
    );
    // The call the README counts those rows with compiles, and is never run.
    held(&counted);
    assert_documented(&format!("| `{DECLARED_COUNT}` | the count of those rows |"));
}

/// The model an arm update is over, and the two calls the README changes the rows of an arm
/// with: the operation, and the driver's own call that upserts. Neither is run.
#[test]
fn the_readme_changes_the_rows_of_an_arm_through_its_update() {
    held::<transport_schema::ArmUpdate<Transport, Courier>>(&BY_COURIER.tag_on_insert());
    held::<payment_schema::ArmUpdate<Payment, ()>>(
        &Payment::MONGO_FIELDS.kind__cash().tag_on_insert(),
    );
    let rows = [
        (
            called!(collection: Booking::update_one(&collection, couriered("SDQ-1", "Acme")?, recouriered("Acme Freight")?).await?),
            "the driver's `UpdateResult`",
        ),
        (
            called!(collection: collection.update_one(couriered("SDQ-1", "Acme")?.into_document(), recouriered("Acme Freight")?.into_document()).upsert(true).await?),
            "the same, as an upsert: the driver's own call",
        ),
    ];
    for (call, answered) in rows {
        assert!(
            call_is_documented(call, answered),
            "the README has no row for this call:\n| `{call}` | {answered} |"
        );
    }
}
