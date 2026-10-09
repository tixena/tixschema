//! One repair several `decode_with` types need, written once by a `macro_rules!` into each type's
//! own `impl`, and read through each type's own list of resolvers.

use bson::{Bson, Document, doc};
use serde::{Deserialize, Serialize};
use tixschema::model_schema;

/// Writes into each type the resolvers its stored rows are read through, in order, as `STORED`.
/// `whole_number_as_text` reads a whole number an older writer stored as text. A macro because
/// each type's resolvers name that type's own schema module, which no trait unites.
macro_rules! shared_resolvers {
    ($($row:ty => $schema:ident),* $(,)?) => {$(
        impl $row {
            const STORED: &'static [$schema::Resolver<'static, Document, Bson>] =
                &[&Self::whole_number_as_text];

            fn whole_number_as_text(
                raw: &mut Document,
                issue: &$schema::Issue<Bson>,
            ) -> $schema::Resolution {
                let $schema::Issue::Invalid {
                    path,
                    expected: $schema::Expected::U32,
                    found: Bson::String(text),
                    reason: _reason,
                } = issue
                else {
                    return $schema::Resolution::NotTouched;
                };
                let settled = text.trim().parse::<i64>().is_ok_and(|number| {
                    number >= 0_i64 && path.set_in_document(raw, Bson::Int64(number))
                });
                if settled {
                    $schema::Resolution::Settled
                } else {
                    $schema::Resolution::Rejected
                }
            }
        }
    )*};
}

#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
struct Invoice {
    number: String,
    open_lines: u32,
}

#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct Order {
    lines: u32,
    number: String,
}

shared_resolvers! {
    Invoice => invoice_schema,
    Order => order_schema,
}

/// An order number an older writer stored as a bare integer, written as the number it stands for.
/// Only `Order` needs it.
fn order_number_as_integer(
    raw: &mut Document,
    issue: &order_schema::Issue<Bson>,
) -> order_schema::Resolution {
    let order_schema::Issue::Invalid {
        path,
        expected: order_schema::Expected::String,
        found: Bson::Int32(number),
        reason: _reason,
    } = issue
    else {
        return order_schema::Resolution::NotTouched;
    };
    if path.set_in_document(raw, Bson::String(format!("ORD-{number:04}"))) {
        order_schema::Resolution::Settled
    } else {
        order_schema::Resolution::Rejected
    }
}

#[test]
fn the_shared_resolver_settles_the_same_repair_for_every_type() {
    let invoice = Invoice::from_bson_piped(
        doc! { "number": "INV-0042", "openLines": "3" },
        Invoice::STORED,
    );
    let order = Order::from_bson_piped(doc! { "lines": "3", "number": "ORD-0007" }, Order::STORED);
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
    let invoice = Invoice::from_bson_piped(
        doc! { "number": "INV-0042", "openLines": "-3" },
        Invoice::STORED,
    )
    .unwrap_err();
    let order = Order::from_bson_piped(doc! { "lines": "-3", "number": "ORD-0007" }, Order::STORED)
        .unwrap_err();
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
        Invoice::STORED,
    )
    .unwrap_err();
    let order = Order::from_bson_piped(
        doc! { "legacy": true, "lines": 3_i64, "number": "ORD-0007" },
        Order::STORED,
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
    let own = Order::from_bson_piped(
        stored(),
        &[&Order::whole_number_as_text, &order_number_as_integer],
    );
    assert_eq!(
        own,
        Ok(Order {
            lines: 2,
            number: "ORD-0007".to_owned(),
        })
    );
    let shown = Order::from_bson_piped(stored(), Order::STORED)
        .unwrap_err()
        .to_string();
    assert!(
        shown.starts_with("number: invalid: expected String, found Int32(7): "),
        "got: {shown}"
    );
}
