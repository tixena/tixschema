//! A renamed alias whose refusal class under its Rust name is named like a type declared above it.

use serde::{Deserialize, Serialize};
use tixschema::model_schema;

#[model_schema()]
#[derive(Debug, Deserialize, Serialize)]
pub struct Parcel {
    pub label: String,
}
#[model_schema()]
#[derive(Debug, Deserialize, Serialize)]
pub struct WrappedRefusal {
    pub note: String,
}

#[model_schema(name = "Bundle")]
pub type Wrapped = Parcel;

fn main() {}
