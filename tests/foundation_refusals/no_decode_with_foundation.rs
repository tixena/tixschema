//! A type that lists "decode_with", in a crate that has not written the `decode_with` library.

extern crate bson2 as bson;

use serde::{Deserialize, Serialize};
use tixschema::model_schema;

#[model_schema(enable = ["decode_with"])]
#[derive(Debug, Deserialize, Serialize)]
pub struct InvoiceLookup {
    pub number: String,
}

fn main() {}
