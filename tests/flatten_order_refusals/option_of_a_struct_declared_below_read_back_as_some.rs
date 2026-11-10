//! A flattened `Option` of a struct declared below its holder whose every member is defaulted.

use serde::{Deserialize, Serialize};
use tixschema::model_schema;

#[model_schema()]
#[derive(Debug, Deserialize, Serialize)]
pub struct Stamp {
    pub at: u32,
    #[serde(default, flatten, skip_serializing_if = "Option::is_none")]
    pub ink: Option<Ink>,
}

#[model_schema()]
#[derive(Debug, Deserialize, Serialize)]
pub struct Ink {
    #[serde(default)]
    pub colour: u32,
}

fn main() {}
