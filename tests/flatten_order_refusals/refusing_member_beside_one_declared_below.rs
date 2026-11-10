//! A strict tag-alone enum declared above beside an open one declared below the holder.

use serde::{Deserialize, Serialize};
use tixschema::model_schema;

#[model_schema()]
#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "kind")]
pub enum Shape {
    Circle { radius: u32 },
}

#[model_schema()]
#[derive(Debug, Deserialize, Serialize)]
pub struct Parcel {
    #[serde(flatten)]
    pub hue: Hue,
    #[serde(flatten)]
    pub shape: Shape,
}

#[model_schema(nonstrict)]
#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "tone")]
pub enum Hue {
    Warm { warmth: u32 },
}

fn main() {}
