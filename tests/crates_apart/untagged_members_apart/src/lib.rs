//! A crate apart from the test binaries: the untagged enum a holder names, which that holder's
//! expansion has never seen.

// No lint table reaches a crate apart, and clippy lints none.
#![deny(warnings)]

use serde::{Deserialize, Serialize};
use tixschema::model_schema;

/// A refrain sung as words or as a count of beats, which serde tells apart by the keys alone.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum RefrainApart {
    Beats { beats: u32 },
    Words { text: String },
}
