//! A minimum length that only repeats a fixed array's length.

use serde::{Deserialize, Serialize};
use tixschema::model_schema;

#[model_schema()]
#[derive(Debug, Deserialize, Serialize)]
pub struct Pontoon {
    #[model_schema_prop(minLength = 2)]
    pub piers: [String; 2],
}

fn main() {}
