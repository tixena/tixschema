//! Every typed path is a key of what serde stores: each path generated for a row is looked up in
//! the document `bson::Serializer::new()` writes for one, the way MongoDB walks a dotted key.

use std::collections::HashMap;

use bson::oid::ObjectId;
use bson::{Bson, Document};
use serde::Serialize as _;

use super::shapes::{Audit, Delivery, Payment, Rebate, Reference, Wrapper};
use super::{
    Address, Customer, Invoice, InvoiceId, InvoiceStatus, LineItem, Point, shown, shown_value,
};

/// What `key` leads to in `value`: a key of a document, a position in an array, or the key in
/// each element of an array.
fn found(value: &Bson, key: &[&str]) -> Vec<Bson> {
    let Some((first, rest)) = key.split_first() else {
        return vec![value.clone()];
    };
    if let Bson::Document(document) = value {
        document
            .get(*first)
            .map_or_else(Vec::new, |held| found(held, rest))
    } else if let Bson::Array(items) = value {
        first.parse::<usize>().map_or_else(
            |_| items.iter().flat_map(|item| found(item, key)).collect(),
            |position| {
                items
                    .get(position)
                    .map_or_else(Vec::new, |held| found(held, rest))
            },
        )
    } else {
        Vec::new()
    }
}

/// The key a filter written by one path is over, and what that key leads to in `stored`.
pub fn looked_up<F>(stored: &Document, filter: F) -> (String, String)
where
    F: Into<Document>,
{
    let written: Document = filter.into();
    let key = written.keys().next().unwrap().clone();
    let segments: Vec<&str> = key.split('.').collect();
    let held: Vec<String> = found(&Bson::Document(stored.clone()), &segments)
        .iter()
        .map(shown_value)
        .collect();
    (key, held.join(" | "))
}

fn address() -> Address {
    Address {
        city: "Santo Domingo".to_owned(),
        postal_code: "10101".to_owned(),
    }
}

fn invoice() -> Invoice {
    Invoice {
        audit: Audit {
            created_by: "eduardo".to_owned(),
            revision: 4_u32,
        },
        billing: Some(address()),
        customer: Customer {
            address: address(),
            name: "Acme Ltd".to_owned(),
            open_invoices: 3_u32,
        },
        delivery: Delivery::Courier {
            carrier: "dhl".to_owned(),
            tracking: "JD0146".to_owned(),
        },
        details: HashMap::from([("po".to_owned(), "7781".to_owned())]),
        discount: Rebate::Coupon {
            code: "FALL".to_owned(),
        },
        id: InvoiceId(ObjectId::parse_str("507f1f77bcf86cd799439011").unwrap()),
        items: vec![
            LineItem {
                article: "A-1".to_owned(),
                price: 9.5_f64,
                quantity: 2_u32,
            },
            LineItem {
                article: "B-7".to_owned(),
                price: 120.0_f64,
                quantity: 1_u32,
            },
        ],
        number: "INV-0042".to_owned(),
        origin: Point(18.47_f64, -69.9_f64),
        paid_at: Some("2026-10-01".to_owned()),
        payment: Payment::Card {
            exp_month: 7_u32,
            last4: "4242".to_owned(),
        },
        reference: Reference::Detailed {
            code: "R-9".to_owned(),
            issuer: "bank".to_owned(),
        },
        scores: vec![72_u32, 84_u32],
        shipping: Wrapper {
            history: vec![address()],
            inner: address(),
            label: "warehouse".to_owned(),
        },
        status: InvoiceStatus::PastDue,
        tags: vec!["export".to_owned(), "priority".to_owned()],
        total: 1250.5_f64,
    }
}

/// The paths of the row's own values, of the models it holds bare, and of the list of plain
/// values.
fn plain_paths(stored: &Document) -> Vec<(String, String)> {
    let paths = Invoice::MONGO_FIELDS;
    let row = invoice();
    vec![
        looked_up(stored, paths.id.eq(row.id).unwrap()),
        looked_up(stored, paths.number.eq(row.number).unwrap()),
        looked_up(stored, paths.total.eq(row.total).unwrap()),
        looked_up(stored, paths.status.eq(row.status).unwrap()),
        looked_up(stored, paths.paid_at.exists(true)),
        looked_up(stored, paths.tags.size(2)),
        looked_up(stored, paths.customer.name.eq(String::new()).unwrap()),
        looked_up(stored, paths.customer.open_invoices.eq(0_u32).unwrap()),
        looked_up(
            stored,
            paths.customer.address.city.eq(String::new()).unwrap(),
        ),
        looked_up(
            stored,
            paths
                .customer
                .address
                .postal_code
                .eq(String::new())
                .unwrap(),
        ),
        looked_up(stored, paths.scores.size(2)),
        looked_up(stored, paths.origin.0.eq(0.0_f64).unwrap()),
        looked_up(stored, paths.origin.1.eq(0.0_f64).unwrap()),
    ]
}

