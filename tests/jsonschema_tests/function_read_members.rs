//! The JSON Schema of a `nonstrict` holder of a member flattened through a function of its
//! author's, which calls no function: beside a `deserialize_with` it reads past a key no member
//! declares, as serde does where the function reads the type as serde's derive does, and beside a
//! `preprocess` it describes an optional member as absent or as its type, whatever the function
//! answers. `docs/guide/flatten.md` and `docs/guide/preprocess.md` state what these measure.

use serde::de::Error as _;
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::{Value, json};
use tixschema::model_schema;

#[model_schema(enable = ["jsonschema"])]
#[derive(Debug, Deserialize, Serialize)]
pub struct Version {
    pub major: u32,
}

#[model_schema(nonstrict, enable = ["jsonschema"])]
#[derive(Debug, Deserialize, Serialize)]
pub struct Release {
    pub id: String,
    #[serde(flatten, deserialize_with = "as_written")]
    pub version: Version,
}

#[model_schema(nonstrict, enable = ["jsonschema"])]
#[derive(Debug, Deserialize, Serialize)]
pub struct Rerelease {
    pub id: String,
    #[serde(flatten, deserialize_with = "through_a_value")]
    pub version: Version,
}

#[model_schema(enable = ["jsonschema"])]
#[derive(Debug, Deserialize, Serialize)]
pub struct Tholepin {
    pub label: String,
}

#[model_schema(nonstrict, enable = ["jsonschema"])]
#[derive(Debug, Deserialize, Serialize)]
pub struct Reeved {
    pub note: String,
    #[model_schema_prop(preprocess = ["asGiven"])]
    #[serde(flatten, default, skip_serializing_if = "Option::is_none")]
    pub tholepin: Option<Tholepin>,
}

/// `Version` read as serde's derive reads it, from the keys serde hands it.
fn as_written<'de, D>(deserializer: D) -> Result<Version, D::Error>
where
    D: Deserializer<'de>,
{
    Version::deserialize(deserializer)
}

/// `Version` read from every key serde hands it, collected into a `Value` first.
fn through_a_value<'de, D>(deserializer: D) -> Result<Version, D::Error>
where
    D: Deserializer<'de>,
{
    let value = Value::deserialize(deserializer)?;
    Version::deserialize(value).map_err(D::Error::custom)
}

/// The payload as it was handed.
const fn as_given(payload: Value) -> Value {
    payload
}

fn takes(schema: &Value, payload: &Value) -> bool {
    jsonschema::validator_for(schema).unwrap().is_valid(payload)
}

/// What serde reads of `payload` as a `T`, written back, or its refusal.
fn serde_reads<T>(payload: &Value) -> String
where
    T: for<'de> Deserialize<'de> + Serialize,
{
    serde_json::from_value::<T>(payload.clone()).map_or_else(
        |refused| format!("refused: {refused}"),
        |read| serde_json::to_string(&read).unwrap(),
    )
}

#[test]
fn a_nonstrict_holder_of_a_member_read_through_a_function_reads_past_a_stray_key() {
    let stray = json!({ "id": "a", "major": 1_u32, "stray": 1_u32 });
    assert_eq!(serde_reads::<Release>(&stray), r#"{"id":"a","major":1}"#);
    assert!(takes(&Release::json_schema(), &stray));
    assert_eq!(
        serde_reads::<Rerelease>(&stray),
        "refused: unknown field `stray`, expected `major`"
    );
    let guide = include_str!("../../docs/guide/flatten.md");
    for stated in [
        "serde reads `{\"id\":\"a\",\"major\":1,\"stray\":1}` past `stray` through `as_written`",
        "with ``unknown field `stray`, expected `major` ``",
        "Marked `nonstrict`, `Release` reads `{\"id\":\"a\",\"major\":1,\"stray\":1}` in serde and \
         in the JSON Schema, and writes it back without `stray`.",
    ] {
        assert!(
            guide.contains(stated),
            "flatten.md no longer states: {stated}"
        );
    }
}

#[test]
fn the_json_schema_takes_what_serde_refuses_in_what_a_function_answers() {
    let schema = Reeved::json_schema();
    for (payload, refused) in [
        (json!({ "note": "n" }), "refused: missing field `label`"),
        (
            json!({ "label": 5_u32, "note": "n" }),
            "refused: invalid type: integer `5`, expected a string",
        ),
    ] {
        assert!(takes(&schema, &payload), "{payload}");
        assert_eq!(serde_reads::<Reeved>(&payload), refused, "{payload}");
    }
    let stated = "takes `{\"note\":\"n\"}` and `{\"label\":5,\"note\":\"n\"}`";
    for (file, guide) in [
        ("flatten.md", include_str!("../../docs/guide/flatten.md")),
        (
            "preprocess.md",
            include_str!("../../docs/guide/preprocess.md"),
        ),
    ] {
        assert!(guide.contains(stated), "{file} no longer states: {stated}");
        assert!(
            guide.contains("``missing field `label` ``")
                && guide.contains("``invalid type: integer `5`, expected a string``"),
            "{file} no longer quotes serde's refusals"
        );
    }
}
