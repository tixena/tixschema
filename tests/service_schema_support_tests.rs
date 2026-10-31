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
#[path = "service_schema_support_tests/billing_amqp_transport.rs"]
mod billing_amqp_transport;

// A transport's dispatcher reaches what the service declared through `$crate`, which is this
// binary's root: a service written in a submodule is named here for the expansion to resolve.
#[cfg(all(test, feature = "serde"))]
use tests::{BillingService, SweepService, billing_service_schema, sweep_service_schema};
