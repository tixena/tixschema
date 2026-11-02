//! Validation of every shape, in every build: `validate()` with no feature, and with `serde` the
//! read that refuses, the recovering decode and a service's request check.

extern crate alloc;
// With `bson` on, `decode_with_foundation!()` and every flagged type write names of the `bson`
// library.
extern crate bson2 as bson;

#[cfg(feature = "serde")]
tixschema::decode_with_foundation!();

#[cfg(test)]
#[path = "every_build_validation_tests/models.rs"]
mod models;

#[cfg(test)]
#[path = "every_build_validation_tests/validate.rs"]
mod validate;

#[cfg(test)]
#[cfg(feature = "serde")]
#[path = "every_build_validation_tests/read.rs"]
mod read;
