//! `#[model_schema(decode_with)]`: `from_bson_with` on structs with named fields, built against
//! version 2 of the `bson` library.

extern crate bson2 as bson;

#[cfg(test)]
#[cfg(all(feature = "bson", feature = "chrono", feature = "mongodb"))]
#[path = "recovering_decode_bson_tests/tests.rs"]
mod tests;
