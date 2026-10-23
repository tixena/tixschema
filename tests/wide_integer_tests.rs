//! A `u64` and an `i64` travel as the JSON number serde writes: what serde writes and reads of
//! each position a member holds one in, and what every surface describes.

#![cfg(feature = "serde")]

extern crate alloc;

#[cfg(test)]
#[path = "wide_integer_tests/models.rs"]
mod models;

#[cfg(test)]
#[path = "wide_integer_tests/tests.rs"]
mod tests;
