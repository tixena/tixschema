//! An enum under a tag that declares no variant: serde writes no value of it and reads none.

use serde::{Deserialize, Serialize};
use tixschema::model_schema;

#[model_schema()]
#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "kind")]
pub enum BecalmTagged {}

fn main() {}
