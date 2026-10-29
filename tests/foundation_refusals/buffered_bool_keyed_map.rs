//! A map keyed by a `bool` in an untagged enum, in an enum under a tag alone and in a variant of
//! one that wraps the map: each is refused.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use tixschema::model_schema;

#[model_schema()]
#[derive(Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum Latches {
    Latched(BTreeMap<bool, u8>),
}

#[model_schema()]
#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "kind")]
pub enum Hasped {
    Named { latches: BTreeMap<bool, u8> },
}

#[model_schema()]
#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "kind")]
pub enum Wrapped {
    Latched(BTreeMap<bool, u8>),
}

fn main() {}
