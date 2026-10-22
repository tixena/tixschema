//! A model type named `Map` beside other model types: a field written `Map<K, V>` is read as the
//! type declared here, never as `serde_json`'s map. With the `indexmap` feature off, so are a type
//! named `IndexMap` and one named `IndexSet`. A model type named `Value`, declared below an alias
//! that names it, is the type that alias holds.
//!
//! A test binary of its own: a model type's name is known to every expansion that follows it in
//! the crate, where a field written `Map` or `Value` would then be read as this type.

#![cfg(feature = "serde")]

extern crate alloc;

// With `bson` on, what a flagged type expands to names the `bson` library.
extern crate bson2 as bson;

tixschema::decode_with_foundation!();
tixschema::mongodb_foundation!();

#[cfg(test)]
#[path = "shadowed_map_tests/tests.rs"]
mod tests;
