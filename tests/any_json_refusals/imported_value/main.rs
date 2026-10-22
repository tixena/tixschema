//! A field written `Value`, where the name is `serde_json`'s, brought into scope with `use`: the
//! check written beside the holder fails with the refusal of a JSON value.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use tixschema::model_schema;

#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Reading {
    pub data: Value,
}

fn main() {}
