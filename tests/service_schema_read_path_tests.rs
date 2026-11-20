//! A message a transport cannot read answers `failed-validation` naming the dotted path of the
//! value serde refused, on every transport that reads a message, and that path is the one
//! `validate()` names for the same member.

extern crate alloc;

#[cfg(test)]
#[macro_use]
#[path = "service_schema_read_path_tests/tests.rs"]
mod tests;

#[cfg(test)]
#[path = "service_schema_read_path_tests/amqp_transport.rs"]
mod amqp_transport;

#[cfg(test)]
#[path = "service_schema_read_path_tests/http_rest_transport.rs"]
mod http_rest_transport;

#[cfg(test)]
#[path = "service_schema_read_path_tests/ws_transport.rs"]
mod ws_transport;

#[cfg(test)]
use tests::{DocumentService, document_service_schema};
