//! Tests for type alias support — semantic types (e.g. `DocumentId`, `UserId`) distinct in
//! TypeScript but backed by primitives in Rust, covering basic, optional, generic and nested
//! aliases, alias fields and collections, and module structure across all three surfaces.

extern crate alloc;

#[cfg(test)]
#[path = "semantic_types_tests/tests.rs"]
mod tests;

/// What a JSON Schema document admits.
#[cfg(test)]
#[cfg(all(feature = "serde", feature = "jsonschema"))]
#[path = "std_range_tests/documents.rs"]
mod documents;

/// Aliases of an `Option` held outside a field, and the payloads serde writes for them.
#[cfg(test)]
#[cfg(feature = "serde")]
#[path = "semantic_types_tests/option_alias_models.rs"]
mod option_alias_models;

/// What serde, TypeScript, Zod and the JSON Schema answer of those aliases.
#[cfg(test)]
#[cfg(feature = "serde")]
#[path = "semantic_types_tests/option_aliases.rs"]
mod option_aliases;
