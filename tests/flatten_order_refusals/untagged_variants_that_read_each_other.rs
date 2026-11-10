//! An untagged enum whose two variants write the same object: each reads every value of the other,
//! so no order reads both back.

use serde::{Deserialize, Serialize};
use tixschema::model_schema;

#[model_schema()]
#[derive(Debug, Deserialize, Serialize)]
pub struct Finish {
    pub colour: String,
}

#[model_schema()]
#[derive(Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum Coating {
    Painted { colour: String },
    Finished(Finish),
}

fn main() {}
