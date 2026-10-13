//! `#[model_schema(decode_with)]`: `from_bson_with` on structs and enums, built against version 3
//! of the `bson` library, in the package whose MongoDB driver is built for it.

#[cfg(test)]
#[cfg(all(feature = "bson", feature = "chrono", feature = "mongodb"))]
#[path = "../../tests/recovering_decode_bson_tests/tests.rs"]
mod tests;

#[cfg(test)]
#[cfg(all(feature = "bson", feature = "chrono", feature = "mongodb"))]
const BSON_MAJOR: u8 = 3;

/// Reads `document` with the function version 3 of the `bson` library reads a document with.
#[cfg(test)]
#[cfg(all(feature = "bson", feature = "chrono", feature = "mongodb"))]
fn read_document<T>(document: bson::Document) -> Result<T, String>
where
    T: for<'de> serde::Deserialize<'de>,
{
    bson::deserialize_from_document(document).map_err(|refused| refused.to_string())
}
