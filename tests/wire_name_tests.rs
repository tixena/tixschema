//! `wire_name()` on an enum whose variants are all unit variants that serde writes as a string:
//! each one equals what `serde_json` writes, and an enum serde writes any other way gets none.

#![cfg(feature = "serde")]

#[cfg(test)]
#[path = "wire_name_tests/tests.rs"]
mod tests;
