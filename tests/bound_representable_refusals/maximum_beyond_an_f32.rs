//! A maximum no f32 holds.

use serde::{Deserialize, Serialize};
use tixschema::model_schema;

#[model_schema()]
#[derive(Debug, Deserialize, Serialize)]
pub struct Mooring {
    #[model_schema_prop(maximum = 1e39)]
    pub single: f32,
}

fn main() {}
