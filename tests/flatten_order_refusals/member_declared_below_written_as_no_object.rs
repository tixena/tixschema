//! A tuple struct declared below the struct that flattens it, which serde writes as no object.

use serde::{Deserialize, Serialize};
use tixschema::model_schema;

#[model_schema()]
#[derive(Debug, Deserialize, Serialize)]
pub struct Parcel {
    pub label: String,
    #[serde(flatten)]
    pub size: Size,
}

#[model_schema()]
#[derive(Debug, Deserialize, Serialize)]
pub struct Size(u32, u32);

fn main() {}
