//! A map keyed by a `bool`, flattened: serde reads no entry of it back.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use tixschema::model_schema;

#[model_schema()]
#[derive(Debug, Deserialize, Serialize)]
pub struct Switches {
    #[serde(flatten)]
    pub rest: BTreeMap<bool, u8>,
}

fn main() {}
