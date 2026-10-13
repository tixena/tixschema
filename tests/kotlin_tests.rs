//! The test binary of `kotlin_tests/`.

#[cfg(test)]
#[cfg(feature = "kotlin")]
#[path = "kotlin_tests/tests.rs"]
mod tests;

#[cfg(test)]
#[cfg(feature = "kotlin")]
#[path = "kotlin_tests/validate.rs"]
mod validate;

/// The two serializers of each type: the read alone, and the type's own, which judges.
#[cfg(test)]
#[cfg(feature = "kotlin")]
#[path = "kotlin_tests/read.rs"]
mod read;
