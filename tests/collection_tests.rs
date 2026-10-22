//! The test binary of `collection_tests/`.

extern crate alloc;

#[cfg(test)]
#[path = "collection_tests/tests.rs"]
mod tests;

#[cfg(test)]
#[cfg(all(feature = "serde", feature = "chrono", feature = "jsonschema"))]
#[path = "collection_tests/keyed_maps.rs"]
mod keyed_maps;

#[cfg(test)]
#[cfg(all(feature = "serde", feature = "chrono", feature = "jsonschema"))]
#[path = "std_range_tests/documents.rs"]
mod documents;

#[cfg(test)]
#[cfg(all(feature = "serde", feature = "chrono", feature = "jsonschema"))]
#[path = "collection_tests/keyed_map_documents.rs"]
mod keyed_map_documents;
