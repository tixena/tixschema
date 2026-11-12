//! The field a dispatcher names for a payload that breaks a rule of a model it holds: the path of
//! the broken value, list indices and map keys included, whether the holder checks it after the
//! read or the held model refuses it while serde reads it.

#![cfg(feature = "serde")]

extern crate alloc;

#[cfg(test)]
#[macro_use]
#[path = "service_schema_refusal_path_tests/tests.rs"]
mod tests;

#[cfg(all(test, feature = "serde"))]
#[path = "service_schema_refusal_path_tests/shipping_ws.rs"]
mod shipping_ws;

// The macro above reaches the trait and the schema module through `$crate`, which is this
// binary's root, so a private `use` here makes them resolve.
#[cfg(all(test, feature = "serde"))]
use tests::{Shipping, shipping_schema};
