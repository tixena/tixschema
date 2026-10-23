//! A `u64` and an `i64` a service carries bare, where no member holds it, travel as the JSON number
//! serde writes through the `amqp_rpc` and the `http_rest` dispatcher and client: the message, a
//! header beside it, the reply, its declared error and a header beside the reply. The `http_rest`
//! dispatcher reads a `u64` beyond `i64::MAX` exactly from a path, a header and a body.

#![cfg(feature = "serde")]

#[cfg(test)]
#[macro_use]
#[path = "service_wide_integer_tests/service.rs"]
mod service;

#[cfg(test)]
#[path = "service_wide_integer_tests/amqp_transport.rs"]
mod amqp_transport;

#[cfg(test)]
#[path = "service_wide_integer_tests/amqp_client.rs"]
mod amqp_client;

#[cfg(test)]
#[path = "service_wide_integer_tests/http_rest_transport.rs"]
mod http_rest_transport;

#[cfg(test)]
#[path = "service_wide_integer_tests/http_rest_client.rs"]
mod http_rest_client;

/// The service that carries a bare `u64`, which a build with `swift` or `kotlin` refuses.
#[cfg(test)]
#[cfg(not(any(feature = "swift", feature = "kotlin")))]
#[macro_use]
#[path = "service_wide_integer_tests/wide_service.rs"]
mod wide_service;

#[cfg(test)]
#[cfg(not(any(feature = "swift", feature = "kotlin")))]
#[path = "service_wide_integer_tests/wide_http_rest_transport.rs"]
mod wide_http_rest_transport;

// A transport reaches what the service declared through `$crate`, which is this binary's root.
#[cfg(test)]
use service::{LedgerService, ledger_service_schema};
#[cfg(all(test, not(any(feature = "swift", feature = "kotlin"))))]
use wide_service::{PurgeService, purge_service_schema};
