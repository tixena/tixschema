//! Tests for a field whose type arrives from a `macro_rules!` `$t:ty` metavariable.

#[cfg(test)]
#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
#[path = "substituted_field_type_tests/tests.rs"]
mod tests;
