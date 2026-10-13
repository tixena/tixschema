//! The test binary of `swift_tests/`.

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
