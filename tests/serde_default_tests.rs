//! The test binary of `serde_default_tests/`: a key under `#[serde(default)]` that the payload
//! leaves out, read as its default on every surface.

extern crate alloc;

#[cfg(test)]
#[path = "serde_default_tests/models.rs"]
mod models;

#[cfg(test)]
#[path = "serde_default_tests/omitted.rs"]
mod omitted;

#[cfg(test)]
#[path = "serde_default_tests/readme.rs"]
mod readme;

#[cfg(test)]
#[path = "serde_default_tests/tests.rs"]
mod tests;

/// What the Swift emitter writes into a type whose reads read a fixed-width integer.
#[cfg(test)]
#[cfg(feature = "swift")]
#[path = "support/swift_fitted.rs"]
mod swift_fitted;

/// The pair of structs the Swift emitter writes for a model type.
#[cfg(test)]
#[cfg(feature = "swift")]
#[path = "support/swift_valid.rs"]
mod swift_valid;

#[cfg(test)]
#[path = "serde_default_tests/variants.rs"]
mod variants;

#[cfg(test)]
#[cfg(feature = "kotlin")]
#[path = "support/kotlin_numbers.rs"]
mod kotlin_numbers;
