//! Two services shaped so that most of what a transport contributes has nothing to do: one whose
//! every operation expects no reply, and one that declares no operation at all.

#![cfg(feature = "serde")]

#[cfg(test)]
#[macro_use]
#[path = "service_schema_lean_service_tests/tests.rs"]
mod tests;

#[cfg(all(test, feature = "serde"))]
#[path = "service_schema_lean_service_tests/bare_amqp_client.rs"]
mod bare_amqp_client;

#[cfg(all(test, feature = "serde"))]
#[path = "service_schema_lean_service_tests/bare_amqp_transport.rs"]
mod bare_amqp_transport;

#[cfg(all(test, feature = "serde"))]
#[path = "service_schema_lean_service_tests/note_amqp_client.rs"]
mod note_amqp_client;

#[cfg(all(test, feature = "serde"))]
#[path = "service_schema_lean_service_tests/note_amqp_transport.rs"]
mod note_amqp_transport;

// Every half reaches what the service declared through `$crate`, which is this binary's root: the
// traits and the services' own modules.
#[cfg(all(test, feature = "serde"))]
use tests::{BareService, NoteService, bare_service_schema, note_service_schema};
