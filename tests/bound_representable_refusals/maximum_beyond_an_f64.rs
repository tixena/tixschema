//! A maximum no f64 holds.

use serde::{Deserialize, Serialize};
use tixschema::model_schema;

#[model_schema()]
#[derive(Debug, Deserialize, Serialize)]
pub struct Mooring {
    #[model_schema_prop(maximum = 1e999)]
    pub double: f64,
}

fn main() {}
