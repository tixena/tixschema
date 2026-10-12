//! The test binary of `mobile_rename_direction_tests/`.

#![cfg(all(
    feature = "serde",
    any(feature = "dart", feature = "swift", feature = "kotlin")
))]

#[cfg(test)]
#[path = "mobile_rename_direction_tests/tests.rs"]
mod tests;
