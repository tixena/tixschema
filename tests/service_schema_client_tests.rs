//! The client `#[service_schema]` generates, driven over transports written by hand: one that
//! hands out prepared answers and records what it was asked to send, and one that loops straight
//! back into the generated dispatcher so both halves of the seam are read against each other.

#![cfg(feature = "serde")]

#[cfg(test)]
#[macro_use]
#[path = "service_schema_client_tests/tests.rs"]
mod tests;

#[cfg(all(test, feature = "serde"))]
#[path = "service_schema_client_tests/amqp_transport.rs"]
mod amqp_transport;

#[cfg(all(test, feature = "serde"))]
#[path = "service_schema_client_tests/amqp_client.rs"]
mod amqp_client;

/// The same client macro a second time, in a module of its own: two clients for one service in one
/// crate, which is what the macro emitting bare items rather than a module of its own is for.
#[cfg(all(test, feature = "serde"))]
#[path = "service_schema_client_tests/spare_amqp_client.rs"]
mod spare_amqp_client;

#[cfg(all(
    test,
    feature = "serde",
    any(feature = "typescript", feature = "zod", feature = "jsonschema")
))]
#[path = "service_schema_client_tests/enrol_amqp_transport.rs"]
mod enrol_amqp_transport;

#[cfg(all(
    test,
    feature = "serde",
    any(feature = "typescript", feature = "zod", feature = "jsonschema")
))]
#[path = "service_schema_client_tests/enrol_amqp_client.rs"]
mod enrol_amqp_client;

// Both halves reach what the service declared through `$crate`, which is this binary's root: a
// service written in a submodule is named here for either expansion to resolve.
#[cfg(all(
    test,
    feature = "serde",
    any(feature = "typescript", feature = "zod", feature = "jsonschema")
))]
use tests::a_bound_the_fields_own_type_declares::{EnrolService, enrol_service_schema};
#[cfg(all(test, feature = "serde"))]
use tests::{ProbeService, probe_service_schema};
