//! A struct `#[model_schema]` did not expand, flattened beside an open tag-alone enum: tixschema
//! reads it as the strictest member.

use serde::{Deserialize, Serialize};
use tixschema::model_schema;

#[derive(Debug, Deserialize, Serialize)]
pub struct Loose {
    pub ink: u32,
}

#[model_schema(nonstrict)]
#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "tone")]
pub enum Hue {
    Warm { warmth: u32 },
}

#[model_schema()]
#[derive(Debug, Deserialize, Serialize)]
pub struct Parcel {
    #[serde(flatten)]
    pub hue: Hue,
    #[serde(flatten)]
    pub loose: Loose,
}

fn main() {}
