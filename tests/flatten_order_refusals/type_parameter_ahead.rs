//! A type parameter flattened ahead of a struct: what fills it may refuse the struct's keys.

use serde::{Deserialize, Serialize};
use tixschema::model_schema;

#[model_schema()]
#[derive(Debug, Deserialize, Serialize)]
pub struct Stamp {
    pub at: String,
}

#[model_schema()]
#[derive(Debug, Deserialize, Serialize)]
pub struct Parcel<Held> {
    #[serde(flatten)]
    pub held: Held,
    #[serde(flatten)]
    pub stamp: Stamp,
}

fn main() {}
