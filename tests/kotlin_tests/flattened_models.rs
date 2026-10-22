//! The models of the flattened Kotlin reads `kotlin_tests/tests.rs` pins and
//! `service_schema_emitted_client_tests/run_kotlin_flattened.rs` runs: a map flattened as the one
//! member of its struct, a flattened struct beside a member of its own, and an optional member
//! beside a flattened one, with its twin under `nullable`. Beside them, a flattened member after
//! a struct and beside a member of its holder's own. serde reads the members in the order
//! declared: a struct takes the keys it reads, one with a member serde never writes among them, a
//! member that names a function takes none, and every flattened member is handed the entries
//! neither its holder's own members nor a member flattened ahead of it took. An externally tagged
//! enum takes the first entry under the name of a variant it reads, beside a member of its
//! holder's own, ahead of a member that names a function and ahead of a struct, and one declared
//! below its holder is read from the object. serde writes such an enum's unit variant as an entry,
//! its name over `null`, the enum optional and under a function too. An optional map, the one
//! member, serde reads as `None` wherever an entry does not read as a count, and under a function
//! from what the function answers.

use alloc::collections::BTreeMap;
use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use tixschema::model_schema;

use super::tuple_variants_models::{Labelling, labelling_schema};

/// What every holder of a member flattened after or beside another below is read from, but those
/// named apart.
pub const PAYLOAD: &str = r#"{"label":"n","north":2}"#;

/// What [`Hitching`] is read from.
pub const HITCHING: &str = r#"{"label":"n","north":2,"note":"a"}"#;

/// What [`Stropping`] is read from.
pub const STROPPING: &str = r#"{"label":"n","note":"a"}"#;

/// What [`Stowing`] is read from: a variant's entry ahead of the holder's own key and after it,
/// beside a key no member reads and beside a second variant's entry, a unit variant's and a
/// newtype variant's entry, and no variant's entry, which serde refuses.
pub const STOWING: [&str; 7] = [
    r#"{"Bay":{"label":"n"},"note":"a"}"#,
    r#"{"note":"a","Bay":{"label":"n"}}"#,
    r#"{"note":"a","Bay":{"label":"n"},"north":2}"#,
    r#"{"Bay":{"label":"n"},"Dock":{"label":"s"},"note":"a"}"#,
    r#"{"Calm":null,"note":"a"}"#,
    r#"{"Held":"x","note":"a"}"#,
    r#"{"note":"a"}"#,
];

/// What [`Belting`] is read from: a variant's entry, and no variant's entry, of a key no variant is
/// read under and of none, where serde reads no value.
pub const BELTING: [&str; 3] = [
    r#"{"Bay":{"label":"n"},"note":"a"}"#,
    r#"{"Other":1,"note":"a"}"#,
    r#"{"note":"a"}"#,
];

/// What [`Heaving`] is read from: one variant's entry, two variants' entries in each order, and an
/// alias after a variant's entry.
pub const HEAVING: [&str; 4] = [
    r#"{"Bay":{"label":"n"},"north":2}"#,
    r#"{"Bay":{"label":"n"},"Dock":{"label":"s"},"north":2}"#,
    r#"{"Dock":{"label":"s"},"Bay":{"label":"n"},"north":2}"#,
    r#"{"Bay":{"label":"n"},"Quay":{"label":"s"},"north":2}"#,
];

/// What [`Cleating`] is read from: a variant under its `rename`, one under the enum's `rename_all`,
/// and one under its Rust name, which serde reads no variant under.
pub const CLEATING: [&str; 3] = [
    r#"{"pier":{"label":"n"},"north":2}"#,
    r#"{"open_bay":{"label":"n"},"north":2}"#,
    r#"{"OpenBay":{"label":"n"},"north":2}"#,
];

/// What [`Rousing`] and [`Gasketing`] are read from: the key of the member [`Unwritten`] reads and
/// never writes among them.
pub const UNWRITTEN: &str = r#"{"label":"n","hidden":1,"north":2}"#;

/// What [`Scudding`] is read from.
pub const SCUDDING: &str = r#"{"Bay":{"label":"n"},"label":"m","north":2}"#;

/// A variant's entry under its `alias`, beside a member of its holder's own and ahead of a member
/// that names a function, which serde reads as the variant.
pub const UNDER_AN_ALIAS: [&str; 2] = [
    r#"{"Quay":{"label":"s"},"note":"a"}"#,
    r#"{"Quay":{"label":"s"},"north":2}"#,
];

