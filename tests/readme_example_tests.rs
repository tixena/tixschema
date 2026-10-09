//! The test binary of `readme_example_tests/`.

#[cfg(test)]
#[cfg(all(feature = "serde", feature = "jsonschema"))]
#[path = "readme_example_tests/record.rs"]
mod record;

#[cfg(test)]
#[path = "readme_example_tests/tests.rs"]
mod tests;

#[cfg(test)]
#[cfg(all(feature = "serde", feature = "jsonschema"))]
#[path = "readme_example_tests/version.rs"]
mod version;
