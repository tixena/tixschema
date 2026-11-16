//! A crate apart from the test binaries: the types a Dart and a Swift holder flattens, which that
//! holder's expansion has never seen.

// No lint table reaches a crate apart, and clippy lints none.
#![deny(warnings)]

use serde::{Deserialize, Serialize};
use tixschema::model_schema;

/// A struct that refuses no key, flattened beside a struct.
#[model_schema(nonstrict)]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct LateLoopApart {
    pub strand: String,
    pub turns: u32,
}

/// A struct that refuses no key, flattened after a struct.
#[model_schema(nonstrict)]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct LateTailApart {
    pub tail: String,
}

/// A struct that refuses a key it does not declare.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct LateRopeApart {
    pub strand: String,
    pub turns: u32,
}

/// An externally tagged enum.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum LateKnotApart {
    Bowline { loops: u32 },
    Sheet { label: String },
}

/// An externally tagged enum with a rename and an alias.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum LateHitchApart {
    Bowline { loops: u32 },
    #[serde(rename = "sheet")]
    Sheet { label: String },
    #[serde(alias = "hitch")]
    Hitch { turns: u32 },
}

/// The item of the [`CargoApart`] an alias fills.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ParcelApart {
    pub label: String,
}

/// A struct with a list of items, which the alias below fills.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CargoApart<Item> {
    pub items: Vec<Item>,
}

/// An alias of a filled [`CargoApart`], flattened by a struct of another crate.
#[model_schema(name = "ManifestApart")]
pub type ManifestListApart = CargoApart<ParcelApart>;
