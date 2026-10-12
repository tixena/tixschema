//! A nested model's paths, which `nest` writes under the key the outer type holds it at, and its
//! filters joined with the outer type's: three types deep, each one's module holding its own
//! `Filter` and `Update`, and each one's expansion writing its own keys alone.

use core::marker::PhantomData;

use bson::{Document, doc};
use serde::{Deserialize, Serialize};
use tixschema::model_schema;

use super::shapes::{Audit, Payment};
use super::{
    Address, Customer, Invoice, InvoiceStatus, Order, customer_schema, invoice_schema, shown,
};

/// The element of the list of models a [`Hamper`] holds.
#[model_schema(decode_with)]
#[derive(Clone, Debug, Deserialize, Serialize)]
struct Treat {
    weight: u32,
}

/// A list of plain values, a list of models and a value a row may leave out: one member per
/// update operator that takes one.
#[model_schema(decode_with)]
#[derive(Clone, Debug, Deserialize, Serialize)]
struct Hamper {
    items: Vec<Treat>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    note: Option<String>,
    tags: Vec<String>,
}

/// Holds [`Hamper`] one type down.
#[model_schema(decode_with)]
#[derive(Clone, Debug, Deserialize, Serialize)]
struct Consignment {
    hamper: Hamper,
}

/// What a generated read of `Invoice` asks of its filter: a filter over the rows of `Invoice`,
/// of whichever module's `Filter` type.
fn taken_by_a_read<F>(filter: F) -> Document
where
    F: Into<Document> + AsRef<PhantomData<Invoice>>,
{
    filter.into()
}

/// What a generated write of `Invoice` asks of its filter and of its update: an update carries
/// the rows it changes, so neither is taken where the other is asked.
fn taken_by_a_write<F, U>(filter: F, update: U) -> (Document, Document)
where
    F: Into<Document> + AsRef<PhantomData<Invoice>>,
    U: Into<Document> + AsRef<PhantomData<fn(Invoice) -> Invoice>>,
{
    (filter.into(), update.into())
}

/// A filter over the rows of `Address`, asked of the one a customer holds.
fn in_city(city: &str) -> customer_schema::Filter<Customer> {
    Customer::MONGO_FIELDS
        .address()
        .nest(Address::MONGO_FIELDS.city().eq(city.to_owned()).unwrap())
}

#[test]
fn a_nested_path_writes_every_key_that_leads_to_it() {
    let (invoice, customer) = (Invoice::MONGO_FIELDS, Customer::MONGO_FIELDS);
    assert_eq!(
        shown(invoice.customer().nest(in_city("Santo Domingo"))),
        r#"{ "customer.address.city": { "$eq": "Santo Domingo" } }"#
    );
    let by_code = Address::MONGO_FIELDS
        .postal_code()
        .eq("10101".to_owned())
        .unwrap();
    assert_eq!(
        shown(invoice.customer().nest(customer.address().nest(by_code))),
        r#"{ "customer.address.postalCode": { "$eq": "10101" } }"#
    );
    assert_eq!(
        shown(
            invoice
                .customer()
                .nest(customer.name().eq("Acme Ltd".to_owned()).unwrap())
        ),
        r#"{ "customer.name": { "$eq": "Acme Ltd" } }"#
    );
    assert_eq!(
        shown(invoice.total().gt(1.0_f64).unwrap()),
        r#"{ "total": { "$gt": Double(1.0) } }"#
    );
}

#[test]
fn a_models_paths_start_at_whatever_holds_it() {
    assert_eq!(
        shown(Order::MONGO_FIELDS.customer().nest(in_city("Santiago"))),
        r#"{ "customer.address.city": { "$eq": "Santiago" } }"#
    );
    assert_eq!(
        shown(in_city("Santiago")),
        r#"{ "address.city": { "$eq": "Santiago" } }"#
    );
    assert_eq!(
        shown(
            Address::MONGO_FIELDS
                .city()
                .eq("Santiago".to_owned())
                .unwrap()
        ),
        r#"{ "city": { "$eq": "Santiago" } }"#
    );
}

