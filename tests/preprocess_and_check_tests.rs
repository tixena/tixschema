//! The test binary of `preprocess_and_check_tests/`: the README's "Preprocess and check" section,
//! held to the parcel it declares and to the tests that hold each cell of its table.

#[cfg(all(
    test,
    feature = "serde",
    any(feature = "typescript", feature = "zod", feature = "jsonschema")
))]
#[path = "preprocess_and_check_tests/models.rs"]
mod models;

#[cfg(test)]
#[path = "preprocess_and_check_tests/readme.rs"]
mod readme;
