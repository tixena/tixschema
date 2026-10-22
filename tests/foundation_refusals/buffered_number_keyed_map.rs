//! A map keyed by a number in an untagged enum and in an enum under a tag alone: serde reads each
//! from a copy of the payload it buffers, and reads no entry of the map back.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use tixschema::model_schema;

#[model_schema()]
#[derive(Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum Leadlines {
    Lines(BTreeMap<u32, u8>),
}

#[model_schema()]
#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "kind")]
pub enum Plumbed {
    Named { depths: BTreeMap<u32, u8> },
}

fn main() {}
