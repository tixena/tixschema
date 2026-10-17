//! The std `Result` serde writes as the object whose one key names its arm: `{"Ok": T}` or
//! `{"Err": E}`, and `null` under an arm of `()`.

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
#[path = "std_result_tests/models.rs"]
mod models;

#[cfg(test)]
#[path = "std_range_tests/documents.rs"]
mod documents;

#[cfg(test)]
#[path = "std_result_tests/surfaces.rs"]
mod surfaces;

#[cfg(test)]
#[path = "std_result_tests/validate.rs"]
mod validate;

#[cfg(test)]
#[path = "std_result_tests/decode.rs"]
mod decode;

#[cfg(test)]
#[path = "std_result_tests/readme.rs"]
mod readme;

#[cfg(test)]
#[path = "std_result_tests/reproduction.rs"]
mod reproduction;
