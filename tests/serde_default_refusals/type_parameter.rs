//! A member whose type is a type parameter of the enum: its default has one JSON for each type
//! that fills the parameter.

use serde::{Deserialize, Serialize};
use tixschema::model_schema;

#[model_schema(default_types(T = String))]
#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "kind")]
pub enum Berthing<T> {
    Docked {
        #[serde(default)]
        extra: T,
        pier: u32,
    },
}

fn main() {}
