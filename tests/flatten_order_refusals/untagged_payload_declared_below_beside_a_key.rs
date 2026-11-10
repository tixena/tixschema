//! An untagged enum flattened beside a field, whose newtype variant holds a struct declared below
//! the enum: the Swift read of the enum hands that struct every key beside it.

use serde::{Deserialize, Serialize};
use tixschema::model_schema;

#[model_schema()]
#[derive(Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum Mark {
    Inked { ink: String },
    Stamped(Stamp),
}

#[model_schema()]
#[derive(Debug, Deserialize, Serialize)]
pub struct Parcel {
    pub label: String,
    #[serde(flatten)]
    pub mark: Mark,
}

#[model_schema()]
#[derive(Debug, Deserialize, Serialize)]
pub struct Stamp {
    pub at: u32,
}

fn main() {}
