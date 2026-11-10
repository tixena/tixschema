//! A field of the holder's own named after the tag of a flattened enum declared below it.

use serde::{Deserialize, Serialize};
use tixschema::model_schema;

#[model_schema(nonstrict)]
#[derive(Debug, Deserialize, Serialize)]
pub struct Parcel {
    pub kind: String,
    #[serde(flatten)]
    pub mark: Mark,
}

#[model_schema()]
#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "kind")]
pub enum Mark {
    Line { length: u32 },
}

fn main() {}
