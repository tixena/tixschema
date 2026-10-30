//! A type's ` ```rust example ` blocks are run through its own `validate()` by a test the macro
//! writes, so `cargo test` fails on an example that breaks a rule of its type.

#[cfg(test)]
#[path = "example_rule_tests/tests.rs"]
mod tests;
