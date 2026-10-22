//! The models of the flattened Dart reads `dart_tests/flattened.rs` pins and
//! `service_schema_emitted_client_tests/run_dart_flattened.rs` runs: a map that is the one member
//! of its struct; and a member that names a function, which is handed the keys serde hands it,
//! after a struct, beside a member of its holder's own and in a variant under a tag key, ahead of a
//! struct, after each member that takes none of the keys it reads, and after an externally tagged
//! enum, an enum under a tag key and an untagged one. serde reads the members in the order
//! declared: a struct takes the keys it reads, an externally tagged enum the first entry under the
//! name of a variant it reads, and every flattened member is handed every entry no member ahead of
//! it took. A struct after a struct that reads the same key is refused.

use alloc::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use tixschema::model_schema;

/// What every struct below but those named apart is read from.
pub const PAYLOAD: &str = r#"{"label":"n","north":2}"#;

/// What [`Spare`] is read from.
pub const SPARE: &str = r#"{"north":2,"south":3}"#;

/// What [`Clamping`] is read from.
pub const CLAMPING: &str = r#"{"label":"n","north":2,"note":"a"}"#;

/// What [`Fastening`] is read from.
pub const FASTENING: &str = r#"{"north":2,"note":"a"}"#;

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

/// The keys a function was handed, which the members below that name one read.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Keying {
    pub keys: Vec<String>,
}

/// A map, the one member of its struct, which serde hands every key.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Spare {
    #[serde(flatten)]
    pub rest: BTreeMap<String, u8>,
}

/// A member that names a function flattened after a struct, with a member of its holder's own
/// between them: serde hands the function the keys neither took.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Clamping {
    #[serde(flatten)]
    pub marking: Marking,
    pub note: String,
    #[model_schema_prop(preprocess = ["withItsKeys"])]
    #[serde(flatten)]
    pub other: Keying,
}

/// A member that names a function, the one flattened member, beside a member of its holder's own.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Fastening {
    pub note: String,
    #[model_schema_prop(preprocess = ["withItsKeys"])]
    #[serde(flatten)]
    pub other: Keying,
}

/// A member that names a function flattened ahead of a struct, where serde hands the function
/// every key, the struct's among them.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Hoisting {
    #[model_schema_prop(preprocess = ["withItsKeys"])]
    #[serde(flatten)]
    pub extra: Keying,
    #[serde(flatten)]
    pub marking: Marking,
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

/// A member that names a function flattened after a struct that names one on itself, where serde
/// hands the function every key.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Lacing {
    #[serde(flatten)]
    pub issued: Ticketing,
    #[model_schema_prop(preprocess = ["withItsKeys"])]
    #[serde(flatten)]
    pub rest: Keying,
}

/// A member that names a function flattened after another that names one, where serde hands the
/// second every key.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Rigging {
    #[model_schema_prop(preprocess = ["asGiven"])]
    #[serde(flatten)]
    pub marking: Marking,
    #[model_schema_prop(preprocess = ["withItsKeys"])]
    #[serde(flatten)]
    pub rest: Keying,
}

/// A member that names a function flattened after a struct with a flattened member of its own,
/// where serde hands the function every key.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Belaying {
    #[serde(flatten)]
    pub bracing: Bracing,
    #[model_schema_prop(preprocess = ["withItsKeys"])]
    #[serde(flatten)]
    pub rest: Keying,
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

/// A member that names a function flattened after an externally tagged enum, where serde hands
/// the function every entry but the one the enum took.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Loading {
    #[serde(flatten)]
    pub hatching: Hatching,
    #[model_schema_prop(preprocess = ["withItsKeys"])]
    #[serde(flatten)]
    pub rest: Keying,
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

/// A member that names a function flattened after [`Warping`].
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Kedging {
    #[serde(flatten)]
    pub kedge: Warping,
    #[model_schema_prop(preprocess = ["withItsKeys"])]
    #[serde(flatten)]
    pub rest: Keying,
}

/// A struct and a member that names a function flattened after an externally tagged enum, where
/// serde hands the struct every entry but the enum's, and the function every entry neither took.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Trimming {
    #[serde(flatten)]
    pub hatching: Hatching,
    #[serde(flatten)]
    pub marking: Marking,
    #[model_schema_prop(preprocess = ["withItsKeys"])]
    #[serde(flatten)]
    pub rest: Keying,
}

/// An enum under a tag key alone, which serde reads as a map and so takes no key.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind")]
pub enum Frapping {
    Rope { label: String },
}

