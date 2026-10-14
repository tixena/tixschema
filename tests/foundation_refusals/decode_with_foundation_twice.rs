//! The `decode_with` library written twice at one crate root.

extern crate bson2 as bson;

tixschema::decode_with_foundation!();
tixschema::decode_with_foundation!();

fn main() {}
