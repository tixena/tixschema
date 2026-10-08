//! `#[model_schema(decode_with)]`: `from_bson_with` on structs and enums, built against version 2
//! of the `bson` library.

extern crate bson2 as bson;

#[cfg(test)]
#[cfg(all(feature = "bson", feature = "chrono", feature = "mongodb"))]
#[path = "recovering_decode_bson_tests/tests.rs"]
mod tests;

/// The major version of the `bson` library this binary is built against, as an example printed
/// under it says.
#[cfg(test)]
#[cfg(all(feature = "bson", feature = "chrono", feature = "mongodb"))]
const BSON_MAJOR: u8 = 2;
