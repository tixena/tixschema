//! A flattened `Option` of a strict tag-alone enum, which serde reads as `None` beside a stray key.

use serde::{Deserialize, Serialize};
use tixschema::model_schema;

#[model_schema()]
#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "kind")]
pub enum Mark {
    Line { length: u32 },
}

#[model_schema()]
#[derive(Debug, Deserialize, Serialize)]
pub struct Parcel {
    pub label: String,
    #[serde(flatten, default, skip_serializing_if = "Option::is_none")]
    pub mark: Option<Mark>,
}

fn main() {}
