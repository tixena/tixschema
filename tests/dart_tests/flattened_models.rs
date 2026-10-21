//! The models of the flattened Dart reads `dart_tests/flattened.rs` pins and
//! `service_schema_emitted_client_tests/run_dart_flattened.rs` runs: a map flattened after a
//! struct, beside a member of its holder's own and in a variant under a tag key, a map that is the
//! one flattened member, a map flattened ahead of a struct, two flattened maps, a map flattened
//! after a member that takes none of the keys it reads, and, after a struct, a struct with a
//! flattened map, a member that names a function, a JSON value and a struct that reads the same
//! key; and a map flattened after an externally tagged enum, after an enum under a tag key and
//! after an untagged one. serde reads the members in the order declared: a struct takes the keys
//! it reads, an externally tagged enum the first entry under the name of a variant it reads, and
//! every flattened member is handed every entry no member ahead of it took.

use alloc::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use tixschema::model_schema;

/// What every other struct is read from.
pub const PAYLOAD: &str = r#"{"label":"n","north":2}"#;

/// What [`Clamping`] is read from.
pub const CLAMPING: &str = r#"{"label":"n","north":2,"note":"a"}"#;

/// What [`Fastening`] is read from.
pub const FASTENING: &str = r#"{"north":2,"note":"a"}"#;

/// What [`Tethering`] is read from.
pub const TETHERING: &str = r#"{"kind":"Held","label":"n","north":2}"#;

/// What [`Stowing`] is read from.
pub const STOWING: &str = r#"{"north":2,"south":3}"#;

/// What [`Loading`] is read from: one variant's entry, two variants' entries in each order, a
/// variant's entry under its alias, and an alias after a variant's entry.
pub const LOADING: [&str; 5] = [
    r#"{"Bay":{"label":"n"},"north":2}"#,
    r#"{"Bay":{"label":"n"},"Dock":{"label":"s"},"north":2}"#,
    r#"{"Dock":{"label":"s"},"Bay":{"label":"n"},"north":2}"#,
    r#"{"Quay":{"label":"s"},"north":2}"#,
    r#"{"Bay":{"label":"n"},"Quay":{"label":"s"},"north":2}"#,
];

/// What [`Kedging`] is read from: a variant under its `rename`, one under the enum's `rename_all`,
/// and one under its Rust name, which serde reads no variant under.
pub const KEDGING: [&str; 3] = [
    r#"{"pier":{"label":"n"},"north":2}"#,
    r#"{"open_bay":{"label":"n"},"north":2}"#,
    r#"{"OpenBay":{"label":"n"},"north":2}"#,
];

/// What [`Trimming`] is read from.
pub const TRIMMING: &str = r#"{"Bay":{"label":"n"},"label":"m","north":2}"#;

/// What [`Mousing`] is read from.
pub const MOUSING: &str = r#"{"kind":"Rope","label":"n","north":2}"#;

/// The member the structs below flatten.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Marking {
    pub label: String,
}

/// A map flattened after a struct, where serde hands it the keys the struct did not take.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Coupling {
    #[serde(flatten)]
    pub marking: Marking,
    #[serde(flatten)]
    pub rest: BTreeMap<String, u8>,
}

/// [`Coupling`] with a member of its own between the two flattened ones.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Clamping {
    #[serde(flatten)]
    pub marking: Marking,
    pub note: String,
    #[serde(flatten)]
    pub rest: BTreeMap<String, u8>,
}

/// A map, the one flattened member, beside a member of its holder's own.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Fastening {
    pub note: String,
    #[serde(flatten)]
    pub rest: BTreeMap<String, u8>,
}

/// A map flattened ahead of a struct, where serde hands it every key, the struct's among them.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Hoisting {
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
    #[serde(flatten)]
    pub marking: Marking,
}

/// Two flattened maps, where serde hands each the same keys.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Stowing {
    #[serde(flatten)]
    pub first: BTreeMap<String, u8>,
    #[serde(flatten)]
    pub second: BTreeMap<String, u8>,
}

/// A struct that names a function on itself, which serde reads as a value.
#[model_schema(preprocess = ["asGiven"])]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Ticketing {
    pub label: String,
}

/// A struct with a flattened member of its own, which serde reads as a map.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Bracing {
    #[serde(flatten)]
    pub marking: Marking,
}

/// A map flattened after a struct that names a function on itself, where serde hands it every key.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Lacing {
    #[serde(flatten)]
    pub issued: Ticketing,
    #[serde(flatten)]
    pub rest: BTreeMap<String, Value>,
}

/// A map flattened after a member that names a function, where serde hands it every key.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Rigging {
    #[model_schema_prop(preprocess = ["asGiven"])]
    #[serde(flatten)]
    pub marking: Marking,
    #[serde(flatten)]
    pub rest: BTreeMap<String, Value>,
}

/// A map flattened after a struct with a flattened member of its own, where serde hands it every
/// key.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Belaying {
    #[serde(flatten)]
    pub bracing: Bracing,
    #[serde(flatten)]
    pub rest: BTreeMap<String, Value>,
}

