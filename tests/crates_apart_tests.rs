//! Types of two crates apart from this one, held and read here: `stored_customers` declares a
//! stored `Customer` this crate's `Invoice` holds, and `invoice_lookups` lists neither `bson` nor
//! `mongodb` in its manifest. Each crate writes its own support libraries.

#![cfg(feature = "bson")]

extern crate bson2 as bson;

tixschema::decode_with_foundation!();
tixschema::mongodb_foundation!();

#[cfg(test)]
#[path = "crates_apart_tests/tests.rs"]
mod tests;
