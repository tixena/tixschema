//! A flattened struct of one slot over a map keyed by a number, declared below the holder.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use tixschema::model_schema;

#[model_schema(nonstrict)]
#[derive(Debug, Deserialize, Serialize)]
pub struct Parcel {
    #[serde(flatten)]
    pub counts: Counts,
}

#[model_schema()]
#[derive(Debug, Deserialize, Serialize)]
pub struct Counts(pub BTreeMap<u32, u32>);

fn main() {}
