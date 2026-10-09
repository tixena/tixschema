//! The server macro `#[service_schema]` generates for the `amqp_rpc` transport, compiled for real:
//! a service, its own `serve_until` named at a concrete type, the loop it runs driven with no
//! broker, and the wire framing a reply is built through.

#[cfg(test)]
#[macro_use]
#[path = "service_schema_server_tests/tests.rs"]
mod tests;

#[cfg(all(test, feature = "serde"))]
#[path = "service_schema_server_tests/amqp_server.rs"]
mod amqp_server;

#[cfg(all(test, feature = "serde"))]
#[path = "service_schema_server_tests/wire.rs"]
mod wire;

// The server macro reaches what the service declared through `$crate`, which is this binary's
// root: the service's own module, which every message it answers is built through.
#[cfg(all(test, feature = "serde"))]
use tests::{PingService, ping_service_schema};
