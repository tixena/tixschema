//! A struct flattened beside a field, which flattens a struct declared below it: the Dart and Swift
//! reads of the middle struct were written before they could see the keys the last one reads.

use serde::{Deserialize, Serialize};
use tixschema::model_schema;

#[model_schema()]
#[derive(Debug, Deserialize, Serialize)]
pub struct Stamp {
    pub at: u32,
    #[serde(flatten)]
    pub ink: Ink,
}

#[model_schema()]
#[derive(Debug, Deserialize, Serialize)]
pub struct Parcel {
    pub label: String,
    #[serde(flatten)]
    pub stamp: Stamp,
}

#[model_schema()]
#[derive(Debug, Deserialize, Serialize)]
pub struct Ink {
    pub colour: u32,
}

fn main() {}
