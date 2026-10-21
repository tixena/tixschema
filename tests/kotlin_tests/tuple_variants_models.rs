//! The models of `kotlin_tests/tuple_variants.rs` and of
//! `service_schema_emitted_client_tests/run_kotlin_tuple_variants.rs`: an enum with a variant of
//! several slots, under each tagging serde accepts for one, and the untagged enum that writes the
//! same slots from a tuple in its variant's one slot. Beside them, a tuple in each place a writer
//! names its wrapper more than once.

#[cfg(not(feature = "jsonschema"))]
use std::collections::HashMap;
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

/// A map of tuples in a variant's one slot, under a tag and a content key. A JSON Schema refuses
/// a tuple as a map value.
#[cfg(not(feature = "jsonschema"))]
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", content = "held")]
pub enum Tallying {
    Counts(HashMap<String, (u32, u8)>),
}

/// A tuple read as its default where the payload leaves its key out.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Strapping {
    #[serde(default)]
    pub pair: (u32, u8),
}

/// The member [`Bundling`] flattens.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Labelling {
    pub label: String,
}

/// A tuple beside a flattened member.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Bundling {
    #[serde(flatten)]
    pub labelling: Labelling,
    pub pair: (u32, u8),
}

/// A tuple in a slot of a tuple struct.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Palleting(pub (u32, u8), pub u8);

/// A tuple in a position of a tuple.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Crating {
    pub pair: ((u32, u8), u8),
}
