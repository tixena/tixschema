//! A maximum length below a fixed array's length.

use serde::{Deserialize, Serialize};
use tixschema::model_schema;

#[model_schema()]
#[derive(Debug, Deserialize, Serialize)]
pub struct Berth {
    #[model_schema_prop(maxLength = 1)]
    pub piers: [String; 2],
}

fn main() {}
