//! A flattened struct that reads, under an alias, the key a struct flattened after it writes.

use serde::{Deserialize, Serialize};
use tixschema::model_schema;

#[model_schema()]
#[derive(Debug, Deserialize, Serialize)]
pub struct Ink {
    #[serde(alias = "at")]
    pub colour: u32,
}

#[model_schema()]
#[derive(Debug, Deserialize, Serialize)]
pub struct Dated {
    pub at: u32,
}

#[model_schema()]
#[derive(Debug, Deserialize, Serialize)]
pub struct Stamp {
    #[serde(flatten)]
    pub ink: Ink,
    #[serde(flatten)]
    pub date: Dated,
}

fn main() {}
