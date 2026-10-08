//! The query types `#[model_schema(decode_with)]` adds under `mongodb`: `Filter`, `Update`, and
//! the typed paths that build them. Two binaries compile this module, each with the name `bson`
//! bound to one major version of the library, so every case runs against both.
//!
//! Every model's paths are the ones generated for it: `MONGO_FIELDS` on the type, a struct of
//! paths named `MongoFields` in the type's own module. A model is declared above the model that
//! holds it wherever a path below it is asked for. A document is asserted as [`shown`] prints it,
//! with the BSON type of each number and date spelled out: that text is the same under both
//! versions of the library.

mod hooked;
mod nested;
// A build that describes a type refuses a renaming whose two directions name two keys.
#[cfg(not(any(feature = "typescript", feature = "zod", feature = "jsonschema")))]
mod one_direction;
mod one_value;
mod operators;
mod readme;
mod shadowing;
mod shapes;
mod stored;
mod unseen;
mod wrapper_names;
mod written_names;

use std::collections::HashMap;

use bson::oid::ObjectId;
use bson::{Bson, Document, doc};
use serde::{Deserialize, Serialize};
use tixschema::model_schema;

use self::shapes::{
    Audit, Delivery, Payment, Rebate, Reference, Wrapper, audit_schema, delivery_schema,
    payment_schema, rebate_schema, reference_schema, wrapper_schema,
};

/// Third level of the nested chain: `Invoice` -> `Customer` -> `Address`.
#[model_schema(decode_with)]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct Address {
    city: String,
    postal_code: String,
}

/// Second level of the nested chain, held by both [`Invoice`] and [`Order`].
#[model_schema(decode_with)]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct Customer {
    address: Address,
    name: String,
    open_invoices: u32,
}

/// A brand over an `ObjectId`: serde writes it as the id it holds.
#[model_schema(decode_with)]
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(transparent)]
struct InvoiceId(ObjectId);

#[model_schema(decode_with)]
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
enum InvoiceStatus {
    Draft,
    Paid,
    PastDue,
}

/// The element of a list of models.
#[model_schema(decode_with)]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct LineItem {
    // Named apart from the key serde writes it under: a path is under the field's name and writes
    // the key.
    #[serde(rename = "sku")]
    article: String,
    price: f64,
    quantity: u32,
}

/// serde writes a tuple struct as an array, so each slot's path is its position.
#[model_schema(decode_with)]
#[derive(Clone, Debug, Deserialize, Serialize)]
struct Point(f64, f64);

/// One field per shape a path is written for: a value every row holds, one a row may leave out,
/// a list of plain values, a map, and a nested model held bare, in an `Option`, in a list and
/// flattened, as a struct, as a tuple struct, as a generic struct and as an enum in each form
/// serde writes one.
#[model_schema(decode_with)]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct Invoice {
    #[serde(flatten)]
    audit: Audit,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    billing: Option<Address>,
    customer: Customer,
    delivery: Delivery,
    details: HashMap<String, String>,
    discount: Rebate,
    #[serde(rename = "_id")]
    id: InvoiceId,
    items: Vec<LineItem>,
    number: String,
    origin: Point,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    paid_at: Option<String>,
    payment: Payment,
    reference: Reference,
    scores: Vec<u32>,
    shipping: Wrapper<Address>,
    status: InvoiceStatus,
    tags: Vec<String>,
    total: f64,
}

/// A second row type that holds [`Customer`], as [`Invoice`] does.
#[model_schema(decode_with)]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct Order {
    customer: Customer,
    placed: bool,
    reference: String,
}

/// A filter or an update as text: the BSON type of each number and date spelled out, in one form
/// under both versions of the library.
fn shown<D>(written: D) -> String
where
    D: Into<Document>,
{
    shown_document(&written.into())
}

fn shown_document(document: &Document) -> String {
    if document.is_empty() {
        return "{}".to_owned();
    }
    let entries: Vec<String> = document
        .iter()
        .map(|(key, value)| format!("{key:?}: {}", shown_value(value)))
        .collect();
    format!("{{ {} }}", entries.join(", "))
}

fn shown_value(value: &Bson) -> String {
    if let Bson::Double(number) = value {
        format!("Double({number:?})")
    } else if let Bson::Int64(number) = value {
        format!("Int64({number})")
    } else if let Bson::String(text) = value {
        format!("{text:?}")
    } else if let Bson::Boolean(flag) = value {
        flag.to_string()
    } else if let Bson::DateTime(at) = value {
        format!("Date({} ms)", at.timestamp_millis())
    } else if let Bson::ObjectId(id) = value {
        format!("ObjectId({})", id.to_hex())
    } else if let Bson::Array(items) = value {
        let members: Vec<String> = items.iter().map(shown_value).collect();
        format!("[{}]", members.join(", "))
    } else if let Bson::Document(document) = value {
        shown_document(document)
    } else {
        format!("{value:?}")
    }
}

