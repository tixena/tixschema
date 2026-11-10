//! A strict tag-alone enum flattened into a struct under its own tag: serde leaves the struct's
//! tag for the enum to read, and the enum refuses it.

use serde::{Deserialize, Serialize};
use tixschema::model_schema;

#[model_schema()]
#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "kind")]
pub enum Shape {
    Circle { radius: u32 },
    Square { side: u32 },
}

#[model_schema()]
#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "parcel")]
pub struct Parcel {
    pub label: String,
    #[serde(flatten)]
    pub shape: Shape,
}

fn main() {}
