//! A field of the holder's own beside a flattened struct declared below it that writes its key.

use serde::{Deserialize, Serialize};
use tixschema::model_schema;

#[model_schema()]
#[derive(Debug, Deserialize, Serialize)]
pub struct Parcel {
    pub hue: String,
    #[serde(flatten)]
    pub ink: Ink,
}

#[model_schema()]
#[derive(Debug, Deserialize, Serialize)]
pub struct Ink {
    pub hue: String,
    pub depth: u32,
}

fn main() {}
