//! A type that names a `preprocess` is read by the recovering read as plain serde reads it: the
//! functions of a type are called once on the value as the read takes it, so the callback and
//! the resolvers are handed the corrected value, and a field's are called wherever the field is
//! read.

use core::cell::Cell;
use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tixschema::model_schema;

use super::lines;

thread_local! {
    /// How often the function of a counted parcel's label was called on this thread.
    static LABEL_CALLS: Cell<u32> = const { Cell::new(0) };
    /// How often the function of a counted parcel was called on this thread.
    static PARCEL_CALLS: Cell<u32> = const { Cell::new(0) };
}

/// A parcel whose functions count their calls.
#[model_schema(decode_with, preprocess = ["countedParcel"])]
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
struct CountedParcel {
    #[model_schema_prop(preprocess = ["countedLabel"])]
    label: String,
}

/// A parcel flattened into the object of its holder.
#[model_schema(decode_with)]
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
struct StoredCrated {
    dock: String,
    #[serde(flatten)]
    parcel: StoredParcel,
}

/// The type that holds the parcel under a key.
#[model_schema(decode_with)]
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
struct StoredManifest {
    parcel: StoredParcel,
}

/// The parcel of the documents, read by the recovering read.
#[model_schema(decode_with, preprocess = ["withoutFragile"])]
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
struct StoredParcel {
    fragile_item_codes: Vec<String>,
    item_code_list: Vec<String>,
    #[model_schema_prop(minLength = 1, preprocess = ["trimmed"])]
    label: String,
}

/// An enum whose function takes its value out of an envelope.
#[model_schema(decode_with, preprocess = ["unwrapped"])]
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(tag = "kind")]
enum StoredShipment {
    Air { flight: String },
    Ground,
}

fn counted_label(payload: Value) -> Value {
    LABEL_CALLS.set(LABEL_CALLS.get() + 1);
    payload
}

fn counted_parcel(payload: Value) -> Value {
    PARCEL_CALLS.set(PARCEL_CALLS.get() + 1);
    payload
}

fn trimmed(payload: Value) -> Value {
    if let Value::String(text) = payload {
        Value::String(text.trim().to_owned())
    } else {
        payload
    }
}

fn unwrapped(payload: Value) -> Value {
    if let Value::Object(mut entries) = payload {
        entries
            .remove("payload")
            .unwrap_or_else(|| Value::Object(entries))
    } else {
        payload
    }
}

fn without_fragile(mut payload: Value) -> Value {
    if let Some(object) = payload.as_object_mut() {
        object
            .entry("fragileItemCodes")
            .or_insert_with(|| Value::Array(Vec::new()));
    }
    payload
}

fn parcel(label: &str, fragile: &[&str]) -> StoredParcel {
    StoredParcel {
        fragile_item_codes: fragile.iter().map(|&code| code.to_owned()).collect(),
        item_code_list: vec!["x".to_owned()],
        label: label.to_owned(),
    }
}

/// Row 1 of the sample: a label with spaces at its ends.
fn row_1() -> Value {
    json!({ "label": "  A  ", "itemCodeList": ["x"], "fragileItemCodes": ["x"] })
}

/// Row 3 of the sample: a payload that holds no `fragileItemCodes`.
fn row_3() -> Value {
    json!({ "label": "A", "itemCodeList": ["x"] })
}

/// Reads `stored` with a callback that rejects, and answers the read with how often the callback
/// ran.
fn rejected(stored: Value) -> (Result<StoredParcel, Vec<String>>, u32) {
    let mut calls = 0_u32;
    let read = StoredParcel::from_value_with(stored, |_, _| {
        calls += 1;
        stored_parcel_schema::Verdict::Reject
    });
    (read.map_err(|refused| lines(&refused)), calls)
}

#[test]
fn the_callback_read_and_the_piped_read_call_each_function() {
    assert_eq!(rejected(row_1()), (Ok(parcel("A", &["x"])), 0));
    assert_eq!(
        StoredParcel::from_value_piped(row_1(), &[]).map_err(|refused| lines(&refused)),
        Ok(parcel("A", &["x"]))
    );
}