/// What [`ProbeLocker`] and its twins are read from: no entry, an entry that reads as a count, an
/// entry that does not, the two side by side, and `null`, which a count does not read.
pub const LOCKER: [&str; 5] = [
    "{}",
    r#"{"north":2}"#,
    r#"{"north":"x"}"#,
    r#"{"north":2,"south":"x"}"#,
    r#"{"north":null}"#,
];

/// A map, the one member, which serde hands every key of the object.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Netting {
    #[serde(flatten)]
    pub rest: HashMap<String, u8>,
}

/// An optional map, the one member: serde reads it as `None` wherever an entry it is handed does
/// not read as a count, and writes `None` as no entry.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ProbeLocker {
    #[serde(default, flatten, skip_serializing_if = "Option::is_none")]
    pub counts: Option<BTreeMap<String, u8>>,
}

/// [`ProbeLocker`] under a function that answers the payload as it was handed: serde reads the
/// map from what it answers, and refuses an entry that does not read as a count.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CorrectedLocker {
    #[model_schema_prop(preprocess = ["asGiven"])]
    #[serde(default, flatten, skip_serializing_if = "Option::is_none")]
    pub counts: Option<BTreeMap<String, u8>>,
}

/// [`ProbeLocker`] under a function that answers `null`, which serde reads as `None`.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct EmptiedLocker {
    #[model_schema_prop(preprocess = ["asNull"])]
    #[serde(default, flatten, skip_serializing_if = "Option::is_none")]
    pub counts: Option<BTreeMap<String, u8>>,
}

/// A flattened struct beside a member of its own.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Knotting {
    #[serde(flatten)]
    pub labelling: Labelling,
    pub note: String,
}

/// An optional member beside a flattened one, its key left out where it holds no value.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Splicing {
    #[serde(flatten)]
    pub labelling: Labelling,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spare: Option<u32>,
}

/// [`Splicing`] with its optional member under `nullable`, whose key is always written.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Seizing {
    #[serde(flatten)]
    pub labelling: Labelling,
    #[model_schema_prop(nullable)]
    pub spare: Option<u32>,
}

/// The keys a function was handed, which the members below that name one read.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Bunting {
    pub keys: Vec<String>,
}

/// A member that reads keys [`Labelling`] does not.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Noting {
    pub note: String,
}

/// A member that names a function flattened after a struct: serde hands the function the keys the
/// struct did not take.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Whipping {
    #[serde(flatten)]
    pub labelling: Labelling,
    #[model_schema_prop(preprocess = ["withItsKeys"])]
    #[serde(flatten)]
    pub tally: Bunting,
}

/// A member that names a function flattened after a struct, with a member of its holder's own
/// between them: serde hands the function the keys neither took.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Hitching {
    #[serde(flatten)]
    pub labelling: Labelling,
    pub note: String,
    #[model_schema_prop(preprocess = ["withItsKeys"])]
    #[serde(flatten)]
    pub tally: Bunting,
}

/// A member that names a function flattened ahead of a struct: serde hands the function every key,
/// the struct's among them, and the struct the keys it reads.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Bending {
    #[model_schema_prop(preprocess = ["withItsKeys"])]
    #[serde(flatten)]
    pub extra: Bunting,
    #[serde(flatten)]
    pub labelling: Labelling,
}

/// A struct that names a function on itself, which serde reads as a value and so takes no key.
#[model_schema(preprocess = ["asGiven"])]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Ticketed {
    pub label: String,
}

/// A member that names a function flattened after [`Ticketed`]: serde hands the function every
/// key.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Lanyarding {
    #[serde(flatten)]
    pub issued: Ticketed,
    #[model_schema_prop(preprocess = ["withItsKeys"])]
    #[serde(flatten)]
    pub tally: Bunting,
}

/// A struct with a flattened member of its own, which serde reads as a map and so takes no key.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Sheeting {
    #[serde(flatten)]
    pub labelling: Labelling,
}

/// A member that names a function flattened after [`Sheeting`]: serde hands the function every
/// key.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Brailing {
    #[serde(flatten)]
    pub sheeting: Sheeting,
    #[model_schema_prop(preprocess = ["withItsKeys"])]
    #[serde(flatten)]
    pub tally: Bunting,
}

/// A struct with a member serde reads and never writes, which takes the keys it reads.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Unwritten {
    #[serde(default, skip_serializing)]
    pub hidden: u8,
    pub label: String,
}

/// A member that names a function flattened after [`Unwritten`]: serde hands the function the keys
/// the struct did not take.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Rousing {
    #[serde(flatten)]
    pub hauled: Unwritten,
    #[model_schema_prop(preprocess = ["withItsKeys"])]
    #[serde(flatten)]
    pub tally: Bunting,
}

