//! An alias that names the crate's own `Value`, declared below it: the check written beside the
//! alias finds it is no `serde_json::Value`, and the crate builds with every warning denied.

#![deny(warnings)]

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use tixschema::model_schema;

/// Keys and their values, in the order they were written.
#[model_schema()]
pub type Mapping = IndexMap<String, Value>;

/// What a key maps to, or what a list holds.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(untagged)]
pub enum Value {
    Boolean(bool),
    Text(String),
    Mapping(Mapping),
    Sequence(Vec<Self>),
}
