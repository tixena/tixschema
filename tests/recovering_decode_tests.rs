//! `#[model_schema(enable = ["decode_with"])]`: `from_value_with` on structs and enums.

extern crate alloc;
// With `bson` on, the library `decode_with_foundation!()` writes names the `bson` library, and so
// does what every flagged type expands to.
extern crate bson2 as bson;

tixschema::decode_with_foundation!();
// One type here lists "mongodb": it gets the MongoDB layer under `mongodb`.
tixschema::mongodb_foundation!();

#[cfg(test)]
#[cfg(feature = "serde")]
#[path = "recovering_decode_tests/tests.rs"]
mod tests;
