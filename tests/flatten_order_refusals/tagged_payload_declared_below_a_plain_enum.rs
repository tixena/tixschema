//! A newtype variant under a tag alone over a plain enum declared below the enum: serde writes the
//! plain enum's variant name beside the tag as a key over `null`.

use serde::{Deserialize, Serialize};
use tixschema::model_schema;

#[model_schema()]
#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "kind")]
pub enum Label {
    Inked(Colour),
}

#[model_schema()]
#[derive(Debug, Deserialize, Serialize)]
pub enum Colour {
    Red,
    Blue,
}

fn main() {}
