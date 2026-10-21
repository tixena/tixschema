//! A type that lists "mongodb" and holds a model declared above with a map keyed by a number.

extern crate bson2 as bson;

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use tixschema::model_schema;

tixschema::decode_with_foundation!();
tixschema::mongodb_foundation!();

#[model_schema(enable = ["decode_with"])]
#[derive(Debug, Deserialize, Serialize)]
pub struct Seating {
    pub by_number: HashMap<u32, String>,
}

#[model_schema(enable = ["decode_with", "mongodb"])]
#[derive(Debug, Deserialize, Serialize)]
pub struct Venue {
    pub seating: Seating,
}

fn main() {}
