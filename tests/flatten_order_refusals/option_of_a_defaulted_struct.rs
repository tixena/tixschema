//! A flattened `Option` of a struct whose every member is defaulted, which serde reads back as
//! `Some`.

use serde::{Deserialize, Serialize};
use tixschema::model_schema;

#[model_schema()]
#[derive(Debug, Deserialize, Serialize)]
pub struct Ink {
    #[serde(default)]
    pub colour: u32,
}

#[model_schema()]
#[derive(Debug, Deserialize, Serialize)]
pub struct Stamp {
    pub at: u32,
    #[serde(default, flatten, skip_serializing_if = "Option::is_none")]
    pub ink: Option<Ink>,
}

fn main() {}
