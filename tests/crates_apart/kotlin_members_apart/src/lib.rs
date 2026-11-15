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

/// A struct with a bound.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct HeldStuffApart {
    #[model_schema_prop(minLength = 1)]
    pub name: String,
}

/// A brand with a bound.
#[model_schema(minLength = 2)]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(transparent)]
pub struct HeldTagApart(String);

/// A plain enum that names a check.
#[model_schema(check = ["heldUnluckyApart"])]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum HeldPickApart {
    A,
    B,
}

/// The check of [`HeldPickApart`].
pub fn held_unlucky_apart(value: &HeldPickApartDraft) -> Result<(), Vec<String>> {
    if matches!(value, HeldPickApartDraft::B) {
        return Err(vec!["a pick is not B".to_owned()]);
    }
    Ok(())
}

/// A plain enum with no check.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum HeldPlainApart {
    A,
    B,
}

/// A tuple struct with a bound.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct HeldPairApart(#[model_schema_prop(minLength = 1)] pub String, pub u32);

/// A generic struct with a bound.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct HeldCrateApart<T> {
    pub content: T,
    #[model_schema_prop(minLength = 1)]
    pub label: String,
}

/// A tagged enum with a bound.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "kind")]
pub enum HeldShapeApart {
    Named {
        #[model_schema_prop(minLength = 1)]
        name: String,
    },
    Bare,
}

/// An alias of a list of models.
#[model_schema()]
pub type HeldListApart = Vec<HeldStuffApart>;

/// An alias of a map of models.
#[model_schema()]
pub type HeldMapApart = std::collections::HashMap<String, HeldStuffApart>;

/// A generic alias of a list.
#[model_schema()]
pub type HeldPairsApart<T> = Vec<T>;
