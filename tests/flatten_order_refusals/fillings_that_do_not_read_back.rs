//! Fillings of a type parameter a type flattens beside a key of its own, each written in a field of
//! a holder: a struct that writes that key, an `Option` of a struct of defaulted members, a map
//! keyed by a number, an `Option` of a map and a map, none of which serde reads back.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use tixschema::model_schema;

#[model_schema(default_types(T = Ink))]
#[derive(Debug, Deserialize, Serialize)]
pub struct Labelled<T> {
    pub code: String,
    #[serde(flatten)]
    pub inner: T,
}

#[model_schema()]
#[derive(Debug, Deserialize, Serialize)]
pub struct Ink {
    pub hue: String,
}

#[model_schema()]
#[derive(Debug, Deserialize, Serialize)]
pub struct Coded {
    pub code: String,
}

#[model_schema()]
#[derive(Debug, Deserialize, Serialize)]
pub struct Pad {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tint: Option<String>,
}

#[model_schema()]
#[derive(Debug, Deserialize, Serialize)]
pub struct Bundle {
    pub coded: Labelled<Coded>,
    pub padded: Labelled<Option<Pad>>,
}

#[model_schema()]
#[derive(Debug, Deserialize, Serialize)]
pub struct Crate {
    pub numbered: Labelled<BTreeMap<u32, u32>>,
    pub maybe_counted: Labelled<Option<BTreeMap<String, u32>>>,
    pub counted: Labelled<BTreeMap<String, u32>>,
}

fn main() {}
