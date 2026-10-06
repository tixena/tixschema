//! A service whose header arguments and path placeholders are named after locals the transports
//! write around them: `request`, `handler`, `path`, `query`, `headers`, `body`, `message`,
//! `captured`, `sending`. Each transport holds such an argument in a local of its own, so the argument is
//! never read as the transport's.
//!
//! The `http_rest` client is called against the `http_rest` dispatcher, and the handler answers
//! with what it was handed. Gated on the `serde` feature, which `#[service_schema]` requires.

#![cfg(feature = "serde")]

#[cfg(test)]
#[macro_use]
#[path = "service_schema_shadowed_locals_tests/tests.rs"]
mod tests;

#[cfg(test)]
#[path = "service_schema_shadowed_locals_tests/http_rest_client.rs"]
mod http_rest_client;

#[cfg(test)]
#[path = "service_schema_shadowed_locals_tests/http_rest_dispatcher.rs"]
mod http_rest_dispatcher;

// What the service declared is reached through `$crate`, which is this binary's root.
#[cfg(test)]
use tests::{ShadowService, shadow_service_schema};
