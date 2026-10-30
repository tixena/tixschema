//! One repair several types of a crate need, written once: the types share their crate's library,
//! so one function settles the issue for each of them, and their reads take one list of resolvers.

use bson::{Bson, Document, doc};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tixschema::model_schema;

use invoice_schema::{DecodeWith, Expected, Issue, Resolution, Resolver, Stored as _};

/// The resolvers the stored rows of this crate's types are read through, in order.
const STORED: &[Resolver<'static, Document, Bson>] = &[&whole_number_as_text];

#[model_schema(enable = ["decode_with", "mongodb"])]
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Invoice {
    pub number: String,
    pub open_lines: u32,
}

#[model_schema(enable = ["decode_with", "mongodb"])]
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Order {
    pub lines: u32,
    pub number: String,
}

#[model_schema(enable = ["decode_with", "mongodb"], default_types(T = u32))]
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Holder<T> {
    pub held: T,
}

fn read_stored<Row>(document: Document) -> Row
where
    Row: DecodeWith,
{
    Row::from_bson_piped(document, STORED).unwrap()
}

fn read_json<Row>(value: Value) -> Row
where
    Row: DecodeWith,
{
    Row::from_value_piped(value, &[]).unwrap()
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
        .is_ok_and(|number| number >= 0_i64 && raw.set_at(path, Bson::Int64(number)));
    if settled {
        Resolution::Settled
    } else {
        Resolution::Rejected
    }
}

/// An order number an older writer stored as a bare integer, written as the number it stands for.
/// Only `Order` needs it.
fn order_number_as_integer(raw: &mut Document, issue: &Issue<Bson>) -> Resolution {
    let Issue::Invalid {
        path,
        expected: Expected::String,
        found: Bson::Int32(number),
        reason: _reason,
    } = issue
    else {
        return Resolution::NotTouched;
    };
    if raw.set_at(path, Bson::String(format!("ORD-{number:04}"))) {
        Resolution::Settled
    } else {
        Resolution::Rejected
    }
}

#[test]
fn the_shared_resolver_settles_the_same_repair_for_every_type() {
    let invoice = Invoice::from_bson_piped(doc! { "number": "INV-0042", "openLines": "3" }, STORED);
    let order = Order::from_bson_piped(doc! { "lines": "3", "number": "ORD-0007" }, STORED);
    assert_eq!(
        invoice,
        Ok(Invoice {
            number: "INV-0042".to_owned(),
            open_lines: 3,
        })
    );
    assert_eq!(
        order,
        Ok(Order {
            lines: 3,
            number: "ORD-0007".to_owned(),
        })
    );
}

#[test]
fn the_shared_resolver_refuses_alike_for_every_type_at_each_types_own_path() {
    let invoice =
        Invoice::from_bson_piped(doc! { "number": "INV-0042", "openLines": "-3" }, STORED)
            .unwrap_err();
    let order =
        Order::from_bson_piped(doc! { "lines": "-3", "number": "ORD-0007" }, STORED).unwrap_err();
    assert_eq!(invoice.issues.len(), 1);
    assert_eq!(order.issues.len(), 1);
    let (invoice_shown, order_shown) = (invoice.to_string(), order.to_string());
    assert!(
        invoice_shown.starts_with("openLines: invalid: expected U32, found String(\"-3\"): "),
        "got: {invoice_shown}"
    );
    assert!(
        order_shown.starts_with("lines: invalid: expected U32, found String(\"-3\"): "),
        "got: {order_shown}"
    );
}

#[test]
fn an_issue_no_shared_resolver_knows_refuses_the_read_for_every_type() {
    let invoice = Invoice::from_bson_piped(
        doc! { "legacy": true, "number": "INV-0042", "openLines": 3_i64 },
        STORED,
    )
    .unwrap_err();
    let order = Order::from_bson_piped(
        doc! { "legacy": true, "lines": 3_i64, "number": "ORD-0007" },
        STORED,
    )
    .unwrap_err();
    assert_eq!(
        invoice.issues,
        [invoice_schema::Issue::Unknown {
            path: invoice_schema::Path(vec![invoice_schema::Segment::Key("legacy".to_owned())]),
            found: Bson::Boolean(true),
        }]
    );
    assert_eq!(
        order.issues,
        [order_schema::Issue::Unknown {
            path: order_schema::Path(vec![order_schema::Segment::Key("legacy".to_owned())]),
            found: Bson::Boolean(true),
        }]
    );
}

#[test]
fn a_type_with_a_repair_of_its_own_reads_through_a_list_of_its_own() {
    let stored = || doc! { "lines": "2", "number": 7_i32 };
    let own = Order::from_bson_piped(stored(), &[&whole_number_as_text, &order_number_as_integer]);
    assert_eq!(
        own,
        Ok(Order {
            lines: 2,
            number: "ORD-0007".to_owned(),
        })
    );
    let shown = Order::from_bson_piped(stored(), STORED)
        .unwrap_err()
        .to_string();
    assert!(
        shown.starts_with("number: invalid: expected String, found Int32(7): "),
        "got: {shown}"
    );
}

#[test]
fn one_generic_function_reads_two_types_of_a_crate_through_the_same_resolvers() {
    let invoice: Invoice = read_stored(doc! { "number": "INV-0042", "openLines": "3" });
    let order: Order = read_stored(doc! { "lines": "3", "number": "ORD-0007" });
    assert_eq!(invoice.open_lines, 3);
    assert_eq!(order.lines, 3);
}

#[test]
fn one_generic_function_reads_two_types_of_a_crate_from_json() {
    let invoice: Invoice = read_json(json!({ "number": "INV-0042", "openLines": 3_i32 }));
    let order: Order = read_json(json!({ "lines": 3_i32, "number": "ORD-0007" }));
    assert_eq!(invoice.open_lines, 3);
    assert_eq!(order.lines, 3);
}

#[test]
fn a_generic_type_is_read_through_the_same_bound() {
    let held: Holder<u32> = read_stored(doc! { "held": 4_i32 });
    let from_json: Holder<u32> = read_json(json!({ "held": 4_i32 }));
    assert_eq!(held, Holder { held: 4 });
    assert_eq!(from_json, held);
}
