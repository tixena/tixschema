//! What the README shows of the typed paths: the kind each member of `Invoice::MONGO_FIELDS` is,
//! the document each operator writes, the filters and updates its examples build, and the paths
//! below a member tixschema holds as one whole value.
//!
//! A table row is read off the code: a call is turned into the row's first cell by `stringify!`,
//! and the document it writes into the second.

use bson::oid::ObjectId;
use bson::{Document, doc};
use serde::{Deserialize, Serialize};
use tixschema::model_schema;

use super::invoice_schema;
use super::{
    Address, Customer, Invoice, InvoiceStatus, LineItem, Payment, assert_declared_and_documented,
    assert_documented, readme,
};

use invoice_schema::{Filter, Update, WriteError};

/// One row of the README's table of kinds: a member of `Invoice`'s paths, held by the compiler
/// to being of the kind the row names.
macro_rules! kind {
    ($paths:ident . $member:ident : $($kind:tt)+) => {{
        held::<$($kind)+>(&$paths.$member);
        [stringify!($member), stringify!($($kind)+)]
    }};
}

/// One row of a README table of documents: a call as it is written, and the document it writes.
macro_rules! written {
    ($($call:tt)+) => {
        (stringify!($($call)+), Document::from($($call)+).to_string())
    };
}

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
    let paths = Invoice::MONGO_FIELDS;
    Ok(paths
        .status
        .eq(InvoiceStatus::PastDue)?
        .or(paths.total.gt(above)?)
        .and(paths.customer.address.city.eq(city.to_owned())?)
        .and(paths.paid_at.exists(false)))
}",
    r#"/// What paying an invoice changes.
fn paid(at: &str) -> Result<Update<Invoice>, WriteError> {
    let paths = Invoice::MONGO_FIELDS;
    Ok(paths
        .status
        .set(InvoiceStatus::Paid)?
        .and(paths.paid_at.set(at.to_owned())?)
        .and(paths.customer.open_invoices.set(0)?)
        .and(paths.tags.push("settled".to_owned())?))
}"#,
    r#"/// Invoices of one cost center that are no draft. `details` is a map: its keys are data.
fn of_cost_center(code: &str) -> Result<Filter<Invoice>, WriteError> {
    let paths = Invoice::MONGO_FIELDS;
    Ok(paths
        .status
        .ne(InvoiceStatus::Draft)?
        .and(Filter::raw(doc! { "details.costCenter": code })))
}"#,
    r#"/// A surcharge: `$inc`, an operator the typed paths do not write, beside one they do.
fn surcharged(by: f64) -> Result<Update<Invoice>, WriteError> {
    let paths = Invoice::MONGO_FIELDS;
    Ok(paths
        .tags
        .push("surcharged".to_owned())?
        .and(Update::raw(doc! { "$inc": { "total": by } })))
}"#,
];

/// The line the README brings the query types into scope with.
const DECLARED_IMPORT: &str = "use invoice_schema::{Filter, Update, WriteError};";

/// The two types the README declares one below the other, and the function that reaches the
/// paths below the one tixschema has not seen, character for character.
const DECLARED_UNSEEN: [&str; 3] = [
    "#[model_schema(decode_with)]
#[derive(Debug, Deserialize, Serialize)]
pub struct Shipment {
    pub carrier: Carrier,
    pub code: String,
}",
    "#[model_schema(decode_with)]
#[derive(Debug, Deserialize, Serialize)]
pub struct Carrier {
    pub name: String,
}",
    r#"/// Shipments of one carrier. `Carrier` is declared below `Shipment`, so the paths below it are
/// built by `Carrier`'s own function, under the keys the member's path holds.
fn carried_by(name: &str) -> Result<shipment_schema::Filter<Shipment>, WriteError> {
    let shipment = Shipment::MONGO_FIELDS;
    let carrier = Carrier::mongo_fields_under::<Shipment>(shipment.carrier.segments());
    Ok(shipment
        .code
        .regex("^S-", "")
        .and(carrier.name.eq(name.to_owned())?))
}"#,
];

