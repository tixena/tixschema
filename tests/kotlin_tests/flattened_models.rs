//! The models of the flattened Kotlin reads `kotlin_tests/tests.rs` pins and
//! `service_schema_emitted_client_tests/run_kotlin_flattened.rs` runs: a map flattened as the one
//! member of its struct, a flattened struct beside a member of its own, and an optional member
//! beside a flattened one, with its twin under `nullable`.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use tixschema::model_schema;

use super::tuple_variants_models::{Labelling, labelling_schema};

/// A map, the one member, which serde hands every key of the object.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Netting {
    #[serde(flatten)]
    pub rest: HashMap<String, u8>,
}

/// A flattened struct beside a member of its own.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Knotting {
    #[serde(flatten)]
    pub labelling: Labelling,
    pub note: String,
}

/// An optional member beside a flattened one, its key left out where it holds no value.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Splicing {
    #[serde(flatten)]
    pub labelling: Labelling,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spare: Option<u32>,
}

/// [`Splicing`] with its optional member under `nullable`, whose key is always written.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Seizing {
    #[serde(flatten)]
    pub labelling: Labelling,
    #[model_schema_prop(nullable)]
    pub spare: Option<u32>,
}
