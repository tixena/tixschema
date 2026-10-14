//! A type that lists "mongodb", holding a generic type that lists "decode_with" alone.

extern crate bson2 as bson;

use serde::{Deserialize, Serialize};
use tixschema::model_schema;

tixschema::decode_with_foundation!();
tixschema::mongodb_foundation!();

#[model_schema(enable = ["decode_with"], default_types(T = String))]
#[derive(Debug, Deserialize, Serialize)]
pub struct Wrapper<T> {
    pub held: T,
}

#[model_schema(enable = ["decode_with", "mongodb"])]
#[derive(Debug, Deserialize, Serialize)]
pub struct Invoice {
    pub number: String,
    pub wrapped: Wrapper<String>,
}

fn main() {}
