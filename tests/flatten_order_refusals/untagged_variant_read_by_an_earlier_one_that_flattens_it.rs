//! An untagged enum whose struct variant flattens what its newtype variant holds, beside an
//! optional member: the struct variant reads every value of the newtype variant after it.

use serde::{Deserialize, Serialize};
use tixschema::model_schema;

#[model_schema()]
#[derive(Debug, Deserialize, Serialize)]
pub struct Dimensions {
    pub width: u32,
}

#[model_schema()]
#[derive(Debug, Deserialize, Serialize)]
pub struct Finish {
    pub colour: String,
}

#[model_schema()]
#[derive(Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum Panel {
    Painted {
        #[serde(flatten)]
        dimensions: Dimensions,
        #[serde(flatten)]
        finish: Option<Finish>,
    },
    Bare(Dimensions),
}

fn main() {}
