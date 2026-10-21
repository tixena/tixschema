//! A field typed with an order-keeping map is walked entry by entry, as a `HashMap` field is: each
//! issue at its key's path, through the callback and through the resolvers.

#[cfg(feature = "indexmap")]
mod index_map;
