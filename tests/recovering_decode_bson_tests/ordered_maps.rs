//! A field typed with an order-keeping map, read from a row: each issue sits at its key's path, and
//! the map holds its keys in the order the row stored them, which a `bson` document keeps.

#[cfg(feature = "indexmap")]
mod index_map;
