//! The four std ranges serde writes as an object: `Range` and `RangeInclusive` as the object of
//! their `start` and their `end`, `RangeFrom` as `start` alone, `RangeTo` as `end` alone.

#![cfg(all(
    feature = "serde",
    feature = "typescript",
    feature = "zod",
    feature = "jsonschema"
))]

extern crate alloc;
// With `bson` on, the library `decode_with_foundation!()` writes names the `bson` library.
extern crate bson2 as bson;

tixschema::decode_with_foundation!();
tixschema::mongodb_foundation!();

#[cfg(test)]
#[path = "std_range_tests/models.rs"]
mod models;

#[cfg(test)]
#[path = "std_range_tests/documents.rs"]
mod documents;

#[cfg(test)]
#[path = "std_range_tests/surfaces.rs"]
mod surfaces;

#[cfg(test)]
#[path = "std_range_tests/validate.rs"]
mod validate;

#[cfg(test)]
#[path = "std_range_tests/decode.rs"]
mod decode;

#[cfg(test)]
#[path = "std_range_tests/readme.rs"]
mod readme;

#[cfg(test)]
#[cfg(not(any(feature = "swift", feature = "kotlin")))]
#[path = "std_range_tests/reproduction.rs"]
mod reproduction;
