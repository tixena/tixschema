//! A crate apart from the test binaries: the generic types a Swift holder fills with models, which
//! that holder's expansion has never seen.

// No lint table reaches a crate apart, and clippy lints none.
#![deny(warnings)]

use serde::{Deserialize, Serialize};
use tixschema::model_schema;

/// A model with a bound.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApartParcel {
    #[model_schema_prop(minLength = 1)]
    pub item_label: String,
}

/// A generic struct with a rule of its own.
#[model_schema(default_types(Item = String))]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApartPage<Item> {
    pub items: Vec<Item>,
    #[model_schema_prop(minLength = 1)]
    pub page_title: String,
}

/// A generic struct whose member reaches the parameter through a tuple.
#[model_schema(default_types(Item = String))]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ApartTupled<Item> {
    pub pair: (Item, u32),
}

/// A generic enum with a tuple variant of several slots.
#[model_schema(default_types(Item = String))]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ApartSlots<Item> {
    Empty,
    Pair(Item, u32),
}
