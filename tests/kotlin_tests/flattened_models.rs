//! The models of the flattened Kotlin reads `kotlin_tests/tests.rs` pins and
//! `service_schema_emitted_client_tests/run_kotlin_flattened.rs` runs: a map flattened as the one
//! member of its struct, a flattened struct beside a member of its own, and an optional member
//! beside a flattened one, with its twin under `nullable`. Beside them, a flattened member after
//! a struct and beside a member of its holder's own. serde reads the members in the order
//! declared: a struct takes the keys it reads, a member that names a function takes none, and
//! every flattened member is handed the entries neither its holder's own members nor a member
//! flattened ahead of it took.

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

/// A map, the one member, which serde hands every key of the object.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Netting {
    #[serde(flatten)]
    pub rest: HashMap<String, u8>,
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