#[model_schema(decode_with)]
#[derive(Debug, Deserialize, Serialize)]
pub struct Shipment {
    pub carrier: Carrier,
    pub code: String,
}

#[model_schema(decode_with)]
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
    ])
}

/// Shipments of one carrier. `Carrier` is declared below `Shipment`, so the paths below it are
/// built by `Carrier`'s own function, under the keys the member's path holds.
fn carried_by(name: &str) -> Result<shipment_schema::Filter<Shipment>, WriteError> {
    let shipment = Shipment::MONGO_FIELDS;
    let carrier = Carrier::mongo_fields_under::<Shipment>(shipment.carrier.segments());
    Ok(shipment
        .code
        .regex("^S-", "")
        .and(carrier.name.eq(name.to_owned())?))
}

/// The cells of one row of a README table whose every cell is code.
fn cells(line: &str) -> Option<Vec<&str>> {
    let inner = line.strip_prefix("| `")?.strip_suffix("` |")?;
    Some(inner.split("` | `").collect())
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
        .status
        .ne(InvoiceStatus::Draft)?
        .and(Filter::raw(doc! { "details.costCenter": code })))
}

/// What paying an invoice changes.
fn paid(at: &str) -> Result<Update<Invoice>, WriteError> {
    let paths = Invoice::MONGO_FIELDS;
    Ok(paths
        .status
        .set(InvoiceStatus::Paid)?
        .and(paths.paid_at.set(at.to_owned())?)
        .and(paths.customer.open_invoices.set(0)?)
        .and(paths.tags.push("settled".to_owned())?))
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
        .tags
        .push("surcharged".to_owned())?
        .and(Update::raw(doc! { "$inc": { "total": by } })))
}

/// Unpaid invoices of one city that are past due or above an amount.
fn to_chase(city: &str, above: f64) -> Result<Filter<Invoice>, WriteError> {
    let paths = Invoice::MONGO_FIELDS;
    Ok(paths
        .status
        .eq(InvoiceStatus::PastDue)?
        .or(paths.total.gt(above)?)
        .and(paths.customer.address.city.eq(city.to_owned())?)
        .and(paths.paid_at.exists(false)))
}

/// The rows of the README's table for a path of one value every row holds, and for one a row
/// may leave out.
fn written_by_a_value() -> Result<Vec<(&'static str, String)>, WriteError> {
    let paths = Invoice::MONGO_FIELDS;
    Ok(vec![
        written!(paths.number.eq("INV-0042".to_owned())?),
        written!(paths.number.ne("INV-0042".to_owned())?),
        written!(paths.total.gt(1000.5)?),
        written!(paths.total.gte(1000.5)?),
        written!(paths.total.lt(1000.5)?),
        written!(paths.total.lte(1000.5)?),
        written!(
            paths
                .number
                .is_in(["INV-0042".to_owned(), "INV-0043".to_owned()])?
        ),
        written!(paths.number.not_in(["INV-0042".to_owned()])?),
        written!(paths.number.regex("^INV-", "i")),
        written!(paths.total.set(99.5)?),
        written!(paths.number.set_on_insert("INV-0099".to_owned())?),
        written!(paths.paid_at.exists(false)),
        written!(paths.paid_at.unset()),
        written!(paths.paid_at.lt("2026-10-01".to_owned())?),
        written!(paths.status.eq(InvoiceStatus::PastDue)?),
        written!(
            paths
                .status
                .is_in([InvoiceStatus::Draft, InvoiceStatus::PastDue])?
        ),
        written!(paths.status.set(InvoiceStatus::Paid)?),
    ])
}

/// The rows of the README's table for a list of plain values.
fn written_by_a_list() -> Result<Vec<(&'static str, String)>, WriteError> {
    let paths = Invoice::MONGO_FIELDS;
    Ok(vec![
        written!(paths.tags.contains("export".to_owned())?),
        written!(
            paths
                .tags
                .contains_any(["export".to_owned(), "priority".to_owned()])?
        ),
        written!(paths.tags.contains_none(["void".to_owned()])?),
        written!(paths.tags.size(0)),
        written!(
            paths.tags.elem_match(
                paths
                    .tags
                    .element()
                    .gte("a".to_owned())?
                    .lt("n".to_owned())?
            )
        ),
        written!(paths.tags.push("reviewed".to_owned())?),
        written!(paths.tags.pull("draft".to_owned())?),
        written!(paths.tags.set(["new".to_owned()])?),
        written!(paths.tags.set_on_insert(["new".to_owned()])?),
    ])
}

