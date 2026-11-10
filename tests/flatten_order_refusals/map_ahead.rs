//! A map flattened ahead of a struct: the map reads the struct's keys as its own entries.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use tixschema::model_schema;

#[model_schema()]
#[derive(Debug, Deserialize, Serialize)]
pub struct Stamp {
    pub at: String,
}

#[model_schema()]
#[derive(Debug, Deserialize, Serialize)]
pub struct Parcel {
    #[serde(flatten)]
    pub counts: BTreeMap<String, u32>,
    #[serde(flatten)]
    pub stamp: Stamp,
}

fn main() {}