#[test]
fn the_filters_of_three_modules_join_into_one() {
    let (invoice, customer) = (Invoice::MONGO_FIELDS, Customer::MONGO_FIELDS);
    assert_eq!(
        shown(
            invoice
                .total()
                .gt(1000.0_f64)
                .unwrap()
                .and(
                    invoice
                        .customer()
                        .nest(customer.name().eq("Acme Ltd".to_owned()).unwrap())
                )
                .and(invoice.customer().nest(in_city("Santo Domingo")))
        ),
        r#"{ "$and": [{ "total": { "$gt": Double(1000.0) } }, { "customer.name": { "$eq": "Acme Ltd" } }, { "customer.address.city": { "$eq": "Santo Domingo" } }] }"#
    );
    assert_eq!(
        shown(
            invoice
                .customer()
                .nest(in_city("Santo Domingo"))
                .or(invoice.total().gt(1000.0_f64).unwrap())
        ),
        r#"{ "$or": [{ "customer.address.city": { "$eq": "Santo Domingo" } }, { "total": { "$gt": Double(1000.0) } }] }"#
    );
    let order = Order::MONGO_FIELDS;
    let by_code = Address::MONGO_FIELDS
        .postal_code()
        .eq("10101".to_owned())
        .unwrap();
    assert_eq!(
        shown(
            order
                .placed()
                .eq(true)
                .unwrap()
                .and(
                    order
                        .customer()
                        .nest(customer.open_invoices().lte(3_u32).unwrap())
                )
                .and(order.customer().nest(customer.address().nest(by_code)))
        ),
        r#"{ "$and": [{ "placed": { "$eq": true } }, { "customer.openInvoices": { "$lte": Int64(3) } }, { "customer.address.postalCode": { "$eq": "10101" } }] }"#
    );
}

#[test]
fn an_operation_takes_a_filter_and_an_update_of_any_module_over_its_rows() {
    let (invoice, customer) = (Invoice::MONGO_FIELDS, Customer::MONGO_FIELDS);
    assert_eq!(
        shown(taken_by_a_read(
            invoice.customer().nest(in_city("Santo Domingo"))
        )),
        r#"{ "customer.address.city": { "$eq": "Santo Domingo" } }"#
    );
    let moved = Address::MONGO_FIELDS.city().set("Moca".to_owned()).unwrap();
    let (filter, update) = taken_by_a_write(
        invoice
            .customer()
            .nest(customer.name().eq("Acme Ltd".to_owned()).unwrap()),
        invoice.customer().nest(customer.address().nest(moved)),
    );
    assert_eq!(
        shown(filter),
        r#"{ "customer.name": { "$eq": "Acme Ltd" } }"#
    );
    assert_eq!(
        shown(update),
        r#"{ "$set": { "customer.address.city": "Moca" } }"#
    );
}

#[test]
fn a_document_written_by_hand_joins_the_typed_paths_through_raw() {
    assert_eq!(
        shown(
            Invoice::MONGO_FIELDS
                .total()
                .gt(1000.0_f64)
                .unwrap()
                .and(invoice_schema::Filter::<Invoice>::raw(
                    doc! { "details.po": "7781" }
                ))
        ),
        r#"{ "$and": [{ "total": { "$gt": Double(1000.0) } }, { "details.po": "7781" }] }"#
    );
}

#[test]
fn the_updates_of_three_modules_merge_into_one() {
    let (invoice, customer) = (Invoice::MONGO_FIELDS, Customer::MONGO_FIELDS);
    let moved = Address::MONGO_FIELDS.city().set("Moca".to_owned()).unwrap();
    assert_eq!(
        shown(
            invoice
                .total()
                .set(0.0_f64)
                .unwrap()
                .and(
                    invoice
                        .customer()
                        .nest(customer.name().set("Acme Ltd".to_owned()).unwrap())
                )
                .and(invoice.customer().nest(customer.address().nest(moved)))
                .and(invoice_schema::Update::<Invoice>::raw(
                    doc! { "$currentDate": { "updatedAt": true } }
                ))
        ),
        r#"{ "$set": { "total": Double(0.0), "customer.name": "Acme Ltd", "customer.address.city": "Moca" }, "$currentDate": { "updatedAt": true } }"#
    );
}

