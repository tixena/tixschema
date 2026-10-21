//! A map keyed by a number, on a type that lists "mongodb".

extern crate bson2 as bson;

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use tixschema::model_schema;

tixschema::decode_with_foundation!();
tixschema::mongodb_foundation!();

#[model_schema(enable = ["decode_with", "mongodb"])]
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Roster {
    pub by_number: HashMap<u32, String>,
}

fn main() {}
