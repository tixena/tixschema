//! The test binary of `ordered_map_tests/`.

#[cfg(test)]
#[cfg(feature = "indexmap")]
#[path = "ordered_map_tests/tests.rs"]
mod tests;

#[cfg(test)]
#[cfg(all(feature = "indexmap", feature = "kotlin"))]
#[path = "support/kotlin_numbers.rs"]
mod kotlin_numbers;
