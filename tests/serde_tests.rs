//! Tests for how tixschema handles Serde attributes under the `serde` feature — `rename_all`
//! case conventions, field-level rename, combinations of the two, optional fields, and enum
//! `rename_all`, across TypeScript, Zod, and JSON schema generation.

#[cfg(test)]
#[path = "serde_tests/raw_identifiers.rs"]
mod raw_identifiers;

#[cfg(test)]
#[path = "serde_tests/tagged_struct.rs"]
mod tagged_struct;

#[cfg(test)]
#[cfg(feature = "serde")]
#[path = "serde_tests/tests.rs"]
mod tests;