/// `$and`, `$or` and `$nor` in one filter, over a required value, a number, a list, a path below
/// a nested model, and a value a row may leave out.
#[test]
fn filters_over_every_kind_of_path_nest_under_and_or_and_nor() {
    let invoice = Invoice::MONGO_FIELDS;
    let overdue_or_large = invoice
        .status
        .eq(InvoiceStatus::PastDue)
        .unwrap()
        .or(invoice.total.gt(1000.0_f64).unwrap());
    let export_or_few_open = invoice
        .tags
        .contains("export".to_owned())
        .unwrap()
        .or(invoice.customer.open_invoices.lte(3_u32).unwrap());
    let unpaid = invoice.paid_at.exists(true).negated();
    assert_eq!(
        shown(overdue_or_large.and(export_or_few_open).or(unpaid)),
        r#"{ "$or": [{ "$and": [{ "$or": [{ "status": { "$eq": "past-due" } }, { "total": { "$gt": Double(1000.0) } }] }, { "$or": [{ "tags": { "$eq": "export" } }, { "customer.openInvoices": { "$lte": Int64(3) } }] }] }, { "$nor": [{ "paidAt": { "$exists": true } }] }] }"#
    );
}

/// A filter, an update, and a document written by hand for a path with no typed form, each from
/// the paths generated for the row.
#[test]
fn generated_paths_write_a_filter_an_update_and_a_raw_document() {
    let invoice = Invoice::MONGO_FIELDS;
    let overdue = invoice
        .status
        .eq(InvoiceStatus::PastDue)
        .unwrap()
        .or(invoice.total.gt(1000.0_f64).unwrap())
        .and(invoice.tags.contains("export".to_owned()).unwrap())
        .or(invoice.paid_at.exists(true).negated());
    assert_eq!(
        shown(overdue),
        r#"{ "$or": [{ "$and": [{ "$or": [{ "status": { "$eq": "past-due" } }, { "total": { "$gt": Double(1000.0) } }] }, { "tags": { "$eq": "export" } }] }, { "$nor": [{ "paidAt": { "$exists": true } }] }] }"#
    );
    let settled = invoice
        .status
        .set(InvoiceStatus::Paid)
        .unwrap()
        .and(invoice.paid_at.unset());
    assert_eq!(
        shown(settled),
        r#"{ "$set": { "status": "paid" }, "$unset": { "paidAt": "" } }"#
    );
    let by_cost_center =
        invoice_schema::Filter::<Invoice>::raw(doc! { "details.costCenter": "CC-7" });
    assert_eq!(shown(by_cost_center), r#"{ "details.costCenter": "CC-7" }"#);
}

/// A path built through the constructors alone, as a consumer builds one below a member that is
/// one whole value, writes what the path generated for the same key writes.
#[test]
fn a_path_built_through_the_constructors_writes_what_the_generated_one_does() {
    use invoice_schema::{Field, ListField, MongoPath, OptionalField};

    let invoice = Invoice::MONGO_FIELDS;
    let total: Field<Invoice, f64> = Field::plain(MongoPath::under(MongoPath::ROOT, "total"));
    let tags: ListField<Invoice, String> =
        ListField::plain(MongoPath::under(MongoPath::ROOT, "tags"));
    let paid_at: OptionalField<Invoice, String> =
        OptionalField::plain(MongoPath::under(MongoPath::ROOT, "paidAt"));
    assert_eq!(
        shown(total.gt(1000.0_f64).unwrap()),
        shown(invoice.total.gt(1000.0_f64).unwrap())
    );
    assert_eq!(
        shown(tags.contains("export".to_owned()).unwrap()),
        shown(invoice.tags.contains("export".to_owned()).unwrap())
    );
    assert_eq!(
        shown(paid_at.exists(true)),
        shown(invoice.paid_at.exists(true))
    );
}

/// A map has no path: its keys are data. A document written by hand names one of them, and joins
/// the typed filters through `Filter::raw`.
#[test]
fn a_key_of_a_map_is_named_by_a_document_written_by_hand() {
    let by_cost_center = Invoice::MONGO_FIELDS
        .number
        .eq("INV-0042".to_owned())
        .unwrap()
        .and(invoice_schema::Filter::<Invoice>::raw(
            doc! { "details.costCenter": "CC-7" },
        ));
    assert_eq!(
        shown(by_cost_center),
        r#"{ "$and": [{ "number": { "$eq": "INV-0042" } }, { "details.costCenter": "CC-7" }] }"#
    );
}

/// A key under the row is the path of its one key, a path of the prefix itself has no key of its
/// own, and the row itself has none.
#[test]
fn a_path_is_the_keys_leading_to_it_joined_by_dots() {
    use invoice_schema::MongoPath;

    assert_eq!(MongoPath::at(MongoPath::ROOT).key(), "");
    let customer = MongoPath::under(MongoPath::ROOT, "customer");
    assert_eq!(customer.key(), "customer");
    assert_eq!(MongoPath::at(customer.segments).key(), "customer");
    let city = MongoPath::under(
        MongoPath::under(customer.segments, "address").segments,
        "city",
    );
    assert_eq!(city.key(), "customer.address.city");
    assert_eq!(
        Invoice::MONGO_FIELDS.total.segments(),
        MongoPath::under(MongoPath::ROOT, "total").segments
    );
}
