//! A member of a variant of a tag-alone enum whose type, declared below the enum, holds a map keyed
//! by a number.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use tixschema::model_schema;

#[model_schema()]
#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "kind")]
pub enum Parcel {
    Counted { counts: Counts },
}

#[model_schema()]
#[derive(Debug, Deserialize, Serialize)]
pub struct Counts {
    pub by_shelf: BTreeMap<u32, u32>,
}

fn main() {}
