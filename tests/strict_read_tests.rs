//! A key no member declares is refused by the Rust read of a model, and by every other surface,
//! unless the model is `nonstrict`.

extern crate alloc;

#[cfg(test)]
#[cfg(feature = "serde")]
#[path = "strict_read_tests/tests.rs"]
mod tests;

#[cfg(test)]
#[cfg(feature = "serde")]
#[path = "strict_read_tests/surfaces.rs"]
mod surfaces;

#[cfg(test)]
#[cfg(feature = "serde")]
#[path = "strict_read_tests/empty_variants.rs"]
mod empty_variants;
