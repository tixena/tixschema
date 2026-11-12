//! A holder reads each brand it holds through the brand's unjudged read, so a payload that breaks
//! a held brand's rule is answered with the kind, the field and the words it was answered with
//! before the brands were sealed.

#![cfg(feature = "serde")]

extern crate alloc;

#[cfg(test)]
#[macro_use]
#[path = "sealed_brand_holder_answers_tests/tests.rs"]
mod tests;

#[cfg(all(test, feature = "serde"))]
#[path = "sealed_brand_holder_answers_tests/probe_ws.rs"]
mod probe_ws;

// The macro above reaches the trait and the schema module through `$crate`, which is this
// binary's root, so a private `use` here makes them resolve.
#[cfg(all(test, feature = "serde"))]
use tests::{ProbeService, probe_service_schema};
