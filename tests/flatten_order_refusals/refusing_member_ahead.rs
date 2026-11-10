//! A tag-alone enum, strict by default, flattened ahead of a struct: serde refuses the struct's keys.

use serde::{Deserialize, Serialize};
use tixschema::model_schema;

#[model_schema()]
#[derive(Debug, Deserialize, Serialize)]
pub struct Stamp {
    pub at: String,
}

#[model_schema()]
#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "kind")]
pub enum Shape {
    Circle { radius: u32 },
    Square { side: u32 },
}

#[model_schema()]
#[derive(Debug, Deserialize, Serialize)]
pub struct Parcel {
    pub label: String,
    #[serde(flatten)]
    pub shape: Shape,
    #[serde(flatten)]
    pub stamp: Stamp,
}

fn main() {}
