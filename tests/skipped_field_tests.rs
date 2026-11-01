//! A field serde neither writes nor reads is not part of the wire, and no surface describes it.
//!
//! There is nothing to describe without a generation feature, so the module is gated on those. A
//! test of what a surface describes is gated on the `serde` feature too: without it no
//! `#[serde(...)]` attribute is read, and no field is left off the wire.

extern crate alloc;

#[cfg(test)]
#[cfg(any(feature = "typescript", feature = "zod"))]
#[path = "skipped_field_tests/tests.rs"]
mod tests;
