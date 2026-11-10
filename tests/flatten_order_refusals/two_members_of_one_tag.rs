//! Two open tag-alone enums under one tag.

use serde::{Deserialize, Serialize};
use tixschema::model_schema;

#[model_schema(nonstrict)]
#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "form")]
pub enum Shape {
    Round { radius: u32 },
}

#[model_schema(nonstrict)]
#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "form")]
pub enum Hue {
    Warm { warmth: u32 },
}

#[model_schema()]
#[derive(Debug, Deserialize, Serialize)]
pub struct Parcel {
    #[serde(flatten)]
    pub shape: Shape,
    #[serde(flatten)]
    pub hue: Hue,
}

fn main() {}
