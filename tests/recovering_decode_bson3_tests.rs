//! `#[model_schema(decode_with)]`: `from_bson_with` on structs with named fields, built against
//! version 3 of the `bson` library.

extern crate bson3 as bson;

#[cfg(test)]
#[cfg(all(feature = "bson", feature = "chrono", feature = "mongodb"))]
#[path = "recovering_decode_bson_tests/tests.rs"]
mod tests;
