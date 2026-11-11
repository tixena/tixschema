//! A strict struct that flattens a member read through a `preprocess`: the functions are handed
//! every key still left, and whether the member refuses a key no member declares depends on what
//! they answer.

use serde::{Deserialize, Serialize};
use tixschema::model_schema;

#[model_schema()]
#[derive(Debug, Deserialize, Serialize)]
pub struct Version {
    pub major: u32,
}

fn as_given(payload: serde_json::Value) -> serde_json::Value {
    payload
}

#[model_schema()]
#[derive(Debug, Deserialize, Serialize)]
pub struct Release {
    pub id: String,
    #[model_schema_prop(preprocess = ["asGiven"])]
    #[serde(flatten)]
    pub version: Version,
}

fn main() {}
