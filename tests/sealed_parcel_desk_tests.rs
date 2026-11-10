//! What a crate apart seals under `non_exhaustive`, built in this one: the `http_rest`
//! dispatcher's `DefaultFaultHandler`, through `new()` and `default()`, and the `amqp_rpc` server's
//! `Context`, through `new(logger)`.

#![cfg(feature = "serde")]

#[cfg(test)]
#[path = "sealed_parcel_desk_tests/tests.rs"]
mod tests;
