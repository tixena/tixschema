//! `#[service_schema]` end to end: one declaration, implemented and then reached three ways — by
//! its Rust name through the emitted trait, by its wire name through the dispatcher, and by its
//! TypeScript name in what the service publishes.

#[cfg(test)]
#[macro_use]
#[path = "service_schema_tests/tests.rs"]
mod tests;

#[cfg(all(test, feature = "serde"))]
#[path = "service_schema_tests/amqp_transport.rs"]
mod amqp_transport;

// Both halves a transport contributes reach what the service declared through `$crate`, which is
// this binary's root.
#[cfg(all(test, feature = "serde"))]
use tests::{ProbeService, probe_service_schema};
