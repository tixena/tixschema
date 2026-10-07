//! The query types `#[model_schema(decode_with)]` adds under `mongodb`: `Filter`, `Update`, and
//! the typed paths that build them. Two binaries compile this module, each with the name `bson`
//! bound to one major version of the library, so every case runs against both.
//!
//! No type has paths generated for it yet, so each model's paths are built here by hand, through
//! the constructors a type's own expansion calls, and held in one struct per model. A document is
//! asserted as [`shown`] prints it, with the BSON type of each number and date spelled out: that
//! text is the same under both versions of the library.

mod hooked;
mod nested;
mod operators;
mod shadowing;
mod wrapper_names;

use bson::oid::ObjectId;
use bson::{Bson, Document, doc};
use serde::{Deserialize, Serialize};
use tixschema::model_schema;

/// The paths of an [`Address`] that is the row itself.
const ADDRESS: AddressPaths<Address> = address_paths(address_schema::MongoPath::ROOT);

/// The paths of a [`Customer`] that is the row itself.
const CUSTOMER: CustomerPaths<Customer> = customer_paths(customer_schema::MongoPath::ROOT);

/// The paths of an [`Invoice`] that is the row itself.
const INVOICE: InvoicePaths<Invoice> = invoice_paths(invoice_schema::MongoPath::ROOT);

/// The paths of a [`LineItem`] that is the row itself, as a filter over one element is built.
const LINE_ITEM: LineItemPaths<LineItem> = line_item_paths(line_item_schema::MongoPath::ROOT);

/// The paths of an [`Order`] that is the row itself.
const ORDER: OrderPaths<Order> = order_paths(order_schema::MongoPath::ROOT);

/// The keys leading to a value, in the form they cross from one type's code to another's.
type Keys = [Option<&'static str>; 8];

/// Third level of the nested chain: `Invoice` -> `Customer` -> `Address`.
#[model_schema(decode_with)]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct Address {
    city: String,
    postal_code: String,
}

/// The paths of an [`Address`], under whatever leads to it in a row of `Root`.
struct AddressPaths<Root> {
    city: address_schema::Field<Root, String>,
    postal_code: address_schema::Field<Root, String>,
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

/// The paths of a [`Customer`], under whatever leads to it in a row of `Root`.
struct CustomerPaths<Root> {
    address: customer_schema::Model<Root, Address, AddressPaths<Root>>,
    name: customer_schema::Field<Root, String>,
    open_invoices: customer_schema::Field<Root, u32>,
}

/// One field per kind of path: a value every row holds, one a row may leave out, a list of plain
/// values, and a nested model held bare, in an `Option` and in a list.
#[model_schema(decode_with)]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct Invoice {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    billing: Option<Address>,
    customer: Customer,
    #[serde(rename = "_id")]
    id: InvoiceId,
    items: Vec<LineItem>,
    number: String,
    origin: Point,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    paid_at: Option<String>,
    scores: Vec<u32>,
    status: InvoiceStatus,
    tags: Vec<String>,
    total: f64,
}

/// A brand over an `ObjectId`: serde writes it as the id it holds.
#[model_schema(decode_with)]
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(transparent)]
struct InvoiceId(ObjectId);

/// The paths of an [`Invoice`], under whatever leads to it in a row of `Root`.
struct InvoicePaths<Root> {
    billing: invoice_schema::OptionalModel<Root, Address, AddressPaths<Root>>,
    customer: invoice_schema::Model<Root, Customer, CustomerPaths<Root>>,
    id: invoice_schema::Field<Root, InvoiceId>,
    items: invoice_schema::ModelList<Root, LineItem, LineItemPaths<Root>>,
    number: invoice_schema::Field<Root, String>,
    origin: invoice_schema::Model<Root, Point, PointPaths<Root>>,
    paid_at: invoice_schema::OptionalField<Root, String>,
    scores: invoice_schema::ListField<Root, u32>,
    status: invoice_schema::Field<Root, InvoiceStatus>,
    tags: invoice_schema::ListField<Root, String>,
    total: invoice_schema::Field<Root, f64>,
}

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

