//! Two flattened structs declared below the holder, both `nonstrict`, that write one key.

use serde::{Deserialize, Serialize};
use tixschema::model_schema;

#[model_schema()]
#[derive(Debug, Deserialize, Serialize)]
pub struct Parcel {
    pub label: String,
    #[serde(flatten)]
    pub ink: Ink,
    #[serde(flatten)]
    pub stamp: Stamp,
}

#[model_schema(nonstrict)]
#[derive(Debug, Deserialize, Serialize)]
pub struct Ink {
    pub hue: String,
    pub depth: u32,
}

#[model_schema(nonstrict)]
#[derive(Debug, Deserialize, Serialize)]
pub struct Stamp {
    pub hue: String,
}

fn main() {}
