//! Tests for how tixschema handles Serde attributes under the `serde` feature — `rename_all`
//! case conventions, field-level rename, combinations of the two, optional fields, and enum
//! `rename_all`, across TypeScript, Zod, and JSON schema generation.

#[cfg(test)]
#[path = "serde_tests/raw_identifiers.rs"]
mod raw_identifiers;

#[cfg(test)]
#[path = "serde_tests/tagged_struct.rs"]
mod tagged_struct;

/// The private functions the Kotlin emitter writes into a serializer that refuses a value it read.
#[cfg(test)]
#[cfg(all(feature = "serde", feature = "kotlin"))]
#[path = "support/kotlin_pathed.rs"]
mod kotlin_pathed;

#[cfg(test)]
#[cfg(feature = "serde")]
#[path = "serde_tests/tests.rs"]
mod tests;
