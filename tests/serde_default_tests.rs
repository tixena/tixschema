//! The test binary of `serde_default_tests/`: a key under `#[serde(default)]` that the payload
//! leaves out, read as its default on every surface.

extern crate alloc;

#[cfg(test)]
#[path = "serde_default_tests/models.rs"]
mod models;

#[cfg(test)]
#[path = "serde_default_tests/readme.rs"]
mod readme;

#[cfg(test)]
#[path = "serde_default_tests/tests.rs"]
mod tests;
