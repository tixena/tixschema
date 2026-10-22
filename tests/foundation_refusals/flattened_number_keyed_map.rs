//! A map keyed by a number, flattened: serde reads no entry of it back.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use tixschema::model_schema;

#[model_schema()]
#[derive(Debug, Deserialize, Serialize)]
pub struct Soundings {
    #[serde(flatten)]
    pub rest: BTreeMap<isize, u8>,
}

fn main() {}
