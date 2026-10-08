//! A nested model's paths, built by its own function under the key the outer type holds it at,
//! and its filters joined with the outer type's: three types deep, each one's module holding its
//! own `Filter` and `Update`, and each one's expansion writing its own keys alone.

use core::marker::PhantomData;

use bson::{Document, doc};

use super::{Address, Customer, Invoice, Order, invoice_schema, shown};

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

#[test]
fn a_nested_path_writes_every_key_that_leads_to_it() {
    let invoice = Invoice::MONGO_FIELDS;
    assert_eq!(
        shown(
            invoice
                .customer
                .address
                .city
                .eq("Santo Domingo".to_owned())
                .unwrap()
        ),
        r#"{ "customer.address.city": { "$eq": "Santo Domingo" } }"#
    );
    assert_eq!(
        shown(
            invoice
                .customer
                .address
                .postal_code
                .eq("10101".to_owned())
                .unwrap()
        ),
        r#"{ "customer.address.postalCode": { "$eq": "10101" } }"#
    );
    assert_eq!(
        shown(invoice.customer.name.eq("Acme Ltd".to_owned()).unwrap()),
        r#"{ "customer.name": { "$eq": "Acme Ltd" } }"#
    );
    assert_eq!(
        shown(invoice.total.gt(1.0_f64).unwrap()),
        r#"{ "total": { "$gt": Double(1.0) } }"#
    );
}

/// The same nested model under a second row type, and as the row itself: its paths start at
/// whatever leads to it.
#[test]
fn a_models_paths_start_at_whatever_holds_it() {
    assert_eq!(
        shown(
            Order::MONGO_FIELDS
                .customer
                .address
                .city
                .eq("Santiago".to_owned())
                .unwrap()
        ),
        r#"{ "customer.address.city": { "$eq": "Santiago" } }"#
    );
    assert_eq!(
        shown(
            Customer::MONGO_FIELDS
                .address
                .city
                .eq("Santiago".to_owned())
                .unwrap()
        ),
        r#"{ "address.city": { "$eq": "Santiago" } }"#
    );
    assert_eq!(
        shown(
            Address::MONGO_FIELDS
                .city
                .eq("Santiago".to_owned())
                .unwrap()
        ),
        r#"{ "city": { "$eq": "Santiago" } }"#
    );
}

/// `invoice_schema`'s filter takes `customer_schema`'s and `address_schema`'s, and the innermost
/// module's filter takes the outermost's just as well.
#[test]
fn the_filters_of_three_modules_join_into_one() {
    let invoice = Invoice::MONGO_FIELDS;
    assert_eq!(
        shown(
            invoice
                .total
                .gt(1000.0_f64)
                .unwrap()
                .and(invoice.customer.name.eq("Acme Ltd".to_owned()).unwrap())
                .and(
                    invoice
                        .customer
                        .address
                        .city
                        .eq("Santo Domingo".to_owned())
                        .unwrap()
                )
        ),
        r#"{ "$and": [{ "total": { "$gt": Double(1000.0) } }, { "customer.name": { "$eq": "Acme Ltd" } }, { "customer.address.city": { "$eq": "Santo Domingo" } }] }"#
    );
    assert_eq!(
        shown(
            invoice
                .customer
                .address
                .city
                .eq("Santo Domingo".to_owned())
                .unwrap()
                .or(invoice.total.gt(1000.0_f64).unwrap())
        ),
        r#"{ "$or": [{ "customer.address.city": { "$eq": "Santo Domingo" } }, { "total": { "$gt": Double(1000.0) } }] }"#
    );
    let order = Order::MONGO_FIELDS;
    assert_eq!(
        shown(
            order
                .placed
                .eq(true)
                .unwrap()
                .and(order.customer.open_invoices.lte(3_u32).unwrap())
                .and(
                    order
                        .customer
                        .address
                        .postal_code
                        .eq("10101".to_owned())
                        .unwrap()
                )
        ),
        r#"{ "$and": [{ "placed": { "$eq": true } }, { "customer.openInvoices": { "$lte": Int64(3) } }, { "customer.address.postalCode": { "$eq": "10101" } }] }"#
    );
}

/// An operation of `Invoice` takes a filter and an update over its rows from whichever module
/// wrote them.
#[test]
fn an_operation_takes_a_filter_and_an_update_of_any_module_over_its_rows() {
    let invoice = Invoice::MONGO_FIELDS;
    assert_eq!(
        shown(taken_by_a_read(
            invoice
                .customer
                .address
                .city
                .eq("Santo Domingo".to_owned())
                .unwrap()
        )),
        r#"{ "customer.address.city": { "$eq": "Santo Domingo" } }"#
    );
    let (filter, update) = taken_by_a_write(
        invoice.customer.name.eq("Acme Ltd".to_owned()).unwrap(),
        invoice
            .customer
            .address
            .city
            .set("Moca".to_owned())
            .unwrap(),
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

/// A document written by hand joins the typed paths through `Filter::raw`, which gives it the
/// row type of the filter it joins.
#[test]
fn a_document_written_by_hand_joins_the_typed_paths_through_raw() {
    assert_eq!(
        shown(
            Invoice::MONGO_FIELDS
                .total
                .gt(1000.0_f64)
                .unwrap()
                .and(invoice_schema::Filter::<Invoice>::raw(
                    doc! { "details.po": "7781" }
                ))
        ),
        r#"{ "$and": [{ "total": { "$gt": Double(1000.0) } }, { "details.po": "7781" }] }"#
    );
}

/// An update of one module merges an update of another under the operator both write, and a raw
/// one beside the typed ones.
#[test]
fn the_updates_of_three_modules_merge_into_one() {
    let invoice = Invoice::MONGO_FIELDS;
    assert_eq!(
        shown(
            invoice
                .total
                .set(0.0_f64)
                .unwrap()
                .and(invoice.customer.name.set("Acme Ltd".to_owned()).unwrap())
                .and(
                    invoice
                        .customer
                        .address
                        .city
                        .set("Moca".to_owned())
                        .unwrap()
                )
                .and(invoice_schema::Update::<Invoice>::raw(
                    doc! { "$currentDate": { "updatedAt": true } }
                ))
        ),
        r#"{ "$set": { "total": Double(0.0), "customer.name": "Acme Ltd", "customer.address.city": "Moca" }, "$currentDate": { "updatedAt": true } }"#
    );
}
