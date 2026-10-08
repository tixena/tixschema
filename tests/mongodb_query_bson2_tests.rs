//! The query types `#[model_schema(decode_with)]` adds under `mongodb`: `Filter`, `Update`, and
//! the typed paths that build them, built against version 2 of the `bson` library.

extern crate bson2 as bson;

#[cfg(test)]
#[cfg(all(feature = "chrono", feature = "mongodb"))]
#[path = "mongodb_query_tests/tests.rs"]
mod tests;

// Version 2's hooks that store a chrono date as a BSON date, over a date and over an `Option` of
// one, under the names the shared sources write as a field's `with`.
#[cfg(test)]
#[cfg(all(feature = "chrono", feature = "mongodb"))]
use bson::serde_helpers::{
    chrono_datetime_as_bson_datetime as date_hook,
    chrono_datetime_as_bson_datetime_optional as optional_date_hook,
};
