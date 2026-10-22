//! The JSON Schema of a struct a map is flattened into as the only member of its object: the map's
//! values describe every key the object holds, and a key the object names that serde hands the
//! map as well is held to both.

use alloc::collections::BTreeMap;

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tixschema::model_schema;

use super::documents::admits;
use super::maps::{
    Bilge, Coaming, Gunwale, Keelson, PAYLOADS, Strake, TaggedThwart, Thwart, Transom, serde_reads,
};

/// Payloads of the models declared here, each with whether serde reads it.
const MORE_PAYLOADS: [(&str, &str, bool); 9] = [
    ("Bitt", r#"{"Fore":2}"#, true),
    ("Bitt", r#"{"Port":2}"#, false),
    ("Bitt", r#"{"Fore":"x"}"#, false),
    ("TaggedBitt", r#"{"kind":"TaggedBitt","Fore":2}"#, false),
    ("Ballasted", r#"{"north":2}"#, true),
    ("Ballasted", r#"{"north":"x"}"#, false),
    ("Plumbed", "{}", true),
    ("Plumbed", r#"{"7":2}"#, false),
    ("Plumbed", r#"{"north":2}"#, false),
];

/// Each model a payload is read as, beside its JSON Schema.
const DOCUMENTS: [(&str, Written); 12] = [
    ("Thwart", Thwart::json_schema),
    ("TaggedThwart", TaggedThwart::json_schema),
    ("Gunwale", Gunwale::json_schema),
    ("Strake", Strake::json_schema),
    ("Coaming", Coaming::json_schema),
    ("Keelson", Keelson::json_schema),
    ("Transom", Transom::json_schema),
    ("Bilge", Bilge::json_schema),
    ("Bitt", Bitt::json_schema),
    ("TaggedBitt", TaggedBitt::json_schema),
    ("Ballasted", Ballasted::json_schema),
    ("Plumbed", Plumbed::json_schema),
];

/// Each model declared here, beside serde's read of a payload as it.
const READS_HERE: [(&str, ReadAs); 4] = [
    ("Bitt", reads::<Bitt>),
    ("TaggedBitt", reads::<TaggedBitt>),
    ("Ballasted", reads::<Ballasted>),
    ("Plumbed", reads::<Plumbed>),
];

/// The keys of a map a plain enum keys.
#[model_schema()]
#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub enum Cleat {
    Aft,
    Fore,
}

/// A map keyed by a plain enum.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Bitt {
    #[serde(flatten)]
    pub lines: BTreeMap<Cleat, u8>,
}

/// A map keyed by a plain enum in a struct written under its own tag, which serde hands the map.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind")]
pub struct TaggedBitt {
    #[serde(flatten)]
    pub lines: BTreeMap<Cleat, u8>,
}

/// A map under a name of its own.
#[model_schema()]
pub type HoldTallies = BTreeMap<String, u8>;

/// A map flattened through an alias.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Ballasted {
    #[serde(flatten)]
    pub counts: HoldTallies,
}

/// A map keyed by a number, which serde reads no key of once flattened.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Plumbed {
    #[serde(flatten)]
    pub depths: BTreeMap<u32, u8>,
}

/// A model's JSON Schema.
type Written = fn() -> Value;

/// Whether serde reads a payload as one model.
type ReadAs = fn(&str) -> bool;

/// The JSON Schema of the model a payload is read as.
fn document(model: &str) -> Value {
    let (_, written) = DOCUMENTS.iter().find(|(name, _)| *name == model).unwrap();
    written()
}

/// Whether serde reads `json` as a `T`.
fn reads<T>(json: &str) -> bool
where
    T: DeserializeOwned,
{
    serde_json::from_str::<T>(json).is_ok()
}

/// Whether serde reads `json` as one of the models declared here.
fn serde_reads_here(model: &str, json: &str) -> bool {
    let (_, read) = READS_HERE.iter().find(|(name, _)| *name == model).unwrap();
    read(json)
}

/// Whether the JSON Schema of `model` admits `json`.
fn admitted(model: &str, json: &str) -> bool {
    admits(&document(model), &serde_json::from_str(json).unwrap())
}

#[test]
fn the_json_schema_admits_each_payload_serde_reads() {
    for payload in &PAYLOADS {
        assert_eq!(
            admitted(payload.read_as, payload.json),
            serde_reads(payload),
            "serde and the JSON Schema part on `{}`: {}",
            payload.name,
            document(payload.read_as)
        );
    }
}

#[test]
fn a_map_keyed_by_an_enum_a_number_or_through_an_alias_admits_what_serde_reads() {
    for (model, json, read) in MORE_PAYLOADS {
        assert_eq!(
            serde_reads_here(model, json),
            read,
            "serde on {model} {json}"
        );
        assert_eq!(
            admitted(model, json),
            read,
            "the JSON Schema of {model} on {json}: {}",
            document(model)
        );
    }
}

/// A map alone in its object describes every key.
#[test]
fn a_map_alone_describes_every_key() {
    assert_eq!(
        Thwart::json_schema(),
        json!({
            "type": "object",
            "properties": {},
            "required": [],
            "additionalProperties": { "type": "integer" },
        })
    );
}

/// A struct's own tag is handed to the map as well, and held to both.
#[test]
fn a_structs_own_tag_is_held_to_the_maps_values_as_well() {
    assert_eq!(
        TaggedThwart::json_schema(),
        json!({
            "type": "object",
            "properties": {
                "kind": { "allOf": [{ "type": "string", "const": "TaggedThwart" }, { "type": "string" }] },
            },
            "required": ["kind"],
            "additionalProperties": { "type": "string" },
        })
    );
}
