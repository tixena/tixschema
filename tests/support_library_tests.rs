//! The support libraries a crate writes once, with `tixschema::decode_with_foundation!()` and
//! `tixschema::mongodb_foundation!()`: every flagged type of the crate re-exports them and reads
//! from BSON under `bson`, and only a type that lists "mongodb" in `enable` gets the MongoDB layer.
//!
//! The initializers are written last, below the `mod` lines: a crate root takes them anywhere.

// With `bson` on, the `decode_with` library and what a flagged type expands to name the `bson`
// library.
extern crate bson2 as bson;

#[cfg(test)]
#[cfg(feature = "serde")]
#[path = "support_library_tests/tests.rs"]
mod tests;

tixschema::decode_with_foundation!();
tixschema::mongodb_foundation!();
