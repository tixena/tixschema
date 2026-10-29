//! A `u64` and an `i64` travel as the JSON number serde writes: what serde writes and reads of
//! each position a member holds one in, and what every surface describes.

#![cfg(feature = "serde")]

extern crate alloc;

#[cfg(test)]
#[path = "wide_integer_tests/models.rs"]
mod models;

#[cfg(test)]
#[cfg(feature = "typescript")]
#[path = "wide_integer_tests/keyed_models.rs"]
mod keyed_models;

#[cfg(test)]
#[path = "wide_integer_tests/keyed.rs"]
mod keyed;

#[cfg(test)]
#[path = "wide_integer_tests/tests.rs"]
mod tests;

#[cfg(test)]
#[cfg(feature = "zod")]
#[path = "wide_integer_tests/bigint_models.rs"]
mod bigint_models;

#[cfg(test)]
#[cfg(all(feature = "zod", not(any(feature = "swift", feature = "kotlin"))))]
#[path = "wide_integer_tests/bigint_u64_models.rs"]
mod bigint_u64_models;

#[cfg(test)]
#[path = "wide_integer_tests/bigint.rs"]
mod bigint;
