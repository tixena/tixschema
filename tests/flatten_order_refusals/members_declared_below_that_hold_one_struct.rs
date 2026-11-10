//! A tag-alone enum and an untagged one, declared below the holder and `nonstrict`, whose newtype
//! variants hold one struct: both read and write its keys.

use serde::{Deserialize, Serialize};
use tixschema::model_schema;

#[model_schema(nonstrict)]
#[derive(Debug, Deserialize, Serialize)]
pub struct Parcel {
    pub label: String,
    #[serde(flatten)]
    pub mark: Mark,
    #[serde(flatten)]
    pub trim: Trim,
}

#[model_schema(nonstrict)]
#[derive(Debug, Deserialize, Serialize)]
pub struct Ink {
    pub hue: String,
    pub depth: u32,
}

#[model_schema(nonstrict)]
#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "kind")]
pub enum Mark {
    Inked(Ink),
    Line { length: u32 },
}

#[model_schema(nonstrict)]
#[derive(Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum Trim {
    Inked(Ink),
    Ribbon { width: u32 },
}

fn main() {}