/// The function of the type gave the payload the key it left out, so nothing is missing from it.
#[test]
fn an_issue_the_function_of_a_type_fixed_is_not_listed() {
    assert_eq!(rejected(row_3()), (Ok(parcel("A", &[])), 0));
    assert_eq!(
        StoredParcel::from_value_piped(row_3(), &[]).map_err(|refused| lines(&refused)),
        Ok(parcel("A", &[]))
    );
    let held = StoredManifest::from_value_with(json!({ "parcel": row_3() }), |_, _| {
        stored_manifest_schema::Verdict::Reject
    });
    assert_eq!(
        held.map_err(|refused| lines(&refused)),
        Ok(StoredManifest {
            parcel: parcel("A", &[])
        })
    );
    let flattened = StoredCrated::from_value_with(
        json!({ "dock": "north", "label": "A", "itemCodeList": ["x"] }),
        |_, _| stored_crated_schema::Verdict::Reject,
    );
    assert_eq!(
        flattened.map_err(|refused| lines(&refused)),
        Ok(StoredCrated {
            dock: "north".to_owned(),
            parcel: parcel("A", &[])
        })
    );
}

#[test]
fn the_callback_is_handed_the_value_the_function_of_the_type_corrected() {
    let mut handed: Option<Value> = None;
    let mut listed: Vec<String> = Vec::new();
    let read = StoredParcel::from_value_with(
        json!({ "label": "  A  ", "itemCodeList": 5_i32 }),
        |value, found| {
            handed = Some(value.clone());
            listed.extend(found.iter().map(|issue| format!("{issue:?}")));
            if let Some(codes) = value.get_mut("itemCodeList") {
                *codes = json!(["x"]);
            }
            stored_parcel_schema::Verdict::Fixed
        },
    );
    // The label is as it was written: its function is called where the label is read.
    assert_eq!(
        handed,
        Some(json!({ "label": "  A  ", "itemCodeList": 5_i32, "fragileItemCodes": [] }))
    );
    assert_eq!(
        listed,
        [
            "Invalid { path: Path([Key(\"itemCodeList\")]), expected: Array(String), found: \
             Number(5), reason: \"not an array\" }"
        ]
    );
    assert_eq!(
        read.map_err(|refused| lines(&refused)),
        Ok(parcel("A", &[]))
    );
}

/// A resolver is handed the value the function of the type corrected, as the callback is.
#[test]
fn a_resolver_is_handed_the_value_the_function_of_the_type_corrected() {
    let handed: Mutex<Vec<Value>> = Mutex::new(Vec::new());
    let repairing = |raw: &mut Value, _issue: &stored_parcel_schema::Issue<Value>| {
        handed.lock().unwrap().push(raw.clone());
        if let Some(codes) = raw.get_mut("itemCodeList") {
            *codes = json!(["x"]);
        }
        stored_parcel_schema::Resolution::Settled
    };
    let read = StoredParcel::from_value_piped(
        json!({ "label": "  A  ", "itemCodeList": 5_i32 }),
        &[&repairing],
    );
    assert_eq!(
        read.map_err(|refused| lines(&refused)),
        Ok(parcel("A", &[]))
    );
    assert_eq!(
        handed.into_inner().unwrap(),
        [json!({ "label": "  A  ", "itemCodeList": 5_i32, "fragileItemCodes": [] })]
    );
}

/// A payload the function of a field leaves unreadable is one issue at the field, in serde's
/// words, and its text is the one the payload holds.
#[test]
fn a_field_its_function_leaves_unreadable_is_one_issue_at_the_field() {
    let (read, calls) = rejected(json!({ "label": 5_i32, "itemCodeList": ["x"] }));
    assert_eq!(
        read,
        Err(vec![
            "label: invalid: expected String, found Number(5): invalid type: integer `5`, \
             expected a string"
                .to_owned()
        ])
    );
    assert_eq!(calls, 1);
}

