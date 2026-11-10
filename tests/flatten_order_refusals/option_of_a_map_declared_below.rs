//! A flattened struct of one slot over an `Option` of a map, declared below the holder.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use tixschema::model_schema;

#[model_schema(nonstrict)]
#[derive(Debug, Deserialize, Serialize)]
pub struct Parcel {
    #[serde(flatten)]
    pub tally: MaybeTally,
}

#[model_schema()]
#[derive(Debug, Deserialize, Serialize)]
pub struct MaybeTally(pub Option<BTreeMap<String, u32>>);

fn main() {}
