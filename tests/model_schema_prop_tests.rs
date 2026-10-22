//! The test binary of `model_schema_prop_tests/`.

extern crate alloc;

#[cfg(test)]
#[path = "model_schema_prop_tests/tests.rs"]
mod tests;

#[cfg(test)]
#[path = "model_schema_prop_tests/literal_models.rs"]
mod literal_models;

#[cfg(test)]
#[path = "model_schema_prop_tests/literals.rs"]
mod literals;
