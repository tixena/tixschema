//! `#[model_schema(decode_with)]`: `from_value_with` on structs with named fields.

extern crate alloc;
// With `bson` on, what a flagged type expands to names the `bson` library.
extern crate bson2 as bson;

#[cfg(test)]
#[cfg(feature = "serde")]
#[path = "recovering_decode_tests/tests.rs"]
mod tests;
