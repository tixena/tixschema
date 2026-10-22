//! Tests for chrono date/time type support.
//!
//! These tests verify that `NaiveDate`, `NaiveTime`, `NaiveDateTime`, and `DateTime`<Tz>
//! types are properly converted to TypeScript types and Zod schemas.

/// What a JSON Schema document admits.
#[cfg(test)]
#[cfg(all(feature = "chrono", feature = "jsonschema"))]
#[path = "std_range_tests/documents.rs"]
mod documents;

/// A date under `as_number` in a member of an untagged enum, and what serde and each surface
/// answer of each payload.
#[cfg(test)]
#[cfg(all(feature = "chrono", feature = "serde"))]
#[path = "chrono_tests/untagged_millis_models.rs"]
mod untagged_millis_models;

#[cfg(test)]
#[cfg(feature = "chrono")]
#[path = "chrono_tests/tests.rs"]
mod tests;
