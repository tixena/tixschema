//! A set, `HashSet`, `BTreeSet` and `IndexSet`: an array whose items are distinct. The JSON
//! Schema and Zod refuse an array that repeats an item, and Rust's read keeps one item of it.

#![cfg(all(
    feature = "serde",
    feature = "typescript",
    feature = "zod",
    feature = "jsonschema"
))]

extern crate alloc;

#[cfg(test)]
#[path = "set_tests/models.rs"]
mod models;

#[cfg(test)]
#[path = "set_tests/surfaces.rs"]
mod surfaces;
