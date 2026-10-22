//! Tests for chrono date/time type support.
//!
//! These tests verify that `NaiveDate`, `NaiveTime`, `NaiveDateTime`, and `DateTime`<Tz>
//! types are properly converted to TypeScript types and Zod schemas.

/// What a JSON Schema document admits.
#[cfg(test)]
#[cfg(all(feature = "chrono", feature = "jsonschema"))]
#[path = "std_range_tests/documents.rs"]
mod documents;

#[cfg(test)]
#[cfg(feature = "chrono")]
#[path = "chrono_tests/tests.rs"]
mod tests;
