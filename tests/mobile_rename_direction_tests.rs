//! The test binary of `mobile_rename_direction_tests/`.

#![cfg(all(
    feature = "serde",
    any(feature = "dart", feature = "swift", feature = "kotlin")
))]

#[cfg(test)]
#[cfg(feature = "dart")]
#[path = "support/dart_integer_read.rs"]
mod dart_integer_read;

#[cfg(test)]
#[path = "mobile_rename_direction_tests/tests.rs"]
mod tests;

#[cfg(test)]
#[cfg(feature = "kotlin")]
#[path = "support/kotlin_numbers.rs"]
mod kotlin_numbers;
