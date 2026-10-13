//! The test binary of `pattern_preprocess_tests/`.

extern crate alloc;

#[cfg(test)]
#[path = "pattern_preprocess_tests/chained.rs"]
mod chained;

#[cfg(test)]
#[path = "pattern_preprocess_tests/models.rs"]
mod models;

#[cfg(test)]
#[path = "pattern_preprocess_tests/read.rs"]
mod read;

#[cfg(test)]
#[path = "pattern_preprocess_tests/readme.rs"]
mod readme;

#[cfg(test)]
#[path = "pattern_preprocess_tests/tests.rs"]
mod tests;

#[cfg(test)]
#[path = "pattern_preprocess_tests/zod.rs"]
mod zod;
