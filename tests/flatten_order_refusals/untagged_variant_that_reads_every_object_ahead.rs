//! A variant of an untagged enum that reads every object, declared ahead of one serde writes as an
//! object.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use tixschema::model_schema;

#[model_schema()]
#[derive(Debug, Deserialize, Serialize)]
pub struct Stamp {
    pub at: u32,
}

#[model_schema(nonstrict)]
#[derive(Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum Shelving {
    Any {
        #[serde(flatten)]
        extra: BTreeMap<String, u32>,
    },
    Stamped(Stamp),
}

fn main() {}
