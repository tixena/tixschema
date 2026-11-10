//! One struct declared below the holder flattened twice, once under an `Option`: both write its keys.

use serde::{Deserialize, Serialize};
use tixschema::model_schema;

#[model_schema()]
#[derive(Debug, Deserialize, Serialize)]
pub struct Parcel {
    pub label: String,
    #[serde(flatten)]
    pub ink: Ink,
    #[serde(flatten)]
    pub spare: Option<Ink>,
}

#[model_schema(nonstrict)]
#[derive(Debug, Deserialize, Serialize)]
pub struct Ink {
    pub hue: String,
    pub depth: u32,
}

fn main() {}
