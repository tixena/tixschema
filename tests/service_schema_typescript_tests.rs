//! The TypeScript a `#[service_schema]` service publishes, and the bundle one registration line
//! per artifact produces.

#[cfg(test)]
#[macro_use]
#[path = "service_schema_typescript_tests/tests.rs"]
mod tests;

#[cfg(all(test, feature = "serde", feature = "typescript"))]
#[path = "service_schema_typescript_tests/amqp_transport.rs"]
mod amqp_transport;

// Both halves a transport contributes reach what the service declared through `$crate`, which is
// this binary's root. The gate is the declaration's own.
#[cfg(all(test, feature = "serde"))]
use tests::{ProbeService, probe_service_schema};
