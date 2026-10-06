//! The aliases the fields beside this module are typed with, and the model type one of them
//! holds. Nothing they are written with is in scope where those fields are.

use alloc::collections::BTreeMap;
use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use tixschema::model_schema;

use super::super::Version;
#[cfg(feature = "jsonschema")]
use super::super::version_schema;

#[model_schema()]
pub type Counts = HashMap<String, i32>;

#[model_schema(decode_with)]
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Mark {
    pub weight: i32,
}

#[model_schema()]
pub type Marks = BTreeMap<String, Vec<Mark>>;

#[model_schema()]
pub type Versions = Vec<Version>;
