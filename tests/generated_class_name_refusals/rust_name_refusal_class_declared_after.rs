//! A type named like the refusal class a renamed type writes under its Rust name.

use serde::{Deserialize, Serialize};
use tixschema::model_schema;

#[model_schema(name = "Package")]
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
