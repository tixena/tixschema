//! A flattened `Option` of a map, which serde reads back as `Some` of a map of no entry.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use tixschema::model_schema;

#[model_schema()]
#[derive(Debug, Deserialize, Serialize)]
pub struct Locker {
    #[serde(default, flatten, skip_serializing_if = "Option::is_none")]
    pub counts: Option<BTreeMap<String, u8>>,
}

fn main() {}
