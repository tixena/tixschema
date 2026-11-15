//! A crate apart from the test binaries: the message a service names whose expansion has never
//! seen it, with a bound on each member and on the type it holds.

// No lint table reaches a crate apart, and clippy lints none.
#![deny(warnings)]

use serde::{Deserialize, Serialize};
use tixschema::model_schema;

/// What a load holds.
#[model_schema(enable = ["jsonschema"])]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ApartItem {
    #[model_schema_prop(minLength = 1)]
    pub tag: String,
}

/// A load a hoist lifts.
#[model_schema(enable = ["jsonschema"])]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ApartLoad {
    pub items: Vec<ApartItem>,
    #[model_schema_prop(minLength = 1)]
    pub label: String,
    #[model_schema_prop(maximum = 100)]
    pub weight: u32,
}

/// A load published under another name than its Rust ident.
#[model_schema(enable = ["jsonschema"], name = "PublishedApart")]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ApartRenamed {
    pub items: Vec<ApartItem>,
    #[model_schema_prop(minLength = 1)]
    pub label: String,
    #[model_schema_prop(maximum = 100)]
    pub weight: u32,
}
