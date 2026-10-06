//! A module named `core` and one named `std` beside model types and a service: what tixschema
//! generates there starts every `core` and `std` path at the crate root, so neither module is
//! taken for the crate it is named after.
//!
//! The assertion is that this binary compiles. Gated on the `serde` feature, which
//! `#[service_schema]` requires.

#![cfg(feature = "serde")]

#[cfg(test)]
#[path = "shadowed_core_module_tests/tests.rs"]
mod tests;

// What the service declared is reached through `$crate`, which is this binary's root.
#[cfg(test)]
use tests::{BalanceService, balance_service_schema};
