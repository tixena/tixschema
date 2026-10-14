//! A type that lists "mongodb", holding a type declared above it that lists "decode_with" alone.

extern crate bson2 as bson;

use serde::{Deserialize, Serialize};
use tixschema::model_schema;

tixschema::decode_with_foundation!();
tixschema::mongodb_foundation!();

#[model_schema(enable = ["decode_with"])]
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Customer {
    pub name: String,
    pub open_invoices: u32,
}

#[model_schema(enable = ["decode_with", "mongodb"])]
#[derive(Debug, Deserialize, Serialize)]
pub struct Invoice {
    pub customer: Customer,
    pub number: String,
}

fn main() {}
