//! A type named like the refusal class of a type declared above it.

use serde::{Deserialize, Serialize};
use tixschema::model_schema;

#[model_schema()]
#[derive(Debug, Deserialize, Serialize)]
pub struct Parcel {
    pub label: String,
}

#[model_schema()]
#[derive(Debug, Deserialize, Serialize)]
pub struct ParcelRefusal {
    pub note: String,
}

fn main() {}
