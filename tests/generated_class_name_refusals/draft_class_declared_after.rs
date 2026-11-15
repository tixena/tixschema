//! A type named like the draft class of a type declared above it.

use serde::{Deserialize, Serialize};
use tixschema::model_schema;

#[model_schema()]
#[derive(Debug, Deserialize, Serialize)]
pub struct Parcel {
    pub label: String,
}

#[model_schema()]
#[derive(Debug, Deserialize, Serialize)]
pub struct ParcelDraft {
    pub note: String,
}

fn main() {}
