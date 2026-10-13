//! The test binary of `dart_tests/`.

#[cfg(test)]
#[cfg(feature = "dart")]
#[path = "dart_tests/tests.rs"]
mod tests;

#[cfg(test)]
#[cfg(all(
    feature = "dart",
    feature = "serde",
    any(feature = "typescript", feature = "zod", feature = "jsonschema")
))]
#[path = "dart_tests/bounded_brands.rs"]
mod bounded_brands;

#[cfg(test)]
#[cfg(all(feature = "dart", feature = "serde"))]
#[path = "dart_tests/validate.rs"]
mod validate;

#[cfg(test)]
#[cfg(all(feature = "dart", feature = "serde"))]
#[path = "dart_tests/read.rs"]
mod read;
