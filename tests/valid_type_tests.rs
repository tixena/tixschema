//! The test binary of `valid_type_tests/`.

#![cfg(feature = "serde")]

#[cfg(test)]
#[path = "valid_type_tests/shapes.rs"]
mod shapes;
#[cfg(test)]
#[path = "valid_type_tests/tests.rs"]
mod tests;