/// The paths below a list of models, a model a row may leave out, a flattened model and a generic
/// one.
fn nested_paths(stored: &Document) -> Vec<(String, String)> {
    let paths = Invoice::MONGO_FIELDS;
    vec![
        looked_up(stored, paths.items.article.eq(String::new()).unwrap()),
        looked_up(stored, paths.items.price.eq(0.0_f64).unwrap()),
        looked_up(stored, paths.items.quantity.eq(0_u32).unwrap()),
        looked_up(stored, paths.billing.city.eq(String::new()).unwrap()),
        looked_up(stored, paths.billing.postal_code.eq(String::new()).unwrap()),
        looked_up(stored, paths.audit.created_by.eq(String::new()).unwrap()),
        looked_up(stored, paths.audit.revision.eq(0_u32).unwrap()),
        looked_up(stored, paths.shipping.inner.eq(address()).unwrap()),
        looked_up(stored, paths.shipping.label.eq(String::new()).unwrap()),
        looked_up(stored, paths.shipping.history.size(1)),
    ]
}

/// The paths of an enum in each form serde writes one, for the variant the stored row holds.
fn enum_paths(stored: &Document) -> Vec<(String, String)> {
    let paths = Invoice::MONGO_FIELDS;
    vec![
        looked_up(stored, paths.payment.is_card()),
        looked_up(stored, paths.payment.card.last4.eq(String::new()).unwrap()),
        looked_up(stored, paths.payment.card.exp_month.eq(0_u32).unwrap()),
        looked_up(stored, paths.delivery.is_courier()),
        looked_up(
            stored,
            paths.delivery.courier.carrier.eq(String::new()).unwrap(),
        ),
        looked_up(
            stored,
            paths.delivery.courier.tracking.eq(String::new()).unwrap(),
        ),
        looked_up(stored, paths.discount.is_coupon()),
        looked_up(
            stored,
            paths.discount.coupon.code.eq(String::new()).unwrap(),
        ),
        looked_up(
            stored,
            paths.reference.detailed.code.eq(String::new()).unwrap(),
        ),
        looked_up(
            stored,
            paths.reference.detailed.issuer.eq(String::new()).unwrap(),
        ),
    ]
}

#[test]
fn every_typed_path_is_found_in_what_serde_stores_for_the_row() {
    let row = invoice().serialize(bson::Serializer::new()).unwrap();
    let stored = row.as_document().unwrap();
    let mut held = plain_paths(stored);
    held.extend(nested_paths(stored));
    held.extend(enum_paths(stored));
    let expected = [
        ("_id", "ObjectId(507f1f77bcf86cd799439011)"),
        ("number", r#""INV-0042""#),
        ("total", "Double(1250.5)"),
        ("status", r#""past-due""#),
        ("paidAt", r#""2026-10-01""#),
        ("tags", r#"["export", "priority"]"#),
        ("customer.name", r#""Acme Ltd""#),
        ("customer.openInvoices", "Int64(3)"),
        ("customer.address.city", r#""Santo Domingo""#),
        ("customer.address.postalCode", r#""10101""#),
        ("scores", "[Int64(72), Int64(84)]"),
        ("origin.0", "Double(18.47)"),
        ("origin.1", "Double(-69.9)"),
        ("items.sku", r#""A-1" | "B-7""#),
        ("items.price", "Double(9.5) | Double(120.0)"),
        ("items.quantity", "Int64(2) | Int64(1)"),
        ("billing.city", r#""Santo Domingo""#),
        ("billing.postalCode", r#""10101""#),
        ("createdBy", r#""eduardo""#),
        ("revision", "Int64(4)"),
        (
            "shipping.inner",
            r#"{ "city": "Santo Domingo", "postalCode": "10101" }"#,
        ),
        ("shipping.label", r#""warehouse""#),
        (
            "shipping.history",
            r#"[{ "city": "Santo Domingo", "postalCode": "10101" }]"#,
        ),
        ("payment.kind", r#""card""#),
        ("payment.last4", r#""4242""#),
        ("payment.expMonth", "Int64(7)"),
        ("delivery.t", r#""courier""#),
        ("delivery.c.carrier", r#""dhl""#),
        ("delivery.c.tracking", r#""JD0146""#),
        ("discount.coupon", r#"{ "code": "FALL" }"#),
        ("discount.coupon.code", r#""FALL""#),
        ("reference.code", r#""R-9""#),
        ("reference.issuer", r#""bank""#),
    ]
    .map(|(key, value)| (key.to_owned(), value.to_owned()));
    assert_eq!(held, expected, "stored: {}", shown(stored.clone()));
}
