//! A `u64` and an `i64` a member writes itself travel as the string of their decimal digits: what
//! serde writes and reads of each position, and what every surface describes.

#![cfg(feature = "serde")]

extern crate alloc;

#[cfg(test)]
#[path = "digit_strings_tests/models.rs"]
mod models;

#[cfg(test)]
#[path = "digit_strings_tests/tests.rs"]
mod tests;
