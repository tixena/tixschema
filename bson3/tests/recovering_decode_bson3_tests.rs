//! `#[model_schema(decode_with)]`: `from_bson_with` on structs and enums, built against version 3
//! of the `bson` library, in the package whose MongoDB driver is built for it.

#[cfg(test)]
#[cfg(all(feature = "bson", feature = "chrono", feature = "mongodb"))]
#[path = "../../tests/recovering_decode_bson_tests/tests.rs"]
mod tests;
