//! The models `run_swift_payload_floats.rs` reads a number written as a float on, through the
//! payload of a `preprocess` and of a flattened member: an integer as a member, a list's item and
//! a float member, beside a function that turns an integer into a float and one that turns a
//! float into an integer. The members of each are declared in the order of their names, the order
//! Swift's encoder writes them in when it sorts them.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use tixschema::model_schema;

/// An integer, a list of them and a float, read through the function of the type.
#[model_schema(preprocess = ["kept"])]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Cargo {
    pub count: u32,
    pub counts: Vec<u32>,
    pub ratio: f64,
}

/// An integer and a float, with no function.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Ballast {
    pub count: u32,
    pub ratio: f64,
}

/// [`Ballast`] flattened, with no function on the member.
#[model_schema(nonstrict)]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Hold {
    #[serde(flatten)]
    pub ballast: Ballast,
    pub name: String,
}

/// [`Ballast`] flattened, with a function on the member.
#[model_schema(nonstrict)]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Stowage {
    #[serde(flatten)]
    #[model_schema_prop(preprocess = ["kept"])]
    pub ballast: Ballast,
    pub name: String,
}

/// An integer whose function answers a float.
#[model_schema(preprocess = ["widened"])]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Widened {
    pub count: u32,
}

/// A float whose function answers an integer.
#[model_schema(preprocess = ["narrowed"])]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Narrowed {
    pub ratio: f64,
}

/// A float member.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Inner {
    pub b: f64,
}

/// An integer under a key that holds a dot, beside a float at the path the dot would write.
#[model_schema(preprocess = ["kept"])]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Crossed {
    pub a: Inner,
    #[serde(rename = "a.b")]
    pub dotted: u32,
}

/// The payload as it is.
pub fn kept(payload: Value) -> Value {
    payload
}

/// The payload with a `count` of `4` answered as the float `2.0`.
pub fn widened(mut payload: Value) -> Value {
    if let Some(count) = payload.get_mut("count")
        && *count == 4_u32
    {
        *count = Value::from(2.0_f64);
    }
    payload
}

/// The payload with a `ratio` of `5.5` answered as the integer `3`.
pub fn narrowed(mut payload: Value) -> Value {
    if let Some(ratio) = payload.get_mut("ratio")
        && *ratio == 5.5_f64
    {
        *ratio = Value::from(3_u32);
    }
    payload
}