/// The rows of the README's table for a nested model: held bare, in an `Option`, in a list, and
/// as an enum under a tag.
fn written_by_a_model() -> Result<Vec<(&'static str, String)>, WriteError> {
    let paths = Invoice::MONGO_FIELDS;
    let item = LineItem::MONGO_FIELDS;
    Ok(vec![
        written!(paths.customer.name.eq("Acme".to_owned())?),
        written!(paths.customer.address.city.eq("Moca".to_owned())?),
        written!(paths.customer.address.eq(moca())?),
        written!(paths.customer.address.ne(moca())?),
        written!(paths.customer.address.is_in([moca()])?),
        written!(paths.customer.address.not_in([moca()])?),
        written!(paths.customer.address.set(moca())?),
        written!(paths.customer.address.set_on_insert(moca())?),
        written!(paths.billing.city.eq("Moca".to_owned())?),
        written!(paths.billing.exists(true)),
        written!(paths.billing.unset()),
        written!(paths.billing.set(moca())?),
        written!(paths.items.price.gt(100.5)?),
        written!(
            paths
                .items
                .elem_match(item.quantity.gte(2)?.and(item.sku.eq("B-7".to_owned())?))
        ),
        written!(paths.items.size(2)),
        written!(paths.items.push(bolts())?),
        written!(paths.items.pull(item.quantity.lt(1)?)),
        written!(paths.items.set([bolts()])?),
        written!(paths.payment.is_card()),
        written!(paths.payment.is_cash()),
        written!(paths.payment.card.last4.eq("4242".to_owned())?),
        written!(paths.payment.set(Payment::Cash)?),
    ])
}

/// The rows of the README's table for what joins filters and updates, and for a document
/// written by hand.
fn written_by_joining() -> Result<Vec<(&'static str, String)>, WriteError> {
    let paths = Invoice::MONGO_FIELDS;
    Ok(vec![
        written!(paths.total.gt(1000.5)?.and(paths.paid_at.exists(false))),
        written!(paths.total.gt(1000.5)?.or(paths.paid_at.exists(false))),
        written!(paths.paid_at.exists(true).negated()),
        written!(Filter::<Invoice>::raw(
            doc! { "details.costCenter": "CC-7" }
        )),
        written!(paths.total.set(99.5)?.and(paths.paid_at.unset())),
        written!(Update::<Invoice>::raw(doc! { "$inc": { "total": 12.5 } })),
    ])
}

#[test]
fn the_readme_declares_the_builders_that_compile_here() {
    let source = include_str!("paths.rs");
    for pinned in DECLARED_BUILDERS
        .into_iter()
        .chain(DECLARED_UNSEEN)
        .chain([DECLARED_IMPORT])
    {
        assert_declared_and_documented(source, pinned);
    }
}

#[test]
fn the_readme_shows_the_document_each_call_writes() {
    let rows = [
        built(),
        written_by_a_value(),
        written_by_a_list(),
        written_by_a_model(),
        written_by_joining(),
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
        kind!(paths.id: Field<Invoice, ObjectId>),
        kind!(paths.billing: OptionalModel<Invoice, Address, _>),
        kind!(paths.customer: Model<Invoice, Customer, _>),
        kind!(paths.items: ModelList<Invoice, LineItem, _>),
        kind!(paths.number: Field<Invoice, String>),
        kind!(paths.paid_at: OptionalField<Invoice, String>),
        kind!(paths.payment: Model<Invoice, Payment, _>),
        kind!(paths.status: Model<Invoice, InvoiceStatus, _>),
        kind!(paths.tags: ListField<Invoice, String>),
        kind!(paths.total: Field<Invoice, f64>),
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
    held::<shipment_schema::Field<Shipment, Carrier>>(&Shipment::MONGO_FIELDS.carrier);
}
