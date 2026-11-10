//! An externally tagged enum with a unit variant, declared below the struct that flattens it.

use serde::{Deserialize, Serialize};
use tixschema::model_schema;

#[model_schema()]
#[derive(Debug, Deserialize, Serialize)]
pub struct Parcel {
    pub label: String,
    #[serde(flatten)]
    pub mark: Mark,
}

#[model_schema()]
#[derive(Debug, Deserialize, Serialize)]
pub enum Mark {
    Plain,
    Stamped { ink: String },
}

fn main() {}
