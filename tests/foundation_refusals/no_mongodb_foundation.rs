//! A type that lists "mongodb", in a crate that has written the `decode_with` library and not the
//! MongoDB one.

extern crate bson2 as bson;

use serde::{Deserialize, Serialize};
use tixschema::model_schema;

tixschema::decode_with_foundation!();

#[model_schema(enable = ["decode_with", "mongodb"])]
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Customer {
    pub name: String,
    pub open_invoices: u32,
}

fn main() {}
