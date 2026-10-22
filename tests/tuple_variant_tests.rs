//! Tests for tuple variant enum support.

#[cfg(test)]
#[cfg(feature = "typescript")]
#[cfg(feature = "serde")]
#[path = "tuple_variant_tests/tests.rs"]
mod tests;

#[cfg(test)]
#[cfg(feature = "typescript")]
#[cfg(feature = "serde")]
#[cfg(feature = "jsonschema")]
#[path = "std_range_tests/documents.rs"]
mod documents;