/// A map flattened after a struct in a variant under a tag key, where serde hands it the keys
/// neither the tag nor the struct took.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind")]
pub enum Tethering {
    Held {
        #[serde(flatten)]
        marking: Marking,
        #[serde(flatten)]
        rest: BTreeMap<String, u8>,
    },
}

/// A struct with a flattened map of its own, which serde reads as a map and so takes no key.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Spare {
    #[serde(flatten)]
    pub rest: BTreeMap<String, Value>,
}

/// A struct flattened after a struct, where serde hands it the keys the first did not take.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Pairing {
    #[serde(flatten)]
    pub marking: Marking,
    #[serde(flatten)]
    pub spare: Spare,
}

/// The keys a function was handed, which [`Twin`] reads its member from.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Keying {
    pub keys: Vec<String>,
}

/// A member that names a function flattened after a struct, where serde hands the function the
/// keys the struct did not take.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Twin {
    #[serde(flatten)]
    pub marking: Marking,
    #[model_schema_prop(preprocess = ["withItsKeys"])]
    #[serde(flatten)]
    pub other: Keying,
}

/// A JSON value flattened after a struct, which serde reads as a map of the keys the struct did not
/// take.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Loosening {
    #[serde(flatten)]
    pub marking: Marking,
    #[serde(flatten)]
    pub rest: Value,
}

/// A struct flattened after a struct that reads the same key, where serde hands the second the
/// object without it and refuses the payload.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Doubling {
    #[serde(flatten)]
    pub marking: Marking,
    #[serde(flatten)]
    pub twice: Marking,
}

/// An externally tagged enum, which serde reads from the first entry under a variant's name or an
/// alias of one.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum Hatching {
    Bay {
        label: String,
    },
    #[serde(alias = "Quay")]
    Dock {
        label: String,
    },
}

/// A map flattened after an externally tagged enum, where serde hands it every entry but the one
/// the enum took.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Loading {
    #[serde(flatten)]
    pub hatching: Hatching,
    #[serde(flatten)]
    pub rest: BTreeMap<String, Value>,
}

/// An externally tagged enum whose variants are written under the enum's `rename_all` and under a
/// `rename` of their own.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Warping {
    #[serde(rename = "pier")]
    Jetty {
        label: String,
    },
    OpenBay {
        label: String,
    },
}

/// A map flattened after [`Warping`].
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Kedging {
    #[serde(flatten)]
    pub kedge: Warping,
    #[serde(flatten)]
    pub rest: BTreeMap<String, Value>,
}

/// A struct and a map flattened after an externally tagged enum, where serde hands the struct
/// every entry but the enum's, and the map every entry neither took.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Trimming {
    #[serde(flatten)]
    pub hatching: Hatching,
    #[serde(flatten)]
    pub marking: Marking,
    #[serde(flatten)]
    pub rest: BTreeMap<String, Value>,
}

/// An enum under a tag key alone, which serde reads as a map and so takes no key.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind")]
pub enum Frapping {
    Rope { label: String },
}

/// A map flattened after [`Frapping`], where serde hands it every key.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Mousing {
    #[serde(flatten)]
    pub frapping: Frapping,
    #[serde(flatten)]
    pub rest: BTreeMap<String, Value>,
}

/// An untagged enum, which serde reads as a map and so takes no key.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(untagged)]
pub enum Chocking {
    Rope { label: String },
}

/// A map flattened after [`Chocking`], where serde hands it every key.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Shoring {
    #[serde(flatten)]
    pub chocking: Chocking,
    #[serde(flatten)]
    pub rest: BTreeMap<String, Value>,
}

/// The payload as it was handed.
const fn as_given(payload: Value) -> Value {
    payload
}

/// The keys of the object `payload`, under `keys`.
fn with_its_keys(payload: Value) -> Value {
    let Value::Object(object) = payload else {
        return payload;
    };
    let keys: Vec<Value> = object
        .into_iter()
        .map(|(key, _)| Value::String(key))
        .collect();
    serde_json::json!({ "keys": keys })
}

/// The text of `value` as serde writes it.
fn written<T>(value: &T) -> String
where
    T: Serialize,
{
    serde_json::to_string(value).unwrap()
}

