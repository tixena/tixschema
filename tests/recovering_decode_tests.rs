//! `#[model_schema(decode_with)]`: `from_value_with` on structs with named fields.

extern crate alloc;

#[cfg(test)]
#[cfg(feature = "serde")]
#[path = "recovering_decode_tests/tests.rs"]
mod tests;
