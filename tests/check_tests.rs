//! The test binary of `check_tests/`: `check` on a type and on a field, as `validate()` runs it.

extern crate alloc;

#[cfg(test)]
#[path = "check_tests/members.rs"]
mod members;

#[cfg(test)]
#[path = "check_tests/models.rs"]
mod models;

#[cfg(test)]
#[path = "check_tests/readme.rs"]
mod readme;

#[cfg(test)]
#[path = "check_tests/table.rs"]
mod table;

#[cfg(test)]
#[path = "check_tests/unchecked.rs"]
mod unchecked;
