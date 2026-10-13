//! Tests of model types named `IndexMap` and `IndexSet` where the `indexmap` feature is on.

use serde::{Deserialize, Serialize};
use serde_json::json;
use tixschema::model_schema;

const WRITTEN: &str = r#"{"held":{"key":"k","value":7},"listed":{"only":"x"}}"#;

#[model_schema(decode_with, default_types(K = String, V = u32))]
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct IndexMap<K, V> {
    pub key: K,
    pub value: V,
}

#[model_schema(decode_with, default_types(T = String))]
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct IndexSet<T> {
    pub only: T,
}

#[model_schema()]
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Holder {
    pub held: IndexMap<String, u32>,
    pub listed: IndexSet<String>,
}

#[model_schema(decode_with)]
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Walked {
    pub held: IndexMap<String, u32>,
    pub listed: IndexSet<String>,
}

#[model_schema()]
pub type Counts = IndexMap<String, u32>;

#[model_schema()]
pub type Names = IndexSet<String>;

#[model_schema(decode_with)]
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Aliased {
    pub counts: Counts,
    pub names: Names,
}

#[test]
fn serde_writes_the_types_declared_here() {
    let holder = Holder {
        held: IndexMap {
            key: "k".to_owned(),
            value: 7_u32,
        },
        listed: IndexSet {
            only: "x".to_owned(),
        },
    };
    assert_eq!(serde_json::to_string(&holder).unwrap(), WRITTEN);
    assert_eq!(serde_json::from_str::<Holder>(WRITTEN).unwrap(), holder);
}

/// A key a type does not declare is unknown, where a map would have held it as an entry.
#[test]
fn the_recovering_decode_walks_the_types_declared_here() {
    let stored: serde_json::Value = serde_json::from_str(WRITTEN).unwrap();
    let read: Walked = serde_json::from_value(stored.clone()).unwrap();
    let mut calls = 0_u32;
    let walked = Walked::from_value_with(stored, |_raw, _found| {
        calls += 1;
        walked_schema::Verdict::Reject
    });
    assert_eq!(walked, Ok(read));
    assert_eq!(calls, 0);

    let entered = json!({
        "held": { "key": "k", "other": 1_u32, "value": 7_u32 },
        "listed": { "only": "x" },
    });
    let refused = Walked::from_value_with(entered, |_raw, _found| walked_schema::Verdict::Reject)
        .unwrap_err();
    assert_eq!(refused.to_string(), "held.other: unknown: found Number(1)");
}

#[test]
fn the_recovering_decode_reads_an_alias_of_each_type_declared_here() {
    let stored = json!({
        "counts": { "key": "k", "value": 7_u32 },
        "names": { "only": "x" },
    });
    let read: Aliased = serde_json::from_value(stored.clone()).unwrap();
    let mut calls = 0_u32;
    let aliased = Aliased::from_value_with(stored, |_raw, _found| {
        calls += 1;
        aliased_schema::Verdict::Reject
    });
    assert_eq!(aliased, Ok(read));
    assert_eq!(calls, 0);
}

#[cfg(feature = "typescript")]
#[test]
fn typescript_names_the_types_declared_here() {
    let ts = Holder::ts_definition();
    assert!(
        ts.contains("  held: IndexMap<string, number>;"),
        "got: {ts}"
    );
    assert!(ts.contains("  listed: IndexSet<string>;"), "got: {ts}");
}

#[cfg(feature = "zod")]
#[test]
fn zod_calls_the_factories_of_the_types_declared_here() {
    let zod = Holder::zod_schema();
    for call in [
        "IndexMap$SchemaFactory(z.string(), z.number().int())",
        "IndexSet$SchemaFactory(z.string())",
    ] {
        assert!(zod.contains(call), "missing `{call}` in: {zod}");
    }
    assert!(!zod.contains("z.record("), "got: {zod}");
    assert!(!zod.contains("z.array("), "got: {zod}");
}

#[cfg(feature = "jsonschema")]
#[test]
fn the_json_schema_is_the_document_of_the_types_declared_here() {
    let properties = &Holder::json_schema()["properties"];
    assert_eq!(properties["held"]["required"], json!(["key", "value"]));
    assert_eq!(properties["listed"]["required"], json!(["only"]));
}

#[cfg(feature = "mongodb")]
#[test]
fn the_paths_of_the_types_declared_here_are_reached_through_the_fields() {
    let held: walked_schema::Model<Walked, IndexMap<String, u32>> = Walked::MONGO_FIELDS.held();
    let counted = held.nest(
        IndexMap::<String, u32>::MONGO_FIELDS
            .value()
            .gt(5_u32)
            .unwrap(),
    );
    assert_eq!(
        counted.into_document(),
        bson::doc! { "held.value": { "$gt": 5_i64 } }
    );

    let listed: walked_schema::Model<Walked, IndexSet<String>> = Walked::MONGO_FIELDS.listed();
    let named = listed.nest(
        IndexSet::<String>::MONGO_FIELDS
            .only()
            .eq("x".to_owned())
            .unwrap(),
    );
    assert_eq!(
        named.into_document(),
        bson::doc! { "listed.only": { "$eq": "x" } }
    );
}

#[cfg(feature = "dart")]
#[test]
fn dart_reads_the_fields_through_the_types_declared_here() {
    let dart = holder_dart::dart_definition();
    for read in [
        "held: IndexMap.fromJsonUnjudged(json['held'], ",
        "listed: IndexSet.fromJsonUnjudged(json['listed'], ",
    ] {
        assert!(dart.contains(read), "missing `{read}` in: {dart}");
    }
}

#[cfg(feature = "swift")]
#[test]
fn swift_names_the_types_declared_here() {
    let swift = holder_swift::swift_definition();
    assert!(
        swift.contains("held: IndexMap<String, UInt32>"),
        "got: {swift}"
    );
    assert!(swift.contains("listed: IndexSet<String>"), "got: {swift}");
}

#[cfg(feature = "kotlin")]
#[test]
fn kotlin_names_the_types_declared_here() {
    let kotlin = holder_kotlin::kotlin_definition();
    assert!(
        kotlin.contains("val held: IndexMap<String, UInt>"),
        "got: {kotlin}"
    );
    assert!(
        kotlin.contains("val listed: IndexSet<String>"),
        "got: {kotlin}"
    );
}
