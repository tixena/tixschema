//! One resolver written over `Stored` serves a list of JSON resolvers and a list of BSON
//! resolvers alike, and removes a key in both.

extern crate bson2 as bson;

tixschema::decode_with_foundation!();

#[cfg(test)]
#[cfg(all(feature = "bson", feature = "serde"))]
mod tests {
    use bson::{Bson, Document, doc};
    use serde::{Deserialize, Serialize};
    use serde_json::{Value, json};
    use tixschema::model_schema;

    use invoice_schema::{DecodeWith as _, Expected, Issue, Resolution, Resolver, Stored};

    const FROM_JSON: &[Resolver<'static, Value, Value>] =
        &[&count_as_text::<Value>, &without_unknown_keys::<Value>];
    const FROM_BSON: &[Resolver<'static, Document, Bson>] = &[
        &count_as_text::<Document>,
        &without_unknown_keys::<Document>,
    ];

    #[model_schema(enable = ["decode_with"])]
    #[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct Invoice {
        pub number: String,
        pub open_lines: u32,
    }

    #[model_schema(enable = ["decode_with"])]
    #[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Order {
        pub lines: u32,
        pub number: String,
    }

    fn count_as_text<D>(raw: &mut D, issue: &Issue<D::Value>) -> Resolution
    where
        D: Stored,
    {
        let Issue::Invalid {
            path,
            expected: Expected::U32,
            found,
            reason: _reason,
        } = issue
        else {
            return Resolution::NotTouched;
        };
        let Some(text) = D::text(found) else {
            return Resolution::NotTouched;
        };
        match text.trim().parse::<i64>() {
            Ok(number) if number >= 0_i64 && raw.set_at(path, D::from_whole(number)) => {
                Resolution::Settled
            }
            _ => Resolution::Rejected,
        }
    }

    fn without_unknown_keys<D>(raw: &mut D, issue: &Issue<D::Value>) -> Resolution
    where
        D: Stored,
    {
        if let Issue::Unknown {
            path,
            found: _found,
        } = issue
            && raw.remove_at(path)
        {
            Resolution::Settled
        } else {
            Resolution::NotTouched
        }
    }

    fn assert_refused_at<V>(issues: &[Issue<V>], at: &str) {
        assert_eq!(issues.len(), 1);
        assert!(
            matches!(&issues[0], Issue::Invalid { path, expected: Expected::U32, found: _found, reason: _reason }
                if path.to_string() == at)
        );
    }

    fn invoice() -> Invoice {
        Invoice {
            number: "INV-0042".to_owned(),
            open_lines: 3,
        }
    }

    fn order() -> Order {
        Order {
            lines: 3,
            number: "ORD-0007".to_owned(),
        }
    }

    #[test]
    fn one_resolver_settles_a_count_stored_as_text_in_json_and_in_bson() {
        let json_invoice =
            Invoice::from_value_piped(json!({ "number": "INV-0042", "openLines": "3" }), FROM_JSON);
        let bson_invoice =
            Invoice::from_bson_piped(doc! { "number": "INV-0042", "openLines": "3" }, FROM_BSON);
        let json_order =
            Order::from_value_piped(json!({ "lines": " 3 ", "number": "ORD-0007" }), FROM_JSON);
        let bson_order =
            Order::from_bson_piped(doc! { "lines": " 3 ", "number": "ORD-0007" }, FROM_BSON);
        assert_eq!(json_invoice, Ok(invoice()));
        assert_eq!(bson_invoice, Ok(invoice()));
        assert_eq!(json_order, Ok(order()));
        assert_eq!(bson_order, Ok(order()));
    }

    #[test]
    fn one_resolver_refuses_alike_in_json_and_in_bson_for_each_type() {
        let json_invoice = Invoice::from_value_piped(
            json!({ "number": "INV-0042", "openLines": "-3" }),
            FROM_JSON,
        )
        .unwrap_err();
        let bson_invoice =
            Invoice::from_bson_piped(doc! { "number": "INV-0042", "openLines": "-3" }, FROM_BSON)
                .unwrap_err();
        let json_order =
            Order::from_value_piped(json!({ "lines": "many", "number": "ORD-0007" }), FROM_JSON)
                .unwrap_err();
        let bson_order =
            Order::from_bson_piped(doc! { "lines": "many", "number": "ORD-0007" }, FROM_BSON)
                .unwrap_err();
        assert_refused_at(&json_invoice.issues, "openLines");
        assert_refused_at(&bson_invoice.issues, "openLines");
        assert_refused_at(&json_order.issues, "lines");
        assert_refused_at(&bson_order.issues, "lines");
    }

    #[test]
    fn one_resolver_removes_a_key_the_type_does_not_declare_in_json_and_in_bson() {
        let json_invoice = Invoice::from_value_piped(
            json!({ "number": "INV-0042", "openLines": 3_i32, "legacy": true }),
            FROM_JSON,
        );
        let bson_invoice = Invoice::from_bson_piped(
            doc! { "number": "INV-0042", "openLines": 3_i32, "legacy": true },
            FROM_BSON,
        );
        let json_order = Order::from_value_piped(
            json!({ "lines": 3_i32, "number": "ORD-0007", "legacy": true }),
            FROM_JSON,
        );
        let bson_order = Order::from_bson_piped(
            doc! { "lines": 3_i32, "number": "ORD-0007", "legacy": true },
            FROM_BSON,
        );
        assert_eq!(json_invoice, Ok(invoice()));
        assert_eq!(bson_invoice, Ok(invoice()));
        assert_eq!(json_order, Ok(order()));
        assert_eq!(bson_order, Ok(order()));
    }

    #[test]
    fn at_reads_what_set_wrote_in_json_and_in_bson() {
        let mut json_row = json!({ "lines": [{ "count": 1_i32 }] });
        let mut bson_row = doc! { "lines": [{ "count": 1_i32 }] };
        let path = invoice_schema::Path(vec![
            invoice_schema::Segment::Key("lines".to_owned()),
            invoice_schema::Segment::Index(0),
            invoice_schema::Segment::Key("count".to_owned()),
        ]);
        assert!(json_row.set_at(&path, Value::from_whole(7_i64)));
        assert!(bson_row.set_at(&path, Document::from_whole(7_i64)));
        assert_eq!(json_row.at(&path).and_then(Value::whole), Some(7_i64));
        assert_eq!(
            bson_row.at(&path).and_then(<Document as Stored>::whole),
            Some(7_i64)
        );
        assert!(json_row.remove_at(&path));
        assert!(bson_row.remove_at(&path));
        assert_eq!(json_row.at(&path), None);
        assert_eq!(bson_row.at(&path), None);
    }
}
