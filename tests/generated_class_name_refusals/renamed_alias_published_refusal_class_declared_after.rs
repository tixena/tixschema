//! A type named like the refusal class a renamed alias writes under its published name, declared above it.

use serde::{Deserialize, Serialize};
use tixschema::model_schema;

#[model_schema()]
#[derive(Debug, Deserialize, Serialize)]
pub struct Parcel {
    pub label: String,
}
#[model_schema(name = "Bundle")]
pub type Wrapped = Parcel;

#[model_schema()]
#[derive(Debug, Deserialize, Serialize)]
pub struct BundleRefusal {
    pub note: String,
}

fn main() {}
