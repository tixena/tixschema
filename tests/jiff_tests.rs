//! Tests for jiff date/time type support.
//!
//! These tests verify that `Date`, `Time`, `DateTime`, and `DateTime`<Tz>
//! types are properly converted to TypeScript types and Zod schemas.

/// What a JSON Schema document admits.
#[cfg(test)]
#[cfg(feature = "jiff")]
#[path = "std_range_tests/documents.rs"]
mod documents;

/// A date under `as_number` in a member of an untagged enum, and what serde and each surface
/// answer of each payload.
#[cfg(test)]
#[cfg(feature = "jiff")]
#[path = "jiff_tests/untagged_millis_models.rs"]
mod untagged_millis_models;

#[cfg(test)]
#[cfg(feature = "jiff")]
#[path = "jiff_tests/tests.rs"]
mod tests;