/// A map after a struct holds the keys the struct did not take, and a map ahead of a struct, and
/// after a member that takes no key, holds every key. serde writes a key two members hold twice.
#[test]
fn serde_hands_a_flattened_map_the_keys_no_member_ahead_of_it_took() {
    let coupling: Coupling = serde_json::from_str(PAYLOAD).unwrap();
    assert_eq!(written(&coupling.rest), r#"{"north":2}"#);
    let clamping: Clamping = serde_json::from_str(CLAMPING).unwrap();
    assert_eq!(written(&clamping.rest), r#"{"north":2}"#);
    let fastening: Fastening = serde_json::from_str(FASTENING).unwrap();
    assert_eq!(written(&fastening.rest), r#"{"north":2}"#);
    let Tethering::Held { marking, rest } = serde_json::from_str(TETHERING).unwrap();
    assert_eq!(marking.label, "n");
    assert_eq!(written(&rest), r#"{"north":2}"#);
    let hoisting: Hoisting = serde_json::from_str(PAYLOAD).unwrap();
    assert_eq!(written(&hoisting.extra), PAYLOAD);
    assert_eq!(written(&hoisting), r#"{"label":"n","north":2,"label":"n"}"#);
    let lacing: Lacing = serde_json::from_str(PAYLOAD).unwrap();
    assert_eq!(written(&lacing.rest), PAYLOAD);
    let rigging: Rigging = serde_json::from_str(PAYLOAD).unwrap();
    assert_eq!(written(&rigging.rest), PAYLOAD);
    let belaying: Belaying = serde_json::from_str(PAYLOAD).unwrap();
    assert_eq!(written(&belaying.rest), PAYLOAD);
    let stowing: Stowing = serde_json::from_str(STOWING).unwrap();
    assert_eq!(
        written(&stowing),
        r#"{"north":2,"south":3,"north":2,"south":3}"#
    );
}

/// A struct with a flattened map, a member that names a function and a JSON value, each after a
/// struct, are handed the keys the struct did not take, and a struct after a struct that reads the
/// same key is refused.
#[test]
fn serde_hands_a_flattened_member_the_keys_no_struct_ahead_of_it_took() {
    let pairing: Pairing = serde_json::from_str(PAYLOAD).unwrap();
    assert_eq!(written(&pairing.spare.rest), r#"{"north":2}"#);
    assert_eq!(written(&pairing), PAYLOAD);
    let twin: Twin = serde_json::from_str(PAYLOAD).unwrap();
    assert_eq!(twin.other.keys, ["north"]);
    assert_eq!(written(&twin), r#"{"label":"n","keys":["north"]}"#);
    let loosening: Loosening = serde_json::from_str(PAYLOAD).unwrap();
    assert_eq!(written(&loosening.rest), r#"{"north":2}"#);
    assert_eq!(written(&loosening), PAYLOAD);
    let refused = serde_json::from_str::<Doubling>(PAYLOAD).unwrap_err();
    assert_eq!(
        refused.to_string(),
        "missing field `label` at line 1 column 23"
    );
}

/// What serde reads of `payload` as `T`, through `parts`, or that it refuses it.
fn read_of<T>(payload: &str, parts: fn(&T) -> String) -> String
where
    T: for<'de> Deserialize<'de>,
{
    serde_json::from_str::<T>(payload).map_or_else(|_| "refused".to_owned(), |read| parts(&read))
}

/// What a map after an externally tagged enum holds of each payload, and the variant the enum
/// read: serde takes the first entry under a variant's name or an alias of one, never one under
/// the variant's Rust name, and hands the map every other entry.
#[test]
fn serde_hands_a_map_after_an_externally_tagged_enum_every_entry_but_the_variants() {
    let loaded: Vec<String> = LOADING
        .iter()
        .map(|payload| {
            read_of(payload, |loading: &Loading| {
                format!("{} {}", written(&loading.hatching), written(&loading.rest))
            })
        })
        .collect();
    assert_eq!(
        loaded,
        [
            r#"{"Bay":{"label":"n"}} {"north":2}"#,
            r#"{"Bay":{"label":"n"}} {"Dock":{"label":"s"},"north":2}"#,
            r#"{"Dock":{"label":"s"}} {"Bay":{"label":"n"},"north":2}"#,
            r#"{"Dock":{"label":"s"}} {"north":2}"#,
            r#"{"Bay":{"label":"n"}} {"Quay":{"label":"s"},"north":2}"#,
        ]
    );
    let kedged: Vec<String> = KEDGING
        .iter()
        .map(|payload| {
            read_of(payload, |kedging: &Kedging| {
                format!("{} {}", written(&kedging.kedge), written(&kedging.rest))
            })
        })
        .collect();
    assert_eq!(
        kedged,
        [
            r#"{"pier":{"label":"n"}} {"north":2}"#,
            r#"{"open_bay":{"label":"n"}} {"north":2}"#,
            "refused",
        ]
    );
    let trimming: Trimming = serde_json::from_str(TRIMMING).unwrap();
    assert_eq!(trimming.marking.label, "m");
    assert_eq!(written(&trimming.rest), r#"{"north":2}"#);
}

/// A map after an enum under a tag key alone, or after an untagged one, holds every key.
#[test]
fn serde_hands_a_map_after_an_enum_read_as_a_map_every_key() {
    let mousing: Mousing = serde_json::from_str(MOUSING).unwrap();
    assert_eq!(written(&mousing.rest), MOUSING);
    let shoring: Shoring = serde_json::from_str(PAYLOAD).unwrap();
    assert_eq!(written(&shoring.rest), PAYLOAD);
}
