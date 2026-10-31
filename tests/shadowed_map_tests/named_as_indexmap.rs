//! With the `indexmap` feature off, `IndexMap` and `IndexSet` are names like any other: a type
//! declared under one, above the type that holds it, is the type a field written with it holds.

use indexed_schema::DecodeWith as _;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tixschema::model_schema;

#[model_schema(enable = ["decode_with", "mongodb"], default_types(K = String, V = u32))]
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct IndexMap<K, V> {
    pub key: K,
    pub value: V,
}

#[model_schema(enable = ["decode_with", "mongodb"], default_types(T = String))]
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct IndexSet<T> {
    pub only: T,
}

#[model_schema(enable = ["decode_with", "mongodb"])]
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Indexed {
    pub held: IndexMap<String, u32>,
    pub listed: IndexSet<String>,
}

#[test]
fn a_field_written_with_either_name_is_read_as_the_type_declared_here() {
    let stored = json!({ "held": { "key": "k", "value": 7_u32 }, "listed": { "only": "o" } });
    let read: Indexed = serde_json::from_value(stored.clone()).unwrap();
    assert_eq!(
        read,
        Indexed {
            held: IndexMap {
                key: "k".to_owned(),
                value: 7_u32,
            },
            listed: IndexSet {
                only: "o".to_owned(),
            },
        }
    );
    let mut calls = 0_u32;
    let walked = Indexed::from_value_with(stored, |_raw, _found| {
        calls += 1;
        indexed_schema::Verdict::Reject
    });
    assert_eq!(walked, Ok(read));
    assert_eq!(calls, 0);
}

#[cfg(feature = "typescript")]
#[test]
fn typescript_names_the_types_declared_here() {
    let ts = Indexed::ts_definition();
    assert!(
        ts.contains("  held: IndexMap<string, number>;"),
        "got: {ts}"
    );
    assert!(ts.contains("  listed: IndexSet<string>;"), "got: {ts}");
}

#[cfg(feature = "zod")]
#[test]
fn zod_calls_the_factories_of_the_types_declared_here() {
    let zod = Indexed::zod_schema();
    for call in [
        "IndexMap$SchemaFactory(z.string(), z.number().int().min(0).max(4294967295))",
        "IndexSet$SchemaFactory(z.string())",
    ] {
        assert!(zod.contains(call), "missing `{call}` in: {zod}");
    }
}

#[cfg(feature = "jsonschema")]
#[test]
fn the_json_schema_is_the_document_of_the_types_declared_here() {
    let properties = &Indexed::json_schema()["properties"];
    assert_eq!(properties["held"]["required"], json!(["key", "value"]));
    assert_eq!(properties["listed"]["required"], json!(["only"]));
}
