//! The messages `#[service_schema]` declares for the operations that named none, read off the
//! three surfaces a client on the far side needs and off the wire itself.

#[cfg(test)]
#[path = "service_schema_message_tests/tests.rs"]
mod tests;

// The transport reaches what the service declared through `$crate`, which is this binary's root.
// The gate is the declaration's own.
#[cfg(all(test, feature = "serde"))]
use tests::{ProbeService, ProbeServiceSchema, probe_service_schema};
