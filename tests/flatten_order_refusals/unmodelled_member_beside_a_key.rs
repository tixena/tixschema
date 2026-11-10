//! A struct and a map alias no `#[model_schema]` expanded, flattened beside a key: tixschema cannot
//! see which keys they write, so it cannot tell whether one of them is the key beside it.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use tixschema::model_schema;

#[derive(Debug, Deserialize, Serialize)]
pub struct Address {
    pub city: String,
}

pub type Tags = BTreeMap<String, u32>;

#[model_schema()]
#[derive(Debug, Deserialize, Serialize)]
pub struct Customer {
    pub name: String,
    #[serde(flatten)]
    pub address: Address,
}

#[model_schema(nonstrict)]
#[derive(Debug, Deserialize, Serialize)]
pub struct Product {
    pub name: String,
    #[serde(flatten)]
    pub tags: Tags,
}

fn main() {}