/// The paths of a [`LineItem`], under whatever leads to it in a row of `Root`. Each is under the
/// field's own name and writes the key serde does.
struct LineItemPaths<Root> {
    article: line_item_schema::Field<Root, String>,
    price: line_item_schema::Field<Root, f64>,
    quantity: line_item_schema::Field<Root, u32>,
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

/// The paths of an [`Order`], under whatever leads to it in a row of `Root`.
struct OrderPaths<Root> {
    customer: order_schema::Model<Root, Customer, CustomerPaths<Root>>,
    placed: order_schema::Field<Root, bool>,
}

/// serde writes a tuple struct as an array, so each slot's path is its position.
#[model_schema(decode_with)]
#[derive(Clone, Debug, Deserialize, Serialize)]
struct Point(f64, f64);

/// The paths of a [`Point`], under whatever leads to it in a row of `Root`.
struct PointPaths<Root>(
    point_schema::Field<Root, f64>,
    point_schema::Field<Root, f64>,
);

const fn address_paths<Root>(prefix: Keys) -> AddressPaths<Root> {
    AddressPaths {
        city: address_schema::Field::plain(address_schema::MongoPath::under(prefix, "city")),
        postal_code: address_schema::Field::plain(address_schema::MongoPath::under(
            prefix,
            "postalCode",
        )),
    }
}

/// Names nothing of an [`Address`] but its type: the nested paths are built by its own function,
/// under this type's key for it.
const fn customer_paths<Root>(prefix: Keys) -> CustomerPaths<Root> {
    CustomerPaths {
        address: customer_schema::Model::plain(
            customer_schema::MongoPath::under(prefix, "address"),
            address_paths(customer_schema::MongoPath::under(prefix, "address").segments),
        ),
        name: customer_schema::Field::plain(customer_schema::MongoPath::under(prefix, "name")),
        open_invoices: customer_schema::Field::plain(customer_schema::MongoPath::under(
            prefix,
            "openInvoices",
        )),
    }
}

const fn invoice_paths<Root>(prefix: Keys) -> InvoicePaths<Root> {
    InvoicePaths {
        billing: invoice_schema::OptionalModel::plain(
            invoice_schema::MongoPath::under(prefix, "billing"),
            address_paths(invoice_schema::MongoPath::under(prefix, "billing").segments),
        ),
        customer: invoice_schema::Model::plain(
            invoice_schema::MongoPath::under(prefix, "customer"),
            customer_paths(invoice_schema::MongoPath::under(prefix, "customer").segments),
        ),
        id: invoice_schema::Field::plain(invoice_schema::MongoPath::under(prefix, "_id")),
        items: invoice_schema::ModelList::plain(
            invoice_schema::MongoPath::under(prefix, "items"),
            line_item_paths(invoice_schema::MongoPath::under(prefix, "items").segments),
        ),
        number: invoice_schema::Field::plain(invoice_schema::MongoPath::under(prefix, "number")),
        origin: invoice_schema::Model::plain(
            invoice_schema::MongoPath::under(prefix, "origin"),
            point_paths(invoice_schema::MongoPath::under(prefix, "origin").segments),
        ),
        paid_at: invoice_schema::OptionalField::plain(invoice_schema::MongoPath::under(
            prefix, "paidAt",
        )),
        scores: invoice_schema::ListField::plain(invoice_schema::MongoPath::under(
            prefix, "scores",
        )),
        status: invoice_schema::Field::plain(invoice_schema::MongoPath::under(prefix, "status")),
        tags: invoice_schema::ListField::plain(invoice_schema::MongoPath::under(prefix, "tags")),
        total: invoice_schema::Field::plain(invoice_schema::MongoPath::under(prefix, "total")),
    }
}

const fn line_item_paths<Root>(prefix: Keys) -> LineItemPaths<Root> {
    LineItemPaths {
        article: line_item_schema::Field::plain(line_item_schema::MongoPath::under(prefix, "sku")),
        price: line_item_schema::Field::plain(line_item_schema::MongoPath::under(prefix, "price")),
        quantity: line_item_schema::Field::plain(line_item_schema::MongoPath::under(
            prefix, "quantity",
        )),
    }
}

const fn order_paths<Root>(prefix: Keys) -> OrderPaths<Root> {
    OrderPaths {
        customer: order_schema::Model::plain(
            order_schema::MongoPath::under(prefix, "customer"),
            customer_paths(order_schema::MongoPath::under(prefix, "customer").segments),
        ),
        placed: order_schema::Field::plain(order_schema::MongoPath::under(prefix, "placed")),
    }
}

const fn point_paths<Root>(prefix: Keys) -> PointPaths<Root> {
    PointPaths(
        point_schema::Field::plain(point_schema::MongoPath::under(prefix, "0")),
        point_schema::Field::plain(point_schema::MongoPath::under(prefix, "1")),
    )
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
    let invoice = INVOICE;
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

/// Paths built where they are used, through the constructors alone, as a consumer reads them once
/// they are generated: a filter, an update, and a document written by hand for a path with no
/// typed form.
#[test]
fn paths_built_through_the_constructors_write_a_filter_an_update_and_a_raw_document() {
    use invoice_schema::{Field, Filter, ListField, MongoPath, OptionalField};

    let status: Field<Invoice, InvoiceStatus> =
        Field::plain(MongoPath::under(MongoPath::ROOT, "status"));
    let total: Field<Invoice, f64> = Field::plain(MongoPath::under(MongoPath::ROOT, "total"));
    let tags: ListField<Invoice, String> =
        ListField::plain(MongoPath::under(MongoPath::ROOT, "tags"));
    let paid_at: OptionalField<Invoice, String> =
        OptionalField::plain(MongoPath::under(MongoPath::ROOT, "paidAt"));

    let overdue = status
        .eq(InvoiceStatus::PastDue)
        .unwrap()
        .or(total.gt(1000.0_f64).unwrap())
        .and(tags.contains("export".to_owned()).unwrap())
        .or(paid_at.exists(true).negated());
    assert_eq!(
        shown(overdue),
        r#"{ "$or": [{ "$and": [{ "$or": [{ "status": { "$eq": "past-due" } }, { "total": { "$gt": Double(1000.0) } }] }, { "tags": { "$eq": "export" } }] }, { "$nor": [{ "paidAt": { "$exists": true } }] }] }"#
    );
    let settled = status
        .set(InvoiceStatus::Paid)
        .unwrap()
        .and(paid_at.unset());
    assert_eq!(
        shown(settled),
        r#"{ "$set": { "status": "paid" }, "$unset": { "paidAt": "" } }"#
    );
    let by_cost_center = Filter::<Invoice>::raw(doc! { "details.costCenter": "CC-7" });
    assert_eq!(shown(by_cost_center), r#"{ "details.costCenter": "CC-7" }"#);
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
        INVOICE.total.segments(),
        MongoPath::under(MongoPath::ROOT, "total").segments
    );
}
