//! The JSON Schema of a struct a map is flattened into: the map's values describe every key no
//! member names, and a key a member names that serde hands the map as well is held to both.

use alloc::collections::BTreeMap;

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tixschema::model_schema;

use super::documents::admits;
use super::maps::{
    Ballast, Billing, Bulkhead, Compartment, Cording, Davit, Dunnage, Enclosure, Hatchway, Marking,
    PAYLOADS, Pigeonhole, Rigging, Scuttle, Sheeting, Shelflist, Stocktake, Stowing,
    TaggedStocktake, Tiers, Trussing, marking_schema, serde_reads, stocktake_schema,
};

/// Payloads of the models declared here, each with whether serde reads it.
const MORE_PAYLOADS: [(&str, &str, bool); 15] = [
    ("Lading", r#"{"crew":"c","note":"a","north":2}"#, true),
    ("Lading", r#"{"crew":"c","note":"a","north":"x"}"#, false),
    ("Lashing", r#"{"label":"n","note":"a","north":2}"#, true),
    ("Lashing", r#"{"label":"n","note":"a","north":"x"}"#, false),
    ("Strapping", r#"{"label":"n","note":"a","north":2}"#, false),
    ("Mooring", r#"{"note":"a","Fore":2}"#, true),
    ("Mooring", r#"{"note":"a","Port":2}"#, false),
    ("Mooring", r#"{"note":"a","Fore":"x"}"#, false),
    ("Fairlead", r#"{"Fore":2,"label":"n"}"#, false),
    (
        "Fairway",
        r#"{"kind":"Fairway","note":"a","Fore":2}"#,
        false,
    ),
    ("Manifest", r#"{"note":"a","north":2}"#, true),
    ("Manifest", r#"{"note":"a","north":"x"}"#, false),
    ("Soundings", r#"{"note":"a"}"#, true),
    ("Soundings", r#"{"note":"a","7":2}"#, false),
    ("Soundings", r#"{"note":"a","north":2}"#, false),
];

/// Each model a payload is read as, beside its JSON Schema.
const DOCUMENTS: [(&str, Written); 26] = [
    ("Stocktake", Stocktake::json_schema),
    ("TaggedStocktake", TaggedStocktake::json_schema),
    ("Stowing", Stowing::json_schema),
    ("Trussing", Trussing::json_schema),
    ("Ballast", Ballast::json_schema),
    ("Dunnage", Dunnage::json_schema),
    ("Tiers", Tiers::json_schema),
    ("Sheeting", Sheeting::json_schema),
    ("Cording", Cording::json_schema),
    ("Rigging", Rigging::json_schema),
    ("Pigeonhole", Pigeonhole::json_schema),
    ("Davit", Davit::json_schema),
    ("Shelflist", Shelflist::json_schema),
    ("Billing", Billing::<String>::json_schema),
    ("Hatchway", Hatchway::json_schema),
    ("Compartment", Compartment::json_schema),
    ("Bulkhead", Bulkhead::json_schema),
    ("Scuttle", Scuttle::json_schema),
    ("Lading", Lading::json_schema),
    ("Lashing", Lashing::json_schema),
    ("Strapping", Strapping::json_schema),
    ("Mooring", Mooring::json_schema),
    ("Fairlead", Fairlead::json_schema),
    ("Fairway", Fairway::json_schema),
    ("Manifest", Manifest::json_schema),
    ("Soundings", Soundings::json_schema),
];

/// Each model declared here, beside serde's read of a payload as it.
const READS_HERE: [(&str, ReadAs); 8] = [
    ("Lading", reads::<Lading>),
    ("Lashing", reads::<Lashing>),
    ("Strapping", reads::<Strapping>),
    ("Mooring", reads::<Mooring>),
    ("Fairlead", reads::<Fairlead>),
    ("Fairway", reads::<Fairway>),
    ("Manifest", reads::<Manifest>),
    ("Soundings", reads::<Soundings>),
];

/// A struct with a flattened map, beside a member of its own holder.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Lading {
    pub crew: String,
    #[serde(flatten)]
    pub stock: Stocktake,
}

/// A struct with a flattened map, after a struct that takes its keys first.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Lashing {
    #[serde(flatten)]
    pub marking: Marking,
    #[serde(flatten)]
    pub stock: Stocktake,
}

/// A struct with a flattened map, ahead of a struct: serde hands the map the struct's keys too.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Strapping {
    #[serde(flatten)]
    pub bundle: Stocktake,
    #[serde(flatten)]
    pub marking: Marking,
}

/// The keys of a map a plain enum keys.
#[model_schema()]
#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub enum Berth {
    Aft,
    Fore,
}

/// A map keyed by a plain enum, beside a member of its own holder.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Mooring {
    #[serde(flatten)]
    pub lines: BTreeMap<Berth, u8>,
    pub note: String,
}

/// A map keyed by a plain enum, ahead of a struct: serde hands the map the struct's keys, which no
/// member of the enum is.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Fairlead {
    #[serde(flatten)]
    pub lines: BTreeMap<Berth, u8>,
    #[serde(flatten)]
    pub marking: Marking,
}

/// A map keyed by a plain enum in a struct written under its own tag, which serde hands the map.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind")]
pub struct Fairway {
    #[serde(flatten)]
    pub lines: BTreeMap<Berth, u8>,
    pub note: String,
}

/// A map under a name of its own.
#[model_schema()]
pub type HoldCounts = BTreeMap<String, u8>;

/// A map flattened through an alias, beside a member of its own holder.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Manifest {
    #[serde(flatten)]
    pub counts: HoldCounts,
    pub note: String,
}

/// A map keyed by a number, which serde reads no key of once flattened.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Soundings {
    #[serde(flatten)]
    pub depths: BTreeMap<u32, u8>,
    pub note: String,
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
fn a_map_in_a_flattened_struct_an_alias_or_keyed_by_an_enum_admits_what_serde_reads() {
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

/// A map beside a member describes every other key.
#[test]
fn a_map_beside_a_member_describes_every_other_key() {
    assert_eq!(
        Stocktake::json_schema(),
        json!({
            "type": "object",
            "properties": { "note": { "type": "string" } },
            "required": ["note"],
            "additionalProperties": { "type": "integer" },
        })
    );
}

/// A map after a flattened struct leaves the struct's keys to the struct.
#[test]
fn a_map_after_a_struct_leaves_its_keys_to_the_struct() {
    assert_eq!(
        Stowing::json_schema(),
        json!({
            "type": "object",
            "properties": { "label": { "type": "string" } },
            "required": ["label"],
            "additionalProperties": { "type": "integer" },
        })
    );
}

/// Two maps are handed the same keys, and a value is held to both.
#[test]
fn two_maps_hold_a_value_to_both() {
    assert_eq!(
        Tiers::json_schema(),
        json!({
            "type": "object",
            "properties": {},
            "required": [],
            "additionalProperties": { "allOf": [{ "type": "integer" }, { "type": "integer" }] },
        })
    );
}

/// A map ahead of a flattened struct holds the struct's keys too.
#[test]
fn a_map_ahead_of_a_struct_holds_its_keys_too() {
    assert_eq!(
        Ballast::json_schema(),
        json!({
            "type": "object",
            "properties": {
                "label": { "allOf": [{ "type": "string" }, { "type": "integer" }] },
            },
            "required": ["label"],
            "additionalProperties": { "type": "integer" },
        })
    );
}

/// A struct with no flattened map describes as before: closed to every key no member names.
#[test]
fn a_struct_with_no_flattened_map_is_closed() {
    let closed = json!({
        "type": "object",
        "properties": { "label": { "type": "string" } },
        "required": ["label"],
        "additionalProperties": false,
    });
    assert_eq!(Marking::json_schema(), closed);
    assert_eq!(Enclosure::json_schema(), closed);
}
