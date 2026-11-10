//! A flattened struct of one slot over a map, declared below the holder, beside a field of its own.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use tixschema::model_schema;

#[model_schema(nonstrict)]
#[derive(Debug, Deserialize, Serialize)]
pub struct Parcel {
    pub label: String,
    #[serde(flatten)]
    pub tally: Tally,
}

#[model_schema()]
#[derive(Debug, Deserialize, Serialize)]
pub struct Tally(pub BTreeMap<String, u32>);

fn main() {}
