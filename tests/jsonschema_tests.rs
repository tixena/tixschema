//! Tests for JSON Schema generation — the document `json_schema()` answers for a type that lists
//! `enable = ["jsonschema"]`, covering primitives, optional fields, arrays, maps, nested structs,
//! enums, numeric types, serde rename attributes, and `ObjectId` support.

#[cfg(test)]
#[cfg(feature = "serde")]
#[path = "jsonschema_tests/tests.rs"]
mod tests;
