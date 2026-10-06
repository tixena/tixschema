//! A service declared beside types named after the standard names generated code reaches for:
//! what `#[service_schema]` emits where the service is declared names each of those by its full
//! path, so none of the types declared here is taken for it. The halves a transport places are
//! held to the same in `service_schema_dual_transport_proof_tests` and
//! `service_schema_ws_rpc_proof_tests`, where they are also run.
//!
//! The assertion is that this binary compiles. Gated on the `serde` feature, which
//! `#[service_schema]` requires.

#![cfg(feature = "serde")]

#[cfg(test)]
#[path = "service_schema_shadowed_names_tests/tests.rs"]
mod tests;

// What the service declared is reached through `$crate`, which is this binary's root.
#[cfg(test)]
use tests::{ShadowService, shadow_service_schema};
