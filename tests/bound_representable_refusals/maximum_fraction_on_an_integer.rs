//! A maximum with a fraction on an integer field, below zero.

use serde::{Deserialize, Serialize};
use tixschema::model_schema;

#[model_schema()]
#[derive(Debug, Deserialize, Serialize)]
pub struct Mooring {
    #[model_schema_prop(maximum = -1.5)]
    pub most: i64,
}

fn main() {}
