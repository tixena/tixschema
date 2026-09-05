//! `header_in`/`header_out` carried over the AMQP transport's own headers channel: a dispatcher
//! that decodes a claimed header before calling the implementation and writes a declared one back
//! into the reply, and a client that mirrors both directions — encoding what it sends, decoding
//! what comes back.
//!
//! The service asks for `http_rest` beside `amqp_rpc`: `http_rest` emits nothing yet, so naming it
//! proves a bound operation is legal on a dual-transport service without depending on anything
//! that transport does not build.
//!
//! Gated on the `serde` feature, which `#[service_schema]` requires: a build without it is refused
//! at the declaration, so a harness declaring a service would not compile at all.
//!
//! The `use` at the foot is what `$crate` reaches: the service's own module, which every message
//! and fault is reached through, and the trait the dispatcher binds.

#![cfg(feature = "serde")]

#[cfg(test)]
#[macro_use]
#[path = "service_schema_header_binding_tests/tests.rs"]
mod tests;

#[cfg(all(test, feature = "serde"))]
#[path = "service_schema_header_binding_tests/amqp_transport.rs"]
mod amqp_transport;

#[cfg(all(test, feature = "serde"))]
#[path = "service_schema_header_binding_tests/amqp_client.rs"]
mod amqp_client;

#[cfg(all(test, feature = "serde"))]
use tests::{DocumentService, document_service_schema};
