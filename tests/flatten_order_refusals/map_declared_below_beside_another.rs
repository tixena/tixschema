//! A struct of one flattened map declared below the holder that flattens it beside an open enum.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use tixschema::model_schema;

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
    pub counts: Counts,
}

#[model_schema()]
#[derive(Debug, Deserialize, Serialize)]
pub struct Counts {
    #[serde(flatten)]
    pub entries: BTreeMap<String, u32>,
}

fn main() {}
