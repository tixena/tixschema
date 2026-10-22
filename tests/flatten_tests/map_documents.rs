//! The JSON Schema of a struct a map is flattened into as the only member of its object: the map's
//! values describe every key the object holds, and a key read from its text, a `char` or a date,
//! names the keys by its own description. An optional map's absence admits every key, with any
//! value.

use alloc::collections::BTreeMap;

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tixschema::model_schema;

use super::documents::admits;
use super::maps::{
    Bilge, Capstan, Coaming, Davit, Deadeye, Fairlead, Gunwale, Hawser, Lazarette, Locker,
    PAYLOADS, Pintle, Ratlines, Scupper, Sheave, Stanchion, Strake, Thole, Thwart, Transom,
    Windlass, serde_reads,
};

/// Payloads of the models declared here, each with whether serde reads it.
const MORE_PAYLOADS: [(&str, &str, bool); 5] = [
    ("Bitt", r#"{"Fore":2}"#, true),
    ("Bitt", r#"{"Port":2}"#, false),
    ("Bitt", r#"{"Fore":"x"}"#, false),
    ("Ballasted", r#"{"north":2}"#, true),
    ("Ballasted", r#"{"north":"x"}"#, false),
];

/// Payloads of the models keyed by a date, each with whether serde reads it.
#[cfg(feature = "chrono")]
const DATED_PAYLOADS: [(&str, &str, bool); 6] = [
    ("Tidemarks", r#"{"2026-10-21":2}"#, true),
    ("Tidemarks", r#"{"x":2}"#, false),
    ("Tidemarks", r#"{"2026-10-21":"x"}"#, false),
    ("Tidedays", r#"{"2026-10-21":2}"#, true),
    ("Tidedays", r#"{"x":2}"#, false),
    ("Tidedays", r#"{"2026-10-21":"x"}"#, false),
];

/// Each model keyed by a date, beside its JSON Schema and serde's read of a payload as it.
#[cfg(feature = "chrono")]
const DATED: [(&str, Written, ReadAs); 2] = [
    ("Tidemarks", Tidemarks::json_schema, reads::<Tidemarks>),
    ("Tidedays", Tidedays::json_schema, reads::<Tidedays>),
];

/// Each model a payload is read as, beside its JSON Schema.
const DOCUMENTS: [(&str, Written); 21] = [
    ("Thwart", Thwart::json_schema),
    ("Gunwale", Gunwale::json_schema),
    ("Strake", Strake::json_schema),
    ("Coaming", Coaming::json_schema),
    ("Transom", Transom::json_schema),
    ("Bilge", Bilge::json_schema),
    ("Locker", Locker::json_schema),
    ("Scupper", Scupper::json_schema),
    ("Windlass", Windlass::json_schema),
    ("Ratlines", Ratlines::json_schema),
    ("Deadeye", Deadeye::json_schema),
    ("Sheave", Sheave::json_schema),
    ("Lazarette", Lazarette::json_schema),
    ("Pintle", Pintle::json_schema),
    ("Thole", Thole::json_schema),
    ("Stanchion", Stanchion::json_schema),
    ("Fairlead", Fairlead::json_schema),
    ("Davit", Davit::json_schema),
    ("Capstan", Capstan::json_schema),
    ("Bitt", Bitt::json_schema),
    ("Ballasted", Ballasted::json_schema),
];

/// Each model declared here, beside serde's read of a payload as it.
const READS_HERE: [(&str, ReadAs); 2] =
    [("Bitt", reads::<Bitt>), ("Ballasted", reads::<Ballasted>)];

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

/// A map keyed by a date, which serde reads from the text of each key.
#[cfg(feature = "chrono")]
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Tidemarks {
    #[serde(flatten)]
    pub entries: BTreeMap<chrono::NaiveDate, u8>,
}

/// A brand over a date, which keys [`Tidedays`].
#[cfg(feature = "chrono")]
#[model_schema()]
#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
pub struct Tideday(pub chrono::NaiveDate);

/// A map keyed by a brand over a date, which serde reads from the text of each key.
#[cfg(feature = "chrono")]
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Tidedays {
    #[serde(flatten)]
    pub entries: BTreeMap<Tideday, u8>,
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
fn a_map_keyed_by_an_enum_or_through_an_alias_admits_what_serde_reads() {
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

#[cfg(feature = "chrono")]
#[test]
fn a_map_keyed_by_a_date_or_a_brand_over_one_admits_what_serde_reads() {
    for (model, json, read) in DATED_PAYLOADS {
        let (_, written, reads_as) = DATED.iter().find(|(name, ..)| *name == model).unwrap();
        assert_eq!(reads_as(json), read, "serde on {model} {json}");
        assert_eq!(
            admits(&written(), &serde_json::from_str(json).unwrap()),
            read,
            "the JSON Schema of {model} on {json}: {}",
            written()
        );
    }
}

/// A map keyed by a `char` names its keys by the description of a `char`.
#[test]
fn a_map_keyed_by_a_char_names_its_keys() {
    assert_eq!(
        Ratlines::json_schema(),
        json!({
            "type": "object",
            "properties": {},
            "required": [],
            "additionalProperties": { "type": "integer" },
            "propertyNames": { "type": "string", "minLength": 1_u8, "maxLength": 1_u8 },
        })
    );
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

/// An optional map is the map, or, where serde reads it as `None`, every key with any value.
#[test]
fn an_optional_map_is_the_map_or_every_key() {
    assert_eq!(
        Locker::json_schema(),
        json!({
            "type": "object",
            "anyOf": [
                {
                    "type": "object",
                    "properties": {},
                    "required": [],
                    "additionalProperties": { "type": "integer" },
                },
                {
                    "type": "object",
                    "properties": {},
                    "required": [],
                    "additionalProperties": true,
                },
            ],
        })
    );
}

/// serde reads a flattened `Option` of a struct as `None` where its keys do not read, and drops
/// them; the JSON Schema refuses that payload, and the README states both.
#[test]
fn serde_reads_as_none_an_optional_flattened_struct_the_json_schema_refuses() {
    let read: Hawser = serde_json::from_str(r#"{"label":3}"#).unwrap();
    assert!(read.burgee.is_none(), "serde reads {read:?}");
    assert_eq!(
        serde_json::to_string(&read).unwrap(),
        "{}",
        "serde writes {read:?} back"
    );
    let document = Hawser::json_schema();
    assert!(
        !admits(&document, &json!({ "label": 3_u8 })),
        "the JSON Schema admits a label of a number: {document}"
    );
    assert!(
        admits(&document, &json!({ "label": "a" })),
        "the JSON Schema refuses a label: {document}"
    );
    assert!(
        admits(&document, &json!({})),
        "the JSON Schema refuses no key: {document}"
    );
    let example = "reads `{\"label\":\"a\"}` with `burgee: Some(Burgee { label: \"a\" })`, and \
                   `{\"label\":3}` with `burgee: None`, which it writes back as `{}`";
    assert!(
        include_str!("../../README.md").contains(example),
        "the README no longer states: {example}"
    );
}
