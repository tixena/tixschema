//! An entry written twice.

extern crate bson2 as bson;

use serde::{Deserialize, Serialize};
use tixschema::model_schema;

tixschema::decode_with_foundation!();

#[model_schema(enable = ["decode_with", "decode_with"])]
#[derive(Debug, Deserialize, Serialize)]
pub struct InvoiceLookup {
    pub number: String,
}

fn main() {}
