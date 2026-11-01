//! Tests for the annotation an item writes over a binding it republishes rather than builds.
//!
//! Built with the `serde` feature, in every combination of the others: the fixtures are brands,
//! aliases of brands and one-slot tuple structs over them, and without the feature no struct is a
//! brand. Only the emitted-annotation assertions carry a gate of their own, and each names exactly
//! the surface it reads.

#[cfg(test)]
#[cfg(feature = "serde")]
#[path = "republished_binding_tests/tests.rs"]
mod tests;
