//! A type that lists `"decode_with"` alone is read from BSON in every build with `bson`, alone and
//! held by a stored type, and gets no MongoDB layer.

use serde::{Deserialize, Serialize};
use tixschema::model_schema;

/// Read from BSON, and never stored.
#[model_schema(enable = ["decode_with"])]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Version {
    #[model_schema_prop(minimum = 1)]
    pub number: u32,
}

/// Stored, holding a type that is not.
#[cfg(any(
    feature = "mongodb",
    all(
        feature = "bson",
        any(feature = "typescript", feature = "zod", feature = "jsonschema")
    )
))]
#[model_schema(enable = ["decode_with", "mongodb"])]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Record {
    pub current: Version,
}

/// The read of one row of a collection, by a filter over the rows of `Record`.
#[cfg(feature = "mongodb")]
type FindRecord = for<'held> fn(
    &'held mongodb::Collection<bson::Document>,
    record_schema::Filter<Record>,
) -> record_schema::Read<'held, Record, Option<Record>>;

/// Takes `Record::find_one`, which builds only where the type has it.
#[cfg(feature = "mongodb")]
const fn found_by(_find: FindRecord) {}

#[cfg(feature = "bson")]
#[test]
fn a_type_that_lists_decode_with_alone_decodes_bson() {
    let row = bson::doc! { "number": 7_i32 };
    let read = Version::from_bson_with(row, |_raw, _found| version_schema::Verdict::Reject);
    assert_eq!(read.unwrap().number, 7);
    let piped = Version::from_bson_piped(bson::doc! { "number": 8_i32 }, &[]);
    assert_eq!(piped.unwrap().number, 8);
}

/// A bound is listed where a schema surface publishes its validator.
#[cfg(all(
    feature = "bson",
    any(feature = "typescript", feature = "zod", feature = "jsonschema")
))]
#[test]
fn a_stored_holder_lists_the_issue_of_a_held_type_that_lists_decode_with_alone() {
    let mut calls = 0_u32;
    let row = bson::doc! { "current": { "number": 0_i32 } };
    let read = Record::from_bson_with(row, |_raw, _found| {
        calls += 1;
        record_schema::Verdict::Reject
    });
    assert_eq!(
        read.unwrap_err().to_string().lines().collect::<Vec<&str>>(),
        ["current.number: invalid: expected U32, found Int32(0): too small: minimum is 1, got 0"]
    );
    assert_eq!(calls, 1);
}

#[test]
fn a_type_that_lists_decode_with_alone_has_no_mongodb_layer_in_any_build() {
    #[derive(Debug, Eq, PartialEq)]
    struct Absent;

    trait Fallback {
        const MONGO_FIELDS: Absent = Absent;
        fn find_one() -> Absent {
            Absent
        }
        fn insert_one() -> Absent {
            Absent
        }
    }

    impl<T> Fallback for T {}

    assert_eq!(Version::MONGO_FIELDS, Absent);
    assert_eq!(Version::find_one(), Absent);
    assert_eq!(Version::insert_one(), Absent);
}

/// The stored holder keeps its operations and its typed paths, the held type's value written
/// whole at its key.
#[cfg(feature = "mongodb")]
#[test]
fn a_stored_holder_keeps_its_mongodb_layer() {
    found_by(Record::find_one);
    let current = Record::MONGO_FIELDS
        .current()
        .eq(Version { number: 7 })
        .unwrap();
    assert_eq!(
        current.into_document().to_string(),
        r#"{ "current": { "$eq": { "number": 7 } } }"#
    );
}
