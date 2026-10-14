//! The test binary of `swift_tests/`.

extern crate alloc;

#[cfg(test)]
#[cfg(feature = "swift")]
#[path = "swift_tests/tests.rs"]
mod tests;

#[cfg(test)]
#[cfg(all(feature = "swift", feature = "serde"))]
#[path = "swift_tests/validate.rs"]
mod validate;

#[cfg(test)]
#[cfg(all(feature = "swift", feature = "serde"))]
#[path = "swift_tests/read.rs"]
mod read;

#[cfg(test)]
#[cfg(all(feature = "swift", feature = "serde"))]
#[path = "swift_tests/preprocess.rs"]
mod preprocess;

#[cfg(test)]
#[cfg(all(feature = "swift", feature = "serde"))]
#[path = "swift_tests/sequences.rs"]
mod sequences;

#[cfg(test)]
#[cfg(all(feature = "swift", feature = "serde"))]
#[path = "swift_tests/generic_variants.rs"]
mod generic_variants;
