//! Types of two crates apart from this one, held and read here: `stored_customers` declares a
//! stored `Customer` this crate's `Invoice` holds, and `invoice_lookups`, whose manifest lists no
//! `mongodb`, declares types that are never stored, one of which a stored type here holds. Each
//! crate writes its own support libraries.

#![cfg(feature = "bson")]

extern crate bson2 as bson;

tixschema::decode_with_foundation!();
tixschema::mongodb_foundation!();

#[cfg(test)]
#[path = "crates_apart_tests/tests.rs"]
mod tests;
