//! A field typed with an order-keeping map is walked entry by entry, as a `HashMap` field is: each
//! issue at its key's path, through the callback and through the resolvers.

#[cfg(feature = "indexmap")]
mod index_map;

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};
use tixschema::model_schema;

use super::lines;

/// A JSON object held as `serde_json`'s own map, under its full path and under its last name.
#[model_schema(enable = ["decode_with"])]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct Dossier {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    extra: Option<Map<String, Value>>,
    raw: serde_json::Map<String, serde_json::Value>,
    title: String,
}

#[model_schema()]
type RawObject = serde_json::Map<String, serde_json::Value>;

/// The same map behind an alias seen above.
#[model_schema(enable = ["decode_with"])]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct AliasedDossier {
    raw: RawObject,
}

#[test]
fn a_json_object_field_holds_any_entry_and_is_never_listed() {
    let stored = json!({
        "extra": { "seen": null },
        "raw": { "zeta": 1_i32, "alpha": { "nested": [true] } },
        "title": "t",
    });
    let mut calls = 0_u32;
    let read = Dossier::from_value_with(stored.clone(), |_raw, _found| {
        calls += 1;
        dossier_schema::Verdict::Reject
    })
    .unwrap();
    assert_eq!(Some(&read.raw), stored["raw"].as_object());
    assert_eq!(read.extra.as_ref(), stored["extra"].as_object());
    assert_eq!(serde_json::to_value(&read).unwrap(), stored);

    let aliased =
        AliasedDossier::from_value_with(json!({ "raw": stored["raw"] }), |_raw, _found| {
            calls += 1;
            aliased_dossier_schema::Verdict::Reject
        })
        .unwrap();
    assert_eq!(aliased.raw, read.raw);
    assert_eq!(calls, 0);
}

#[test]
fn a_json_object_field_that_holds_no_object_is_listed_at_its_own_path() {
    let stored = json!({ "extra": 7_i32, "raw": ["zeta"], "title": 3_i32 });
    let expected = [
        "extra: invalid: expected Optional(Map(Unknown)), found Number(7): not an object",
        "raw: invalid: expected Map(Unknown), found Array [String(\"zeta\")]: not an object",
        "title: invalid: expected String, found Number(3): invalid type: integer `3`, expected a \
         string",
    ];
    let read = Dossier::from_value_with(stored.clone(), |_raw, _found| {
        dossier_schema::Verdict::Reject
    });
    assert_eq!(lines(&read.unwrap_err()), expected);
    let piped = Dossier::from_value_piped(stored, &[]);
    assert_eq!(lines(&piped.unwrap_err()), expected);

    let aliased = AliasedDossier::from_value_with(json!({ "raw": 5_i32 }), |_raw, _found| {
        aliased_dossier_schema::Verdict::Reject
    });
    assert_eq!(
        lines(&aliased.unwrap_err()),
        ["raw: invalid: expected Map(Unknown), found Number(5): not an object"]
    );
}
