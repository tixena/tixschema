//! A model type named `Path` held by an enum that writes a string check: the check does not reach
//! it, and the declaration builds.
//!
//! A test binary of its own: a model type's name is known to every expansion that follows it in
//! the crate, where a field written `Path` would then be read as this type.

#![cfg(all(feature = "serde", feature = "typescript"))]

extern crate alloc;

#[cfg(test)]
#[path = "shadowed_path_tests/tests.rs"]
mod tests;
