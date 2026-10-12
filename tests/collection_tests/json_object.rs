//! `serde_json::Map<String, Value>` is a map on every surface: what a `HashMap<String, Value>`
//! gives, under no feature.

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};
use std::collections::HashMap;
use tixschema::model_schema;

// A `//` comment on each: a doc comment is written into the outputs the two are compared by.
// The map under its full path and under its last name, and under what a field may hold it in.
#[model_schema()]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JsonObjectFields {
    pub listed: Vec<Map<String, Value>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub optional: Option<Map<String, Value>>,
    pub raw: serde_json::Map<String, serde_json::Value>,
}

#[model_schema()]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JsonObjectFieldsTwin {
    pub listed: Vec<HashMap<String, Value>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub optional: Option<HashMap<String, Value>>,
    pub raw: HashMap<String, serde_json::Value>,
}

/// `output`, which the twin gave, as the type it is the twin of would have given it.
#[cfg(any(
    feature = "typescript",
    feature = "zod",
    feature = "dart",
    feature = "swift",
    feature = "kotlin"
))]
fn under_its_own_name(output: &str) -> String {
    output.replace("Twin", "")
}

#[test]
fn a_json_object_field_reads_and_writes_what_its_std_twin_does() {
    let stored = json!({
        "listed": [{ "zeta": 1_i32 }, {}],
        "optional": { "alpha": null },
        "raw": { "zeta": "1", "alpha": { "nested": [true] } },
    });
    let read: JsonObjectFields = serde_json::from_value(stored.clone()).unwrap();
    assert_eq!(Some(&read.raw), stored["raw"].as_object());
    assert_eq!(read.listed.len(), 2);
    let twin: JsonObjectFieldsTwin = serde_json::from_value(stored.clone()).unwrap();
    assert_eq!(twin.raw.get("zeta"), Some(&json!("1")));
    assert_eq!(twin.listed.len(), 2);
    assert_eq!(serde_json::to_value(&read).unwrap(), stored);
    assert_eq!(serde_json::to_value(&twin).unwrap(), stored);
    let unset: JsonObjectFields =
        serde_json::from_value(json!({ "listed": [], "raw": {} })).unwrap();
    assert_eq!(unset.optional, None);
}

#[cfg(feature = "typescript")]
#[test]
fn typescript_writes_a_json_object_as_its_std_twin() {
    let ts = JsonObjectFields::ts_definition();
    assert!(
        ts.contains("  raw: Partial<Record<string, unknown>>;"),
        "got: {ts}"
    );
    assert!(
        ts.contains("  listed: Array<Partial<Record<string, unknown>>>;"),
        "got: {ts}"
    );
    assert_eq!(
        ts,
        under_its_own_name(&JsonObjectFieldsTwin::ts_definition())
    );
}

#[cfg(feature = "zod")]
#[test]
fn zod_writes_a_json_object_as_its_std_twin() {
    let zod = JsonObjectFields::zod_schema();
    assert!(
        zod.contains("raw: z.record(z.string(), z.unknown()),"),
        "got: {zod}"
    );
    assert_eq!(zod, under_its_own_name(&JsonObjectFieldsTwin::zod_schema()));
}

#[cfg(feature = "jsonschema")]
#[test]
fn the_json_schema_of_a_json_object_is_its_std_twins() {
    let schema = JsonObjectFields::json_schema();
    assert_eq!(
        schema["properties"]["raw"],
        json!({ "type": "object", "additionalProperties": {} })
    );
    assert_eq!(schema, JsonObjectFieldsTwin::json_schema());
}

#[cfg(feature = "dart")]
#[test]
fn dart_writes_a_json_object_as_its_std_twin() {
    assert_eq!(
        json_object_fields_dart::dart_definition(),
        under_its_own_name(&json_object_fields_twin_dart::dart_definition())
    );
}

#[cfg(feature = "swift")]
#[test]
fn swift_writes_a_json_object_as_its_std_twin() {
    assert_eq!(
        json_object_fields_swift::swift_definition(),
        under_its_own_name(&json_object_fields_twin_swift::swift_definition())
    );
}

#[cfg(feature = "kotlin")]
#[test]
fn kotlin_writes_a_json_object_as_its_std_twin() {
    assert_eq!(
        json_object_fields_kotlin::kotlin_definition(),
        under_its_own_name(&json_object_fields_twin_kotlin::kotlin_definition())
    );
}
