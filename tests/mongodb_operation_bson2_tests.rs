//! The operations `#[model_schema(decode_with)]` adds under `mongodb`: the reads, `count`,
//! `insert_one`, the updates and the deletes, with the `Read` the first answer and the
//! `OperationError` they all fail with, built against version 2 of the `bson` library, which this
//! package's MongoDB driver is built for.

extern crate bson2 as bson;

#[cfg(test)]
#[cfg(feature = "mongodb")]
#[path = "mongodb_operation_tests/tests.rs"]
mod tests;

/// The major version of the `bson` library this binary is built against, as a stand-down names it.
#[cfg(test)]
#[cfg(feature = "mongodb")]
const BSON_MAJOR: u8 = 2;
