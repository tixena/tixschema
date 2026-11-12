//! A minimum with a fraction on an integer field.

use serde::{Deserialize, Serialize};
use tixschema::model_schema;

#[model_schema()]
#[derive(Debug, Deserialize, Serialize)]
pub struct Mooring {
    #[model_schema_prop(minimum = 1.5)]
    pub least: i32,
}

fn main() {}
