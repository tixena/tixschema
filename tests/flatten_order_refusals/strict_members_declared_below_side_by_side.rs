//! A strict struct and a strict tag-alone enum, both declared below the holder that flattens them:
//! the Dart and Swift reads, written before either is met, hand each the keys of the other.

use serde::{Deserialize, Serialize};
use tixschema::model_schema;

#[model_schema(nonstrict)]
#[derive(Debug, Deserialize, Serialize)]
pub struct Parcel {
    #[serde(flatten)]
    pub stamp: Stamp,
    #[serde(flatten)]
    pub shape: Shape,
}

#[model_schema()]
#[derive(Debug, Deserialize, Serialize)]
pub struct Stamp {
    pub at: u32,
}

#[model_schema()]
#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "kind")]
pub enum Shape {
    Circle { radius: u32 },
}

fn main() {}