/// A key is written under the one that holds it, and an empty one is left out with its dot: the
/// key of a flattened model on the holder's side, the key of a value written whole on the other.
#[test]
fn nest_joins_two_keys_by_a_dot_and_leaves_an_empty_one_out() {
    let invoice = Invoice::MONGO_FIELDS;
    assert_eq!(
        shown(invoice.customer().nest(in_city("Lima"))),
        r#"{ "customer.address.city": { "$eq": "Lima" } }"#
    );
    assert_eq!(
        shown(
            invoice.audit().nest(
                Audit::MONGO_FIELDS
                    .created_by()
                    .eq("eduardo".to_owned())
                    .unwrap()
            )
        ),
        r#"{ "createdBy": { "$eq": "eduardo" } }"#
    );
    assert_eq!(
        shown(
            invoice
                .status()
                .nest(InvoiceStatus::MONGO_FIELDS.eq(InvoiceStatus::Paid).unwrap())
        ),
        r#"{ "status": { "$eq": "paid" } }"#
    );
}

#[test]
fn a_filter_joined_and_negated_is_nested_key_by_key() {
    let customer = Customer::MONGO_FIELDS;
    let named = customer.name().eq("Acme Ltd".to_owned()).unwrap();
    let few_open = customer.open_invoices().lte(3_u32).unwrap();
    let elsewhere = in_city("Moca").negated();
    assert_eq!(
        shown(
            Invoice::MONGO_FIELDS
                .customer()
                .nest(named.and(few_open).or(elsewhere))
        ),
        r#"{ "$or": [{ "$and": [{ "customer.name": { "$eq": "Acme Ltd" } }, { "customer.openInvoices": { "$lte": Int64(3) } }] }, { "$nor": [{ "customer.address.city": { "$eq": "Moca" } }] }] }"#
    );
}

/// An operator a raw document writes at its top is no key: it is kept as it is written.
#[test]
fn an_operator_of_a_raw_filter_is_kept_as_it_is_written() {
    let raw = customer_schema::Filter::<Customer>::raw(doc! {
        "$expr": { "$gt": ["$openInvoices", 3_i64] },
        "name": "Acme Ltd",
    });
    assert_eq!(
        shown(Invoice::MONGO_FIELDS.customer().nest(raw)),
        r#"{ "$expr": { "$gt": ["$openInvoices", Int64(3)] }, "customer.name": "Acme Ltd" }"#
    );
}

#[test]
fn an_update_of_each_operator_is_nested_one_type_down() {
    let (invoice, customer) = (Invoice::MONGO_FIELDS, Customer::MONGO_FIELDS);
    assert_eq!(
        shown(
            invoice
                .customer()
                .nest(customer.name().set("John".to_owned()).unwrap())
        ),
        r#"{ "$set": { "customer.name": "John" } }"#
    );
    assert_eq!(
        shown(
            invoice
                .customer()
                .nest(customer.open_invoices().set_on_insert(0_u32).unwrap())
        ),
        r#"{ "$setOnInsert": { "customer.openInvoices": Int64(0) } }"#
    );
    let (consignment, hamper) = (Consignment::MONGO_FIELDS, Hamper::MONGO_FIELDS);
    assert_eq!(
        shown(
            consignment
                .hamper()
                .nest(hamper.tags().push("gift".to_owned()).unwrap())
        ),
        r#"{ "$push": { "hamper.tags": "gift" } }"#
    );
    assert_eq!(
        shown(
            consignment.hamper().nest(
                hamper
                    .items()
                    .pull(Treat::MONGO_FIELDS.weight().lt(1_u32).unwrap())
            )
        ),
        r#"{ "$pull": { "hamper.items": { "weight": { "$lt": Int64(1) } } } }"#
    );
    assert_eq!(
        shown(consignment.hamper().nest(hamper.note().unset())),
        r#"{ "$unset": { "hamper.note": "" } }"#
    );
    let both = hamper
        .tags()
        .push("gift".to_owned())
        .unwrap()
        .and(hamper.note().unset());
    assert_eq!(
        shown(consignment.hamper().nest(both)),
        r#"{ "$push": { "hamper.tags": "gift" }, "$unset": { "hamper.note": "" } }"#
    );
}

/// A filter and an update over the rows of what a variant holds, each under the enum's key.
#[test]
fn what_a_variant_holds_is_nested_under_the_key_of_its_enum() {
    let (invoice, payment) = (Invoice::MONGO_FIELDS, Payment::MONGO_FIELDS);
    assert_eq!(
        shown(
            invoice
                .payment()
                .nest(payment.card().last4().set("4242".to_owned()).unwrap())
        ),
        r#"{ "$set": { "payment.last4": "4242" } }"#
    );
}