/// A struct flattened after [`Unwritten`] that reads a key it took: serde hands the second the
/// object without it and refuses the payload.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Gasketing {
    #[serde(flatten)]
    pub hauled: Unwritten,
    #[serde(flatten)]
    pub labelling: Labelling,
}

/// A struct flattened after a struct that reads other keys, which serde reads.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Stropping {
    #[serde(flatten)]
    pub labelling: Labelling,
    #[serde(flatten)]
    pub noting: Noting,
}

/// A struct flattened after a struct that reads the same key: serde hands the second the object
/// without it and refuses the payload.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Stoppering {
    #[serde(flatten)]
    pub labelling: Labelling,
    #[serde(flatten)]
    pub twice: Labelling,
}

/// A struct flattened beside a member of its holder's own that reads the same key: serde hands the
/// struct the object without it and refuses the payload.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Nipping {
    pub label: String,
    #[serde(flatten)]
    pub labelling: Labelling,
}

/// An optional struct flattened after a struct that reads the same key: serde hands the second
/// the object without it and reads no value.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Slinging {
    #[serde(flatten)]
    pub labelling: Labelling,
    #[serde(flatten, skip_serializing_if = "Option::is_none")]
    pub spare: Option<Labelling>,
}

/// An externally tagged enum, which serde reads from the first entry under a variant's name or an
/// alias of one.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum Tacking {
    Bay {
        label: String,
    },
    Calm,
    #[serde(alias = "Quay")]
    Dock {
        label: String,
    },
    Held(String),
}

/// An externally tagged enum flattened beside a member of its holder's own: serde hands the enum
/// the first entry under the name of a variant, and leaves every other key to the holder.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Stowing {
    pub note: String,
    #[serde(flatten)]
    pub tacking: Tacking,
}

/// [`Stowing`] with its enum optional, which serde reads no value of where no entry is under the
/// name of a variant.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Belting {
    pub note: String,
    #[serde(flatten, skip_serializing_if = "Option::is_none")]
    pub tacking: Option<Tacking>,
}

/// [`Stowing`] with its enum under a function that answers the payload as it was handed, which
/// serde writes as it writes [`Stowing`]'s.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Yawing {
    pub note: String,
    #[model_schema_prop(preprocess = ["asGiven"])]
    #[serde(flatten)]
    pub tacking: Tacking,
}

/// A member that names a function flattened after an externally tagged enum, where serde hands
/// the function every entry but the one the enum took.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Heaving {
    #[serde(flatten)]
    pub tacking: Tacking,
    #[model_schema_prop(preprocess = ["withItsKeys"])]
    #[serde(flatten)]
    pub tally: Bunting,
}

/// An externally tagged enum whose variants are written under the enum's `rename_all` and under a
/// `rename` of their own.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Jibing {
    #[serde(rename = "pier")]
    Jetty {
        label: String,
    },
    OpenBay {
        label: String,
    },
}

/// A member that names a function flattened after [`Jibing`].
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Cleating {
    #[serde(flatten)]
    pub jibing: Jibing,
    #[model_schema_prop(preprocess = ["withItsKeys"])]
    #[serde(flatten)]
    pub tally: Bunting,
}

/// A struct and a member that names a function flattened after an externally tagged enum, where
/// serde hands the struct every entry but the enum's, and the function every entry neither took.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Scudding {
    #[serde(flatten)]
    pub hatch: Tacking,
    #[serde(flatten)]
    pub labelling: Labelling,
    #[model_schema_prop(preprocess = ["withItsKeys"])]
    #[serde(flatten)]
    pub tally: Bunting,
}

/// An externally tagged enum flattened beside a member of its holder's own and declared below the
/// holder, which the emitter has not written when it writes the holder.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Luffing {
    pub note: String,
    #[serde(flatten)]
    pub quaying: Quaying,
}

/// The enum [`Luffing`] flattens.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum Quaying {
    Bay { label: String },
}

/// The payload as it was handed.
const fn as_given(payload: Value) -> Value {
    payload
}

/// `null`, whatever `payload` is.
fn as_null(_payload: Value) -> Value {
    Value::Null
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

/// What serde reads of `payload` as `T`, written back, or `refused` where it refuses it.
pub fn read_of<T>(payload: &str) -> String
where
    T: for<'de> Deserialize<'de> + Serialize,
{
    serde_json::from_str::<T>(payload).map_or_else(
        |_| "refused".to_owned(),
        |read| serde_json::to_string(&read).unwrap(),
    )
}
