//! A member that holds an `i128`, which no surface describes.

use serde::{Deserialize, Serialize};
use tixschema::model_schema;

#[model_schema()]
#[derive(Debug, Deserialize, Serialize)]
pub struct FieldWide {
    pub x: i128,
}

fn main() {}
