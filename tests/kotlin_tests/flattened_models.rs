//! The models of the flattened Kotlin reads `kotlin_tests/tests.rs` pins and
//! `service_schema_emitted_client_tests/run_kotlin_flattened.rs` runs: a struct whose members are
//! all flattened, one of them a map, and its twin with a member of its own.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use tixschema::model_schema;

use super::tuple_variants_models::{Labelling, labelling_schema};

/// Every member flattened, the map last, where serde hands it the keys no member ahead took.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Netting {
    #[serde(flatten)]
    pub labelling: Labelling,
    #[serde(flatten)]
    pub rest: HashMap<String, u8>,
}

/// [`Netting`] with a member of its own beside the two flattened ones.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Knotting {
    #[serde(flatten)]
    pub labelling: Labelling,
    pub note: String,
    #[serde(flatten)]
    pub rest: HashMap<String, u8>,
}
