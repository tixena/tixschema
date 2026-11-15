//! `from_deserializer` on the valid type of every model type: one value read over any serde
//! deserializer and judged once, every broken rule one entry, or the deserializer's own error as
//! the one entry. The types live in a crate apart that lists no `serde_json`.

#![cfg(feature = "serde")]

#[cfg(test)]
#[path = "from_deserializer_tests/tests.rs"]
mod tests;
