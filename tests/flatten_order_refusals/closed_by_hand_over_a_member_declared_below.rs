//! A holder its author closed with `deny_unknown_fields` over an open enum declared below it.

use serde::{Deserialize, Serialize};
use tixschema::model_schema;

#[model_schema()]
#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Parcel {
    pub label: String,
    #[serde(flatten)]
    pub hue: Hue,
}

#[model_schema(nonstrict)]
#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "tone")]
pub enum Hue {
    Warm { warmth: u32 },
}

fn main() {}
