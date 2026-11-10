//! A type parameter flattened beside an open tag-alone enum: each reads every key, and what fills
//! the parameter may refuse the enum's.

use serde::{Deserialize, Serialize};
use tixschema::model_schema;

#[model_schema(nonstrict)]
#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "tone")]
pub enum Hue {
    Warm { warmth: u32 },
    Cold { cold: u32 },
}

#[model_schema()]
#[derive(Debug, Deserialize, Serialize)]
pub struct Parcel<Held> {
    #[serde(flatten)]
    pub hue: Hue,
    #[serde(flatten)]
    pub held: Held,
}

fn main() {}
