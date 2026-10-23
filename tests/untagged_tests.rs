//! Tests for `#[serde(untagged)]` enum support (TypeScript union / Zod `z.union` / JSON `anyOf`).

extern crate alloc;

// `#[serde(untagged)]` parsing requires the `serde` feature, and the branded `DateString`
// newtype is only emitted when a generation feature (typescript/zod/jsonschema) is enabled.
#[cfg(test)]
#[cfg(feature = "serde")]
#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
#[path = "untagged_tests/tests.rs"]
mod tests;

/// What a JSON Schema document admits.
#[cfg(test)]
#[cfg(all(feature = "serde", feature = "jsonschema"))]
#[path = "std_range_tests/documents.rs"]
mod documents;

/// Untagged enums with a newtype variant over an `Option`, and the variant serde takes for each
/// payload.
#[cfg(test)]
#[cfg(feature = "serde")]
#[path = "untagged_tests/null_models.rs"]
mod null_models;

/// What serde and the JSON Schema answer of each payload of those enums.
#[cfg(test)]
#[cfg(feature = "serde")]
#[path = "untagged_tests/nulls.rs"]
mod nulls;

/// A bound on a member whose type names the enum's own type parameter, held as the member is read.
#[cfg(test)]
#[cfg(feature = "serde")]
#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
#[path = "untagged_tests/parameter_bounds.rs"]
mod parameter_bounds;

/// A rule of a model a member holds, which chooses no variant, and the README's statement of it.
#[cfg(test)]
#[cfg(feature = "serde")]
#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
#[path = "untagged_tests/held_rules.rs"]
mod held_rules;
