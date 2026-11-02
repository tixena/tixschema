//! Models that derive serde and carry `#[serde(...)]` attributes, built with the `serde` feature
//! off.
//!
//! Without the feature no surface reads a `#[serde(...)]` attribute: TypeScript and Zod write each
//! member as its Rust name in camelCase, no key is optional or left unwritten because of
//! `default`, `skip_serializing_if` or `skip_serializing`, no variant is left out because of
//! `skip`, and an enum is written as it is with no attribute, `untagged` or `tag`. The Dart, Swift
//! and Kotlin clients write every key they read. `validate()` and the JSON Schema document name
//! each member by its Rust name.

#![cfg(all(
    not(feature = "serde"),
    any(
        feature = "typescript",
        feature = "zod",
        feature = "dart",
        feature = "swift",
        feature = "kotlin"
    )
))]

#[cfg(test)]
#[path = "serde_without_the_flag_tests/tests.rs"]
mod tests;
