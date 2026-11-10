//! An `Option` of an untagged enum, strict by default, flattened ahead of two structs.

use serde::{Deserialize, Serialize};
use tixschema::model_schema;

#[model_schema()]
#[derive(Debug, Deserialize, Serialize)]
pub struct Stamp {
    pub at: String,
}

#[model_schema()]
#[derive(Debug, Deserialize, Serialize)]
pub struct Ink {
    pub colour: String,
}

#[model_schema()]
#[derive(Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum Seal {
    Wax { wax: String },
    Foil { foil: String },
}

#[model_schema()]
#[derive(Debug, Deserialize, Serialize)]
pub struct Parcel {
    #[serde(flatten, default, skip_serializing_if = "Option::is_none")]
    pub seal: Option<Seal>,
    #[serde(flatten)]
    pub stamp: Stamp,
    #[serde(flatten)]
    pub ink: Ink,
}

fn main() {}
