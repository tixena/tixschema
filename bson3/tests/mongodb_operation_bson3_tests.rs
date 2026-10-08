//! The operations `#[model_schema(decode_with)]` adds under `mongodb`: the reads, `count`,
//! `insert_one`, the updates and the deletes, with the `Read` the first answer and the
//! `OperationError` they all fail with, built against version 3 of the `bson` library, in the
//! package whose MongoDB driver is built for it.

#[cfg(test)]
#[cfg(feature = "mongodb")]
#[path = "../../tests/mongodb_operation_tests/tests.rs"]
mod tests;

/// The major version of the `bson` library this binary is built against, as a stand-down names it.
#[cfg(test)]
#[cfg(feature = "mongodb")]
const BSON_MAJOR: u8 = 3;
