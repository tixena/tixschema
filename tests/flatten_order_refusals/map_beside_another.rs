//! A map flattened last beside a nonstrict tag-alone enum: the map reads the enum's keys as entries.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use tixschema::model_schema;

#[model_schema(nonstrict)]
#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "tone")]
pub enum Hue {
    Cold,
    Warm { warmth: u32 },
}

#[model_schema()]
#[derive(Debug, Deserialize, Serialize)]
pub struct Parcel {
    #[serde(flatten)]
    pub hue: Hue,
    #[serde(flatten)]
    pub counts: BTreeMap<String, u32>,
}

fn main() {}
