//! The test binary of `check_tests/`: `check` on a type and on a field, as `validate()` runs it.

extern crate alloc;

#[cfg(test)]
#[path = "check_tests/members.rs"]
mod members;

#[cfg(test)]
#[path = "check_tests/models.rs"]
mod models;

#[cfg(test)]
#[path = "check_tests/read.rs"]
mod read;

#[cfg(test)]
#[path = "check_tests/readme.rs"]
mod readme;

#[cfg(test)]
#[path = "check_tests/table.rs"]
mod table;

#[cfg(test)]
#[path = "check_tests/unchecked.rs"]
mod unchecked;

/// What a draft's issues answer as a `validate()` that makes no value.
pub(crate) fn judged_issues(found: Vec<String>) -> Result<(), Vec<String>> {
    if found.is_empty() { Ok(()) } else { Err(found) }
}
