//! A newtype variant under a tag alone over a struct of one slot over a `String`, declared below
//! the enum: serde refuses every write of the variant.

use serde::{Deserialize, Serialize};
use tixschema::model_schema;

#[model_schema()]
#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "kind")]
pub enum Label {
    Coded(Code),
}

#[model_schema()]
#[derive(Debug, Deserialize, Serialize)]
pub struct Code(pub String);

fn main() {}
