//! A model type named `Vec` beside other model types: what tixschema generates for them names
//! the standard `Vec` by its full path, so the type declared here is never taken for it.
//!
//! A test binary of its own: a model type's name is known to every expansion that follows it in
//! the crate, where a field written `Vec` would then be read as this type.

#![cfg(all(feature = "serde", feature = "typescript"))]

#[cfg(test)]
#[path = "shadowed_vec_tests/tests.rs"]
mod tests;
