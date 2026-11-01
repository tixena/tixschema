//! The `"jsonschema"` entry written twice.

use serde::{Deserialize, Serialize};
use tixschema::model_schema;

#[model_schema(enable = ["jsonschema", "jsonschema"])]
#[derive(Debug, Deserialize, Serialize)]
pub struct Signup {
    pub username: String,
}

fn main() {}
