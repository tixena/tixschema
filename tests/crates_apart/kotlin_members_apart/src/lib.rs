//! A crate apart from the test binaries: the types a holder in `tests/crates_apart_tests.rs`
//! names in a member, which that holder's expansion has never seen.

// No lint table reaches a crate apart, and clippy lints none.
#![deny(warnings)]

use serde::{Deserialize, Serialize};
use tixschema::model_schema;

/// A plain enum.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum ZedApart {
    A,
    B,
}

/// A struct.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ZedBagApart {
    pub count: u32,
}

/// A brand over a text.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(transparent)]
pub struct TagApart(String);

/// A type a service writes for itself, which has no draft.
#[model_schema(__tixschema_undrafted)]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ServedApart {
    pub note: String,
}
