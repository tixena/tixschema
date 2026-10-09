//! The supporting types `#[service_schema]` generates, driven at runtime: a transport implementing
//! two services' reply handles, and a call error carrying the operation's own error.

#[cfg(test)]
#[macro_use]
#[path = "service_schema_support_tests/tests.rs"]
mod tests;

#[cfg(all(test, feature = "serde"))]
#[path = "service_schema_support_tests/amqp_transport.rs"]
mod amqp_transport;

#[cfg(all(test, feature = "serde"))]
#[path = "service_schema_support_tests/usage_amqp_transport.rs"]
mod usage_amqp_transport;

// A transport's dispatcher reaches what the service declared through `$crate`, which is this
// binary's root: a service written in a submodule is named here for the expansion to resolve.
#[cfg(all(test, feature = "serde"))]
use tests::{SweepService, UsageService, sweep_service_schema, usage_service_schema};
