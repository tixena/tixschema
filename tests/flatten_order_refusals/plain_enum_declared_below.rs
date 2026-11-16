//! A flattened plain enum declared below the holder.

use serde::{Deserialize, Serialize};
use tixschema::model_schema;

#[model_schema()]
#[derive(Debug, Deserialize, Serialize)]
pub struct Casement {
    pub width: u32,
    #[serde(flatten)]
    pub weathering: Weathering,
}

#[model_schema()]
#[derive(Debug, Deserialize, Serialize)]
pub enum Weathering {
    Sound,
    Worn,
}

fn main() {}
