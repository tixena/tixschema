//! A field typed with an order-keeping map, read from a row: each issue sits at its key's path, and
//! the map holds its keys in the order the row stored them, which a `bson` document keeps.

#[cfg(feature = "indexmap")]
mod index_map;

use bson::doc;
use serde::{Deserialize, Serialize};
use tixschema::model_schema;

/// A JSON object held as `serde_json`'s own map.
#[model_schema(enable = ["decode_with", "mongodb"])]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct Dossier {
    raw: serde_json::Map<String, serde_json::Value>,
    title: String,
}

#[test]
fn a_json_object_field_is_read_from_a_document_and_no_other_value() {
    let mut calls = 0_u32;
    let read = Dossier::from_bson_with(
        doc! { "raw": { "zeta": 1_i32, "alpha": { "nested": [true] } }, "title": "t" },
        |_raw, _found| {
            calls += 1;
            dossier_schema::Verdict::Reject
        },
    )
    .unwrap();
    assert_eq!(calls, 0);
    assert_eq!(
        serde_json::Value::Object(read.raw),
        serde_json::json!({ "zeta": 1_i32, "alpha": { "nested": [true] } })
    );

    let refused =
        Dossier::from_bson_with(doc! { "raw": ["zeta"], "title": "t" }, |_raw, _found| {
            dossier_schema::Verdict::Reject
        })
        .unwrap_err();
    assert_eq!(refused.issues.len(), 1);
    let shown = refused.to_string();
    assert!(
        shown.starts_with("raw: invalid: expected Map(Unknown), found "),
        "got: {shown}"
    );
    assert!(shown.ends_with(": not an object"), "got: {shown}");
}
