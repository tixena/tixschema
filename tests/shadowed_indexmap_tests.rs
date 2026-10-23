//! Model types named `IndexMap` and `IndexSet` with the `indexmap` feature on: a field written
//! with either name is read as the type declared here, never as `indexmap`'s map or list.
//!
//! A test binary of its own: a model type's name is known to every expansion that follows it in
//! the crate, where an `indexmap::IndexMap` field would then be read as this type.

#![cfg(all(feature = "serde", feature = "indexmap"))]

// With `bson` on, what a flagged type expands to names the `bson` library.
extern crate bson2 as bson;

tixschema::decode_with_foundation!();
tixschema::mongodb_foundation!();

#[cfg(test)]
#[path = "shadowed_indexmap_tests/tests.rs"]
mod tests;

#[cfg(test)]
#[cfg(feature = "kotlin")]
#[path = "support/kotlin_numbers.rs"]
mod kotlin_numbers;
