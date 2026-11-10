//! A tag-alone enum declared below the holder that flattens it ahead of a struct: the holder reads
//! what it is from its hidden constant.

use serde::{Deserialize, Serialize};
use tixschema::model_schema;

#[model_schema()]
#[derive(Debug, Deserialize, Serialize)]
pub struct Stamp {
    pub at: String,
}

#[model_schema()]
#[derive(Debug, Deserialize, Serialize)]
pub struct Parcel {
    #[serde(flatten)]
    pub shape: Shape,
    #[serde(flatten)]
    pub stamp: Stamp,
}

#[model_schema()]
#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "kind")]
pub enum Shape {
    Circle { radius: u32 },
}

fn main() {}
