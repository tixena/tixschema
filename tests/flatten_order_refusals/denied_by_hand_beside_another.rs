//! A struct that flattens a struct and is closed by its author's `deny_unknown_fields`, beside an
//! open tag-alone enum: the author's attribute is what refuses the enum's keys.

use serde::{Deserialize, Serialize};
use tixschema::model_schema;

#[model_schema()]
#[derive(Debug, Deserialize, Serialize)]
pub struct Stamp {
    pub at: String,
}

#[model_schema()]
#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Postmark {
    pub office: String,
    #[serde(flatten)]
    pub stamp: Stamp,
}

#[model_schema(nonstrict)]
#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "tone")]
pub enum Hue {
    Warm { warmth: u32 },
    Cold { cold: u32 },
}

#[model_schema()]
#[derive(Debug, Deserialize, Serialize)]
pub struct Parcel {
    #[serde(flatten)]
    pub hue: Hue,
    #[serde(flatten)]
    pub postmark: Postmark,
}

fn main() {}
