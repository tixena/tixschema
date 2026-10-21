//! The models of `kotlin_tests/tuple_variants.rs` and of
//! `service_schema_emitted_client_tests/run_kotlin_tuple_variants.rs`: an enum with a variant of
//! several slots, under each tagging serde accepts for one, and the untagged enum that writes the
//! same slots from a tuple in its variant's one slot.

use std::collections::HashSet;

use serde::{Deserialize, Serialize};
use tixschema::model_schema;

/// A variant of several slots under a tag and a content key.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", content = "held")]
pub enum Racking {
    Loose { members: HashSet<String> },
    Split(Vec<u32>, u8),
}

/// A variant of several slots under its own name.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum Stacking {
    Loose { members: HashSet<String> },
    Split(Vec<u32>, u8),
}

/// The slots of [`Racking::Split`], written as they are: an untagged variant of several slots is
/// refused, so its one slot holds them as a tuple.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(untagged)]
pub enum Piling {
    Loose { members: HashSet<String> },
    Split((Vec<u32>, u8)),
}
