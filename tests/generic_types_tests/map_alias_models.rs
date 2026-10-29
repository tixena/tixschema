//! Aliases of a map whose values are a model: one over a plain model, and one over an enum
//! declared below that holds the alias.

use alloc::collections::BTreeMap;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tixschema::model_schema;

/// A plain model, the value of an alias declared beside it.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Stall {
    pub goods: String,
}

#[model_schema()]
pub type Bazaar = BTreeMap<String, Stall>;

#[model_schema()]
pub type Souk = HashMap<String, Stall>;

/// An alias declared above the enum that holds it.
#[model_schema()]
pub type Sheet = HashMap<String, Cell>;

#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[non_exhaustive]
pub enum Cell {
    Flag(bool),
    Nested(Sheet),
    Row(Vec<Self>),
    Text(String),
}
