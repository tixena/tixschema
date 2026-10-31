//! A minimum length above the maximum length.

use serde::{Deserialize, Serialize};
use tixschema::model_schema;

#[model_schema()]
#[derive(Debug, Deserialize, Serialize)]
pub struct Mooring {
    #[model_schema_prop(minLength = 5, maxLength = 2)]
    pub line: String,
}

fn main() {}
