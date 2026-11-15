//! An alias whose refusal class under its published name is named like a type declared above it.

use serde::{Deserialize, Serialize};
use tixschema::model_schema;

#[model_schema()]
#[derive(Debug, Deserialize, Serialize)]
pub struct Parcel {
    pub label: String,
}
#[model_schema()]
#[derive(Debug, Deserialize, Serialize)]
pub struct BundleTypeRefusal {
    pub note: String,
}

#[model_schema()]
pub type Bundle = Parcel;

fn main() {}
