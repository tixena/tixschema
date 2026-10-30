//! An `i64` is stored as the BSON integer serde writes: the row an operation writes and the value
//! a filter or an update writes hold an `Int64`, each major of the `bson` library reads the row
//! back, and `from_bson_with` reads the integer, and the string of its digits a stored row may
//! hold, with no callback, a string that is no `i64` listed as one issue.

use core::ops::Range;
use std::collections::HashMap;
use wide_row_schema::DecodeWith as _;

use bson::{Bson, Document, doc};
use serde::{Deserialize, Serialize};
use tixschema::model_schema;

use crate::read_document;

/// An `i64` in each place a member holds one, in a row of its own.
#[model_schema(enable = ["decode_with", "mongodb"])]
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
struct WideRow {
    by_name: HashMap<String, i64>,
    history: Vec<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    maybe: Option<i64>,
    paired: (i64, String),
    total: i64,
}

/// A row that holds a [`WideRow`], which a filter and an update write whole.
#[model_schema(enable = ["decode_with", "mongodb"])]
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
struct WideHolderRow {
    held: WideRow,
}

/// An alias of an `i64`, declared above the row that holds it.
#[model_schema()]
type RowRevision = i64;

/// An `i64` through an alias, as a range's bound, as a `Result`'s arm and under a `literal`, in a
/// row of its own.
#[model_schema(enable = ["decode_with", "mongodb"])]
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
struct WideReachedRow {
    aliased: RowRevision,
    #[model_schema_prop(literal = 214)]
    fixed: i64,
    ranged: Range<i64>,
    resulted: Result<i64, String>,
}

/// The row every read below answers.
fn row() -> WideRow {
    WideRow {
        by_name: HashMap::from([("a".to_owned(), -7)]),
        history: vec![1, i64::MAX],
        maybe: Some(i64::MIN),
        paired: (9, "x".to_owned()),
        total: -(1_i64 << 60_i32),
    }
}

/// What the row is stored as.
fn stored() -> Document {
    doc! {
        "byName": { "a": -7_i64 },
        "history": [1_i64, i64::MAX],
        "maybe": i64::MIN,
        "paired": [9_i64, "x"],
        "total": -(1_i64 << 60_i32),
    }
}

/// Reads `stored_row` with a callback that rejects, and answers the read with the issues it was
/// handed and how often it ran.
fn read(stored_row: Document) -> (Option<WideRow>, Vec<String>, u32) {
    let mut handed: Vec<String> = Vec::new();
    let mut calls = 0_u32;
    let answered = WideRow::from_bson_with(stored_row, |_raw, found| {
        calls = calls.saturating_add(1);
        handed.extend(found.iter().map(|issue| format!("{issue:?}")));
        wide_row_schema::Verdict::Reject
    });
    (answered.ok(), handed, calls)
}

#[test]
fn a_row_stores_each_i64_as_the_bson_integer() {
    assert_eq!(row().mongo_written_row().unwrap(), stored());
    assert_eq!(
        read_document::<WideRow>(stored()),
        Ok(row()),
        "the major's own read"
    );
}

#[test]
fn the_integer_and_its_digits_are_read_alike_and_reach_no_callback() {
    assert_eq!(read(stored()), (Some(row()), Vec::new(), 0));
    let as_digits = doc! {
        "byName": { "a": "-7" },
        "history": ["1", "9223372036854775807"],
        "maybe": "-9223372036854775808",
        "paired": ["9", "x"],
        "total": "-1152921504606846976",
    };
    assert_eq!(read(as_digits), (Some(row()), Vec::new(), 0));
}

#[test]
fn a_string_that_is_no_i64_is_one_issue_where_it_is_held() {
    let mut beyond = stored();
    beyond.insert("total", "9223372036854775808");
    let (answered, handed, calls) = read(beyond);
    assert_eq!(answered, None, "the callback rejected");
    assert_eq!(handed.len(), 1, "one issue: {handed:?}");
    // The reason is the `bson` library's wording around serde's, which its two majors word apart.
    assert!(
        handed[0].starts_with(
            "Invalid { path: Path([Key(\"total\")]), expected: I64, found: \
             String(\"9223372036854775808\"), reason: "
        ) && handed[0].contains("invalid type: string \\\"9223372036854775808\\\", expected i64"),
        "the issue: {handed:?}"
    );
    assert_eq!(calls, 1, "the callback runs once");
}

#[test]
fn a_filter_and_an_update_write_each_i64_as_the_bson_integer() {
    let fields = WideHolderRow::MONGO_FIELDS;
    let filter: Document = fields.held().eq(row()).unwrap().into();
    assert_eq!(filter, doc! { "held": { "$eq": Bson::Document(stored()) } });
    let update: Document = fields.held().set(row()).unwrap().into();
    assert_eq!(
        update,
        doc! { "$set": { "held": Bson::Document(stored()) } }
    );
    let nested: Document = fields
        .held()
        .nest(WideRow::MONGO_FIELDS.total().gt(5_i64).unwrap())
        .into();
    assert_eq!(nested, doc! { "held.total": { "$gt": 5_i64 } });
}

#[test]
fn a_row_stores_each_i64_its_holder_reaches_as_the_bson_integer() {
    let reached = WideReachedRow {
        aliased: -(1_i64 << 60_i32),
        fixed: 214,
        ranged: 1..i64::MAX,
        resulted: Ok(7),
    };
    let integers = doc! {
        "aliased": -(1_i64 << 60_i32),
        "fixed": 214_i64,
        "ranged": { "start": 1_i64, "end": i64::MAX },
        "resulted": { "Ok": 7_i64 },
    };
    assert_eq!(
        reached.mongo_written_row().unwrap(),
        integers,
        "the row the operations write"
    );
    assert_eq!(
        read_document::<WideReachedRow>(integers.clone()),
        Ok(reached.clone()),
        "the major's own read"
    );
    let as_digits = doc! {
        "aliased": "-1152921504606846976",
        "fixed": "214",
        "ranged": { "start": "1", "end": "9223372036854775807" },
        "resulted": { "Ok": "7" },
    };
    for stored_row in [integers, as_digits] {
        let mut calls = 0_u32;
        let answered = WideReachedRow::from_bson_with(stored_row, |_raw, _found| {
            calls = calls.saturating_add(1);
            wide_reached_row_schema::Verdict::Reject
        });
        assert_eq!(answered.ok(), Some(reached.clone()), "the read");
        assert_eq!(calls, 0, "no callback");
    }
}
