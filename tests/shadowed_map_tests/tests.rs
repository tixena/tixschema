//! Tests of a model type named `Map`, and of ones named `IndexMap` and `IndexSet` where the
//! `indexmap` feature is off.

#[cfg(not(feature = "indexmap"))]
mod named_as_indexmap;

use serde::{Deserialize, Serialize};
use serde_json::json;
use tixschema::model_schema;

#[model_schema(decode_with, default_types(K = String, V = u32))]
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Map<K, V> {
    pub key: K,
    pub value: V,
}

#[model_schema(decode_with)]
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Holder {
    pub held: Map<String, u32>,
}

/// A key the type does not declare is unknown, where a map would have held it as an entry.
#[test]
fn a_field_written_with_the_name_is_read_as_the_type_declared_here() {
    let stored = json!({ "held": { "key": "k", "value": 7_u32 } });
    let read: Holder = serde_json::from_value(stored.clone()).unwrap();
    assert_eq!(
        read,
        Holder {
            held: Map {
                key: "k".to_owned(),
                value: 7_u32,
            },
        }
    );
    let mut calls = 0_u32;
    let walked = Holder::from_value_with(stored, |_raw, _found| {
        calls += 1;
        holder_schema::Verdict::Reject
    });
    assert_eq!(walked, Ok(read));
    assert_eq!(calls, 0);

    let entered = json!({ "held": { "key": "k", "other": 1_u32, "value": 7_u32 } });
    let refused = Holder::from_value_with(entered, |_raw, _found| holder_schema::Verdict::Reject)
        .unwrap_err();
    assert_eq!(refused.to_string(), "held.other: unknown: found Number(1)");
}

#[cfg(feature = "typescript")]
#[test]
fn typescript_names_the_type_declared_here() {
    let ts = Holder::ts_definition();
    assert!(ts.contains("  held: Map<string, number>;"), "got: {ts}");
}

#[cfg(feature = "zod")]
#[test]
fn zod_calls_the_factory_of_the_type_declared_here() {
    let zod = Holder::zod_schema();
    assert!(
        zod.contains("Map$SchemaFactory(z.string(), z.number().int())"),
        "got: {zod}"
    );
    assert!(!zod.contains("z.record("), "got: {zod}");
}

#[cfg(feature = "jsonschema")]
#[test]
fn the_json_schema_is_the_document_of_the_type_declared_here() {
    let held = &Holder::json_schema()["properties"]["held"];
    assert_eq!(held["properties"]["key"], json!({ "type": "string" }));
    assert_eq!(held["required"], json!(["key", "value"]));
}

#[cfg(feature = "mongodb")]
#[test]
fn the_paths_of_the_type_declared_here_are_reached_through_the_field() {
    let held: holder_schema::Model<Holder, Map<String, u32>> = Holder::MONGO_FIELDS.held();
    let asked = held.nest(Map::<String, u32>::MONGO_FIELDS.value().gt(5_u32).unwrap());
    assert_eq!(
        asked.into_document(),
        bson::doc! { "held.value": { "$gt": 5_i64 } }
    );
}

#[cfg(feature = "dart")]
#[test]
fn dart_reads_the_field_through_the_type_declared_here() {
    let dart = holder_dart::dart_definition();
    assert!(
        dart.contains("held: Map.fromJsonUnjudged(json['held'], "),
        "got: {dart}"
    );
}

#[cfg(feature = "swift")]
#[test]
fn swift_names_the_type_declared_here() {
    let swift = holder_swift::swift_definition();
    assert!(swift.contains("held: Map<String, UInt32>"), "got: {swift}");
}
