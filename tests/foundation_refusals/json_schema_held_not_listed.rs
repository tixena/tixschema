//! A type that lists `"jsonschema"` holds a model type that does not: its document calls the held
//! type's `json_schema_within`, which that type does not have.

use serde::{Deserialize, Serialize};
use tixschema::model_schema;

#[model_schema()]
#[derive(Debug, Deserialize, Serialize)]
pub struct Version {
    pub number: u32,
}

#[model_schema(enable = ["jsonschema"])]
#[derive(Debug, Deserialize, Serialize)]
pub struct Record {
    pub versions: Vec<Version>,
}

fn main() {}
