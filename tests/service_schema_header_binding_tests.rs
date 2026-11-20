//! `header_in`/`header_out` carried over the AMQP transport's own headers channel: a dispatcher
//! that decodes a claimed header before calling the implementation and writes a declared one back
//! into the reply, and a client that mirrors both directions — encoding what it sends, decoding
//! what comes back.

#[cfg(test)]
#[macro_use]
#[path = "service_schema_header_binding_tests/tests.rs"]
mod tests;

#[cfg(test)]
#[path = "service_schema_header_binding_tests/amqp_transport.rs"]
mod amqp_transport;

#[cfg(test)]
#[path = "service_schema_header_binding_tests/amqp_client.rs"]
mod amqp_client;

#[cfg(test)]
use tests::{DocumentService, document_service_schema};
