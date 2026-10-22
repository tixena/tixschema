//! The form each value crosses the wire in: what serde answers of each value a caller hands a
//! client, and that serde and the JSON Schema read every payload a client sends.

#![cfg(all(feature = "serde", feature = "chrono"))]

extern crate alloc;

#[cfg(test)]
#[path = "wire_forms_tests/models.rs"]
mod models;

/// What a JSON Schema document admits.
#[cfg(test)]
#[cfg(feature = "jsonschema")]
#[path = "std_range_tests/documents.rs"]
mod documents;

#[cfg(test)]
#[path = "wire_forms_tests/tests.rs"]
mod tests;
