//! Tests for `#[serde(flatten)]` intersection-type generation.

extern crate alloc;

#[cfg(test)]
#[cfg(feature = "serde")]
#[path = "flatten_tests/tests.rs"]
mod tests;

#[cfg(test)]
#[cfg(feature = "serde")]
#[path = "flatten_tests/maps.rs"]
mod maps;

#[cfg(test)]
#[cfg(feature = "serde")]
#[path = "flatten_tests/map_schemas.rs"]
mod map_schemas;

#[cfg(test)]
#[cfg(feature = "serde")]
#[path = "std_range_tests/documents.rs"]
mod documents;

#[cfg(test)]
#[cfg(feature = "serde")]
#[path = "flatten_tests/map_documents.rs"]
mod map_documents;
