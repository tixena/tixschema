//! A flattened plain enum declared below the holder.

use serde::{Deserialize, Serialize};
use tixschema::model_schema;

#[model_schema()]
#[derive(Debug, Deserialize, Serialize)]
pub struct Parcel {
    pub label: String,
    #[serde(flatten)]
    pub colour: Colour,
}

#[model_schema()]
#[derive(Debug, Deserialize, Serialize)]
pub enum Colour {
    Red,
    Blue,
}

fn main() {}
