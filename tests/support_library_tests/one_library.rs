//! Three types of one crate, each re-exporting the crate's one library: what a callback and a
//! resolver work with is one type for the three, and one resolver serves the reads of all three.

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tixschema::model_schema;

/// What a read of `$stored` as `$type` hands its callback, each issue as `Debug` writes it, and
/// how many issues its refusal holds.
macro_rules! rejected {
    ($type:ident, $module:ident, $stored:expr) => {{
        let mut handed: Vec<String> = Vec::new();
        let read = $type::from_value_with($stored, |_raw, found| {
            handed = found.iter().map(|issue| format!("{issue:?}")).collect();
            $module::Verdict::Reject
        });
        (handed, read.map_err(|refused| refused.issues.len()))
    }};
}

#[model_schema(enable = ["decode_with"])]
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
struct InvoiceLookup {
    number: String,
}

#[model_schema(enable = ["decode_with"])]
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
struct CustomerLookup {
    name: String,
}

#[model_schema(enable = ["decode_with"])]
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
struct ParcelLookup {
    label: String,
}

/// Settles a text field an older writer stored as a number, wherever it sits, for the read of
/// any type of the crate. Its signature names the modules of three types: they hold one library.
fn number_as_text(
    raw: &mut Value,
    issue: &invoice_lookup_schema::Issue<Value>,
) -> parcel_lookup_schema::Resolution {
    let customer_lookup_schema::Issue::Invalid {
        path,
        expected: invoice_lookup_schema::Expected::String,
        found: Value::Number(number),
        reason: _reason,
    } = issue
    else {
        return customer_lookup_schema::Resolution::NotTouched;
    };
    if path.set_in_value(raw, Value::String(number.to_string())) {
        invoice_lookup_schema::Resolution::Settled
    } else {
        invoice_lookup_schema::Resolution::Rejected
    }
}

/// A path, as one module names it.
fn shown(path: &customer_lookup_schema::Path) -> String {
    path.to_string()
}

#[test]
fn each_module_names_the_one_library_of_the_crate() {
    let verdicts = [
        invoice_lookup_schema::Verdict::Reject,
        customer_lookup_schema::Verdict::Reject,
        parcel_lookup_schema::Verdict::Fixed,
    ];
    assert_eq!(verdicts.len(), 3);
    let path = parcel_lookup_schema::Path(vec![invoice_lookup_schema::Segment::Key(
        "number".to_owned(),
    )]);
    assert_eq!(shown(&path), "number");
}

#[test]
fn each_type_answers_its_callback_as_it_did_with_a_library_of_its_own() {
    assert_eq!(
        rejected!(
            InvoiceLookup,
            invoice_lookup_schema,
            json!({ "number": "INV-1" })
        ),
        (
            Vec::new(),
            Ok(InvoiceLookup {
                number: "INV-1".to_owned()
            })
        )
    );
    let (missing, unnamed) = rejected!(CustomerLookup, customer_lookup_schema, json!({}));
    assert_eq!(
        missing,
        [r#"Missing { path: Path([Key("name")]), expected: String }"#]
    );
    assert_eq!(unnamed, Err(1));
    let (invalid, unlabelled) = rejected!(
        ParcelLookup,
        parcel_lookup_schema,
        json!({ "label": 7_i32 })
    );
    assert_eq!(
        invalid,
        [
            r#"Invalid { path: Path([Key("label")]), expected: String, found: Number(7), reason: "invalid type: integer `7`, expected a string" }"#
        ]
    );
    assert_eq!(unlabelled, Err(1));
}

#[test]
fn one_resolver_serves_the_reads_of_every_type_of_the_crate() {
    let settled: &[invoice_lookup_schema::Resolver<'_, Value, Value>] = &[&number_as_text];
    assert_eq!(
        InvoiceLookup::from_value_piped(json!({ "number": 42_i32 }), settled),
        Ok(InvoiceLookup {
            number: "42".to_owned()
        })
    );
    assert_eq!(
        ParcelLookup::from_value_piped(json!({ "label": 7_i32 }), settled),
        Ok(ParcelLookup {
            label: "7".to_owned()
        })
    );
    assert_eq!(
        CustomerLookup::from_value_piped(json!({ "name": "Ada" }), settled),
        Ok(CustomerLookup {
            name: "Ada".to_owned()
        })
    );
}