/// A member that names a function flattened after [`Frapping`], where serde hands the function
/// every key.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Mousing {
    #[serde(flatten)]
    pub frapping: Frapping,
    #[model_schema_prop(preprocess = ["withItsKeys"])]
    #[serde(flatten)]
    pub rest: Keying,
}

/// An untagged enum, which serde reads as a map and so takes no key.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(untagged)]
pub enum Chocking {
    Rope { label: String },
}

/// A member that names a function flattened after [`Chocking`], where serde hands the function
/// every key.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Shoring {
    #[serde(flatten)]
    pub chocking: Chocking,
    #[model_schema_prop(preprocess = ["withItsKeys"])]
    #[serde(flatten)]
    pub rest: Keying,
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

/// A map that is the one member of its struct holds every key.
#[test]
fn serde_hands_a_map_alone_every_key() {
    let spare: Spare = serde_json::from_str(SPARE).unwrap();
    assert_eq!(written(&spare.rest), SPARE);
}

/// A function after a struct is handed the keys neither the struct nor its holder took, and one
/// ahead of a struct, and after a member that takes no key, every key but its holder's own.
#[test]
fn serde_hands_a_function_the_keys_no_member_ahead_of_it_took() {
    let clamping: Clamping = serde_json::from_str(CLAMPING).unwrap();
    assert_eq!(clamping.other.keys, ["north"]);
    let fastening: Fastening = serde_json::from_str(FASTENING).unwrap();
    assert_eq!(fastening.other.keys, ["north"]);
    let hoisting: Hoisting = serde_json::from_str(PAYLOAD).unwrap();
    assert_eq!(hoisting.extra.keys, ["label", "north"]);
    assert_eq!(hoisting.marking.label, "n");
    let lacing: Lacing = serde_json::from_str(PAYLOAD).unwrap();
    assert_eq!(lacing.rest.keys, ["label", "north"]);
    let rigging: Rigging = serde_json::from_str(PAYLOAD).unwrap();
    assert_eq!(rigging.rest.keys, ["label", "north"]);
    let belaying: Belaying = serde_json::from_str(PAYLOAD).unwrap();
    assert_eq!(belaying.rest.keys, ["label", "north"]);
    let twin: Twin = serde_json::from_str(PAYLOAD).unwrap();
    assert_eq!(twin.other.keys, ["north"]);
    assert_eq!(written(&twin), r#"{"label":"n","keys":["north"]}"#);
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

/// What a function after an externally tagged enum is handed of each payload, and the variant the
/// enum read: serde takes the first entry under a variant's name or an alias of one, never one
/// under the variant's Rust name, and hands the function every other entry.
#[test]
fn serde_hands_a_function_after_an_externally_tagged_enum_every_entry_but_the_variants() {
    let loaded: Vec<String> = LOADING
        .iter()
        .map(|payload| {
            read_of(payload, |loading: &Loading| {
                format!("{} {:?}", written(&loading.hatching), loading.rest.keys)
            })
        })
        .collect();
    assert_eq!(
        loaded,
        [
            r#"{"Bay":{"label":"n"}} ["north"]"#,
            r#"{"Bay":{"label":"n"}} ["Dock", "north"]"#,
            r#"{"Dock":{"label":"s"}} ["Bay", "north"]"#,
            r#"{"Dock":{"label":"s"}} ["north"]"#,
            r#"{"Bay":{"label":"n"}} ["Quay", "north"]"#,
        ]
    );
    let kedged: Vec<String> = KEDGING
        .iter()
        .map(|payload| {
            read_of(payload, |kedging: &Kedging| {
                format!("{} {:?}", written(&kedging.kedge), kedging.rest.keys)
            })
        })
        .collect();
    assert_eq!(
        kedged,
        [
            r#"{"pier":{"label":"n"}} ["north"]"#,
            r#"{"open_bay":{"label":"n"}} ["north"]"#,
            "refused",
        ]
    );
    let trimming: Trimming = serde_json::from_str(TRIMMING).unwrap();
    assert_eq!(trimming.marking.label, "m");
    assert_eq!(trimming.rest.keys, ["north"]);
}

/// A function after an enum under a tag key alone, or after an untagged one, is handed every key.
#[test]
fn serde_hands_a_function_after_an_enum_read_as_a_map_every_key() {
    let mousing: Mousing = serde_json::from_str(MOUSING).unwrap();
    assert_eq!(mousing.rest.keys, ["kind", "label", "north"]);
    let shoring: Shoring = serde_json::from_str(PAYLOAD).unwrap();
    assert_eq!(shoring.rest.keys, ["label", "north"]);
}
