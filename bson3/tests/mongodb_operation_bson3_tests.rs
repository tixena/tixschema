//! The operations `#[model_schema(decode_with)]` adds under `mongodb`: the reads, `count`,
//! `insert_one`, the updates and the deletes, with the `Read` the first answer and the
//! `OperationError` they all fail with, built against version 3 of the `bson` library, in the
//! package whose MongoDB driver is built for it.

#[cfg(test)]
#[cfg(feature = "mongodb")]
#[path = "../../tests/mongodb_operation_tests/tests.rs"]
mod tests;

#[cfg(test)]
#[cfg(feature = "mongodb")]
const BSON_MAJOR: u8 = 3;
