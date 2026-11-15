//! A service names the variant of its fault by its text, so a fault declared above its service,
//! below it or in another crate, with a draft or without one, answers the same status over
//! `http_rest` and the same arm over `mcp`.

#![cfg(feature = "serde")]

extern crate alloc;

#[cfg(test)]
#[macro_use]
#[path = "fault_variant_tests/http_service.rs"]
mod http_service;

#[cfg(test)]
#[path = "fault_variant_tests/http_transport.rs"]
mod http_transport;

#[cfg(test)]
#[macro_use]
#[path = "fault_variant_tests/mcp_service.rs"]
mod mcp_service;

#[cfg(test)]
#[path = "fault_variant_tests/tests.rs"]
mod tests;

// What a service generates reaches the trait and its module through `$crate`, this binary's root.
#[cfg(test)]
use http_service::{StatusService, status_service_schema};
#[cfg(test)]
use mcp_service::{AskingService, asking_service_schema};
