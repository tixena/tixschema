//! The models whose Zod schemas are read for the `id` their meta carries: a struct with an
//! example, a renamed struct, a tagged enum, a unit struct, a brand with an example and a plain
//! enum, which carry one, a generic struct and two aliases, which carry none of their own, and a
//! holder of the aliases.

use alloc::vec::Vec;
use serde::{Deserialize, Serialize};
use tixschema::model_schema;

/// A depth in fathoms.
/// ```rust example
/// IdentifiedDepthDraft { fathoms: 3 }
/// ```
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct IdentifiedDepth {
    pub fathoms: u32,
}

/// A struct published under another name.
#[model_schema(name = "IdentifiedFathom")]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct IdentifiedSounding {
    pub fathoms: u32,
}

/// An externally tagged enum.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum IdentifiedSignal {
    Beacon(String),
    Silent,
}

/// A unit struct.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct IdentifiedMark;

/// A harbour code.
/// ```rust example
/// IdentifiedCode::new("NYC")?
/// ```
#[model_schema(minLength = 1)]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(transparent)]
pub struct IdentifiedCode(String);

/// A plain enum, which has no draft.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum IdentifiedTone {
    High,
    Low,
}

/// A generic struct with a declared default.
#[model_schema(default_types(Item = String))]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct IdentifiedPage<Item> {
    pub items: Vec<Item>,
}

/// An alias of a model type.
#[model_schema()]
pub type IdentifiedFloor = IdentifiedDepth;

/// An alias of a list.
#[model_schema()]
pub type IdentifiedSoundings = Vec<u32>;

/// A struct that holds both aliases.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct IdentifiedChart {
    pub floor: IdentifiedFloor,
    pub soundings: IdentifiedSoundings,
}

/// The TypeScript and Zod text of every model above.
pub fn identified_texts() -> Vec<String> {
    vec![
        IdentifiedDepth::ts_definition(),
        IdentifiedDepth::zod_schema(),
        IdentifiedSounding::ts_definition(),
        IdentifiedSounding::zod_schema(),
        IdentifiedSignal::ts_definition(),
        IdentifiedSignal::zod_schema(),
        IdentifiedMark::ts_definition(),
        IdentifiedMark::zod_schema(),
        IdentifiedCode::ts_definition(),
        IdentifiedCode::zod_schema(),
        IdentifiedTone::ts_definition(),
        IdentifiedTone::zod_schema(),
        IdentifiedPage::<String>::ts_definition(),
        IdentifiedPage::<String>::zod_schema(),
        identified_floor_schema::Schema::ts_definition(),
        identified_floor_schema::Schema::zod_schema(),
        identified_soundings_schema::Schema::ts_definition(),
        identified_soundings_schema::Schema::zod_schema(),
        IdentifiedChart::ts_definition(),
        IdentifiedChart::zod_schema(),
    ]
}
