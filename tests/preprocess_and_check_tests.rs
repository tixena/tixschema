//! The test binary of `preprocess_and_check_tests/`: the README's "Preprocess and check" section,
//! held to the parcel it declares and to the tests that hold each cell of its table.

#[cfg(all(test, feature = "serde"))]
#[path = "preprocess_and_check_tests/models.rs"]
mod models;

#[cfg(test)]
#[path = "preprocess_and_check_tests/readme.rs"]
mod readme;

/// What a draft's issues answer as a `validate()` that makes no value.
pub(crate) fn judged_issues(found: Vec<String>) -> Result<(), Vec<String>> {
    if found.is_empty() { Ok(()) } else { Err(found) }
}
