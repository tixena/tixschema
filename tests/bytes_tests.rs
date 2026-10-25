//! A `Vec<u8>` is the bytes of a payload: what each surface publishes for it and for each sequence
//! that stays an array.

#![cfg(all(feature = "serde", any(feature = "typescript", feature = "dart")))]

extern crate alloc;

#[cfg(test)]
#[path = "bytes_tests/models.rs"]
mod models;

#[cfg(test)]
#[cfg(feature = "typescript")]
#[path = "bytes_tests/text.rs"]
mod text;

#[cfg(test)]
#[path = "bytes_tests/rust.rs"]
mod rust;

#[cfg(test)]
#[cfg(feature = "dart")]
#[path = "bytes_tests/dart.rs"]
mod dart;
