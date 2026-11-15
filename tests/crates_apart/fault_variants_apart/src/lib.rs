//! A crate apart from the test binaries: the faults a service names, which the expansion of that
//! service has never seen.

// No lint table reaches a crate apart, and clippy lints none.
#![deny(warnings)]

use serde::{Deserialize, Serialize};
use tixschema::model_schema;

/// A fault with a draft: one of its variants holds a member.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case", tag = "errorCode")]
pub enum DraftedApart {
    Closed { reason: String },
    NeedsDrive { message: String, url: String },
    NotFound,
}

/// A fault that is its own valid type: every variant is a bare name.
#[model_schema()]
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum PlainApart {
    Closed,
    NotFound,
}
