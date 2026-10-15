//! The models of `swift_tests/wrapped_maps.rs` and of
//! `service_schema_emitted_client_tests/run_swift_wrapped_maps.rs`: a map under a key Swift reads
//! through a wrapper, a number, a plain enum and a `bool`, in a tuple's slot, in a tuple struct's
//! slot, as another map's value, in a variant's one slot and in the tuple of a variant of several
//! slots, under each tagging serde accepts, in a type that declares a parameter and in one that
//! declares none.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use tixschema::model_schema;

/// A plain enum a map is keyed by.
#[model_schema()]
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub enum Latch {
    Inner,
    Outer,
}

/// A model with a rule, held as the value of a map under a key Swift wraps.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Satchel {
    #[model_schema_prop(minLength = 1)]
    pub label: String,
}

/// Each map in a tuple's slot, as the value of a map Swift keys natively and as the value of a map
/// under a key Swift wraps.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Cubby {
    pub flags: (HashMap<bool, String>, u8),
    pub flags_by_number: HashMap<u32, HashMap<bool, String>>,
    pub latches: (HashMap<Latch, String>, u8),
    pub latches_by_latch: HashMap<Latch, HashMap<Latch, String>>,
    pub numbers: (HashMap<u32, String>, u8),
    pub numbers_by_name: HashMap<String, HashMap<u32, String>>,
    pub satchels: (HashMap<u32, Satchel>, u8),
    pub satchels_by_name: HashMap<String, HashMap<u32, Satchel>>,
}

/// [`Cubby`] in a type that declares a parameter.
#[model_schema(default_types(Stowed = String))]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct StowedCubby<Stowed> {
    pub flags: (HashMap<bool, Stowed>, u8),
    pub flags_by_number: HashMap<u32, HashMap<bool, Stowed>>,
    pub latches: (HashMap<Latch, Stowed>, u8),
    pub latches_by_latch: HashMap<Latch, HashMap<Latch, Stowed>>,
    pub numbers: (HashMap<u32, Stowed>, Stowed),
    pub numbers_by_name: HashMap<String, HashMap<u32, Stowed>>,
    pub satchels: (HashMap<u32, Satchel>, Stowed),
    pub satchels_by_name: HashMap<String, HashMap<u32, Satchel>>,
}

/// A tuple struct whose slots are each map, and a map of models with a rule.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Cabinet(
    pub HashMap<u32, String>,
    pub HashMap<bool, String>,
    pub HashMap<Latch, String>,
    pub HashMap<u32, Satchel>,
);

/// [`Cabinet`] in a type that declares a parameter.
#[model_schema(default_types(Stowed = String))]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct StowedCabinet<Stowed>(
    pub HashMap<u32, Stowed>,
    pub HashMap<bool, Stowed>,
    pub HashMap<Latch, Stowed>,
    pub HashMap<u32, Satchel>,
);

/// Each map in a variant's one slot and in the tuple of a variant of several slots, a map of models
/// with a rule in each, and a variant with named members that holds maps in a tuple, under no tag
/// attribute.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum Drawer {
    Bagged(HashMap<u32, Satchel>),
    Flagged(HashMap<bool, String>),
    Keyed(
        HashMap<u32, String>,
        HashMap<bool, String>,
        HashMap<Latch, String>,
    ),
    Latched(HashMap<Latch, String>),
    Numbered(HashMap<u32, String>),
    Packed(HashMap<bool, Satchel>, String),
    Stacked {
        pair: (HashMap<u32, String>, HashMap<bool, String>),
    },
}

/// [`Drawer`] in a type that declares a parameter.
#[model_schema(default_types(Stowed = String))]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum StowedDrawer<Stowed> {
    Bagged(HashMap<u32, Satchel>),
    Flagged(HashMap<bool, Stowed>),
    Keyed(
        HashMap<u32, Stowed>,
        HashMap<bool, Stowed>,
        HashMap<Latch, Stowed>,
    ),
    Latched(HashMap<Latch, Stowed>),
    Numbered(HashMap<u32, Stowed>),
    Packed(HashMap<bool, Satchel>, Stowed),
    Stacked {
        pair: (HashMap<u32, Stowed>, HashMap<bool, Stowed>),
    },
}

/// [`Drawer`] under a tag and a content key.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "kind", content = "held")]
pub enum AdjacentDrawer {
    Bagged(HashMap<u32, Satchel>),
    Flagged(HashMap<bool, String>),
    Keyed(
        HashMap<u32, String>,
        HashMap<bool, String>,
        HashMap<Latch, String>,
    ),
    Latched(HashMap<Latch, String>),
    Numbered(HashMap<u32, String>),
    Packed(HashMap<bool, Satchel>, String),
    Stacked {
        pair: (HashMap<u32, String>, HashMap<bool, String>),
    },
}

/// [`StowedDrawer`] under a tag and a content key.
#[model_schema(default_types(Stowed = String))]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "kind", content = "held")]
pub enum AdjacentStowedDrawer<Stowed> {
    Bagged(HashMap<u32, Satchel>),
    Flagged(HashMap<bool, Stowed>),
    Keyed(
        HashMap<u32, Stowed>,
        HashMap<bool, Stowed>,
        HashMap<Latch, Stowed>,
    ),
    Latched(HashMap<Latch, Stowed>),
    Numbered(HashMap<u32, Stowed>),
    Packed(HashMap<bool, Satchel>, Stowed),
    Stacked {
        pair: (HashMap<u32, Stowed>, HashMap<bool, Stowed>),
    },
}

/// The variants an untagged enum takes, one slot holding a map, a tuple or a map of models with a
/// rule, and named members, each map keyed by a plain enum: serde reads an untagged enum from what
/// it buffers, where it refuses a number key and a `bool` key written as text.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum UntaggedDrawer {
    Latched(HashMap<Latch, String>),
    Paired((HashMap<Latch, String>, u8)),
    Sealed(HashMap<Latch, Satchel>),
    Stacked { pair: (HashMap<Latch, String>, u8) },
}

/// [`UntaggedDrawer`] in a type that declares a parameter.
#[model_schema(default_types(Stowed = String))]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum UntaggedStowedDrawer<Stowed> {
    Latched(HashMap<Latch, Stowed>),
    Paired((HashMap<Latch, Stowed>, u8)),
    Sealed(HashMap<Latch, Satchel>),
    Stacked { pair: (HashMap<Latch, Stowed>, u8) },
}

/// The variant a tag alone takes, one with named members that holds each map in a tuple and as
/// another map's value, and a map of models with a rule in a tuple, keyed by a plain enum as in
/// [`UntaggedDrawer`].
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "kind")]
pub enum TaggedDrawer {
    Stacked {
        latches_by_latch: HashMap<Latch, HashMap<Latch, String>>,
        pair: (HashMap<Latch, String>, u8),
        sealed: (HashMap<Latch, Satchel>, u8),
    },
}

/// [`TaggedDrawer`] in a type that declares a parameter.
#[model_schema(default_types(Stowed = String))]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "kind")]
pub enum TaggedStowedDrawer<Stowed> {
    Stacked {
        latches_by_latch: HashMap<Latch, HashMap<Latch, Stowed>>,
        pair: (HashMap<Latch, Stowed>, u8),
        sealed: (HashMap<Latch, Satchel>, u8),
    },
}
