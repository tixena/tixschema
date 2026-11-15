//! A type named like the refusal class of an alias declared above it.

use serde::{Deserialize, Serialize};
use tixschema::model_schema;

#[model_schema()]
#[derive(Debug, Deserialize, Serialize)]
pub struct Parcel {
    pub label: String,
}
#[model_schema()]
pub type Bundle = Parcel;

#[model_schema()]
#[derive(Debug, Deserialize, Serialize)]
pub struct BundleRefusal {
    pub note: String,
}

fn main() {}
