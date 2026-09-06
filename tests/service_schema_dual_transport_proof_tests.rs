//! One service, `transports = ["amqp_rpc", "http_rest"]`, driven both ways in one crate: the
//! `http_rest` dispatcher and client joined through an in-memory plain-terms loop, and the
//! `amqp_rpc` dispatcher and client joined through the headers-capable seam. One implementation
//! answers every call on both loops, which is the whole of what this harness proves — the
//! published surface of every transport task this crate closed is enough to build a working pair
//! of loops, with no name reached that those tasks did not publish.
//!
//! Gated on the `serde` feature, which `#[service_schema]` requires: a build without it is
//! refused at the declaration, so a harness declaring a service would not compile at all.
//!
//! The `use` at the foot is what `$crate` reaches inside every macro this crate expands: the
//! service's own module, which every message and fault is reached through, and the trait the
//! dispatchers bind.

#![cfg(feature = "serde")]

#[cfg(test)]
#[macro_use]
#[path = "service_schema_dual_transport_proof_tests/tests.rs"]
mod tests;

#[cfg(all(test, feature = "serde"))]
#[path = "service_schema_dual_transport_proof_tests/amqp_transport.rs"]
mod amqp_transport;

#[cfg(all(test, feature = "serde"))]
#[path = "service_schema_dual_transport_proof_tests/amqp_client.rs"]
mod amqp_client;

#[cfg(all(test, feature = "serde"))]
#[path = "service_schema_dual_transport_proof_tests/http_rest_transport.rs"]
mod http_rest_transport;

#[cfg(all(test, feature = "serde"))]
#[path = "service_schema_dual_transport_proof_tests/http_rest_client.rs"]
mod http_rest_client;

#[cfg(all(test, feature = "serde"))]
use tests::{DocumentService, document_service_schema};