/// The held parcel is corrected where it is walked: what its function fixed is not listed, and
/// what is broken in it is listed at its place in the manifest.
#[test]
fn a_held_type_is_corrected_where_it_is_walked() {
    let held = StoredManifest::from_value_with(
        json!({ "parcel": { "label": "A", "itemCodeList": 5_i32 } }),
        |_, _| stored_manifest_schema::Verdict::Reject,
    );
    assert_eq!(
        held.map_err(|refused| lines(&refused)),
        Err(vec![
            "parcel.itemCodeList: invalid: expected Array(String), found Number(5): not an array"
                .to_owned()
        ])
    );
}

/// The bound is held to the label the field's function answered.
#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
#[test]
fn the_bound_of_a_field_is_checked_on_what_its_function_answered() {
    let (read, calls) =
        rejected(json!({ "label": "   ", "itemCodeList": ["x"], "fragileItemCodes": ["x"] }));
    assert_eq!(
        read,
        Err(vec![
            "label: invalid: expected String, found String(\"   \"): too short: minimum length \
             is 1, got 0"
                .to_owned()
        ])
    );
    assert_eq!(calls, 1);
}

/// A build with no schema surface holds no bound, and the label its function emptied is read.
#[cfg(not(any(feature = "typescript", feature = "zod", feature = "jsonschema")))]
#[test]
fn the_label_a_function_emptied_is_read_where_no_bound_is_held() {
    let (read, calls) =
        rejected(json!({ "label": "   ", "itemCodeList": ["x"], "fragileItemCodes": ["x"] }));
    assert_eq!(read, Ok(parcel("", &["x"])));
    assert_eq!(calls, 0);
}

#[test]
fn an_enum_is_taken_out_of_its_envelope_by_the_recovering_read() {
    let enveloped = json!({ "payload": { "kind": "Air", "flight": "F1" } });
    let read =
        StoredShipment::from_value_with(enveloped, |_, _| stored_shipment_schema::Verdict::Reject);
    assert_eq!(
        read.map_err(|refused| lines(&refused)),
        Ok(StoredShipment::Air {
            flight: "F1".to_owned()
        })
    );
}

/// The function of the type is called once by a recovering read, which takes the value, reads it
/// and lists its issues. The function of a field is called by the read and by the list, each of
/// which reads the field.
#[test]
fn a_recovering_read_calls_the_function_of_the_type_once() {
    let before = (PARCEL_CALLS.get(), LABEL_CALLS.get());
    let read = CountedParcel::from_value_with(json!({ "label": "A" }), |_, _| {
        counted_parcel_schema::Verdict::Reject
    });
    assert_eq!(read.map(|held| held.label).ok().as_deref(), Some("A"));
    assert_eq!(
        (PARCEL_CALLS.get(), LABEL_CALLS.get()),
        (before.0 + 1, before.1 + 2)
    );
    // A value the callback repairs is read again, and the type's function is not called again.
    let repaired = CountedParcel::from_value_with(json!({ "label": 5_i32 }), |value, _| {
        *value = json!({ "label": "A" });
        counted_parcel_schema::Verdict::Fixed
    });
    assert_eq!(repaired.map(|held| held.label).ok().as_deref(), Some("A"));
    assert_eq!(PARCEL_CALLS.get(), before.0 + 2);
}

/// Under `bson` the two entry points over a document read it the same way.
#[cfg(feature = "bson")]
#[test]
fn a_document_is_read_as_the_value_is() {
    let row_3 = bson::doc! { "label": "A", "itemCodeList": ["x"] };
    let mut calls = 0_u32;
    let read = StoredParcel::from_bson_with(row_3.clone(), |_, _| {
        calls += 1;
        stored_parcel_schema::Verdict::Reject
    });
    assert_eq!(
        read.map_err(|refused| lines(&refused)),
        Ok(parcel("A", &[]))
    );
    assert_eq!(calls, 0);
    assert_eq!(
        StoredParcel::from_bson_piped(row_3, &[]).map_err(|refused| lines(&refused)),
        Ok(parcel("A", &[]))
    );
    let row_1 = bson::doc! { "label": "  A  ", "itemCodeList": ["x"], "fragileItemCodes": ["x"] };
    assert_eq!(
        StoredParcel::from_bson_piped(row_1, &[]).map_err(|refused| lines(&refused)),
        Ok(parcel("A", &["x"]))
    );
}
