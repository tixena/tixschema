//! Two refused members of two variants, each reported in the one expansion.

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
    Moored {
        #[serde(default, skip_serializing_if = "String::is_empty")]
        dock: String,
    },
}

fn main() {}
