//! A string check on a `#[serde(transparent)]` struct, in a build with no feature: without
//! tixschema's `serde` feature the struct is no brand, and the check is refused naming the feature.

use serde::{Deserialize, Serialize};
use tixschema::model_schema;

#[model_schema(minLength = 3)]
#[derive(Debug, Deserialize, Serialize)]
#[serde(transparent)]
pub struct Username(String);

fn main() {}
