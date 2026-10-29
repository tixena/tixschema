//! `IndexMap` and `IndexSet` fields read from a row: the order a `bson` document keeps is the
//! order each map holds.

use bson::{Bson, Document, doc};
use indexmap::{IndexMap, IndexSet};
use serde::{Deserialize, Serialize};
use tixschema::model_schema;

use super::super::written;
use ranking_schema::{Expected, Issue, Resolution, Stored as _};

#[model_schema(enable = ["decode_with", "mongodb"])]
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
struct Tracked {
    number: i32,
}

#[model_schema()]
type RankedScores = IndexMap<String, i32>;

/// Model types and plain values in an `IndexMap`, one behind an alias seen above, and plain
/// values in an `IndexSet`.
#[model_schema(enable = ["decode_with", "mongodb"])]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
struct Ranking {
    aliased: RankedScores,
    by_name: IndexMap<String, Tracked>,
    labels: IndexSet<String>,
    scores: IndexMap<String, i32>,
}

/// A whole number an older writer stored as text.
fn number_as_text(raw: &mut Document, issue: &Issue<Bson>) -> Resolution {
    let Issue::Invalid {
        path,
        expected: Expected::I32,
        found: Bson::String(text),
        reason: _reason,
    } = issue
    else {
        return Resolution::NotTouched;
    };
    let settled = text
        .parse::<i32>()
        .is_ok_and(|number| raw.set_at(path, Bson::Int32(number)));
    if settled {
        Resolution::Settled
    } else {
        Resolution::Rejected
    }
}

/// Where `issue` sits and what was expected there, less the `bson` library's own wording.
fn placed(issue: &Issue<Bson>) -> String {
    if let Issue::Invalid {
        path,
        expected,
        found: _found,
        reason: _reason,
    } = issue
    {
        format!("{path}: {expected:?}")
    } else {
        format!("{issue:?}")
    }
}

/// The keys of the document `row` holds under `key`, in the order it holds them.
fn keys_under<'row>(row: &'row Document, key: &str) -> Vec<&'row str> {
    row.get_document(key)
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect()
}

fn stored_as_text() -> Document {
    doc! {
        "aliased": { "zeta": "7", "alpha": 1_i32 },
        "byName": { "zeta": { "number": "1" }, "alpha": { "number": 2_i32 } },
        "labels": ["b", "a"],
        "scores": { "zeta": "9", "mid": 5_i32, "alpha": 2_i32 },
    }
}

#[test]
fn a_row_is_written_back_with_each_maps_keys_in_the_order_it_stored_them() {
    let stored_row = doc! {
        "aliased": { "zeta": 7_i32, "alpha": 1_i32 },
        "byName": { "zeta": { "number": 1_i32 }, "alpha": { "number": 2_i32 } },
        "labels": ["b", "a"],
        "scores": { "zeta": 9_i32, "mid": 5_i32, "alpha": 2_i32 },
    };
    let mut calls = 0_u32;
    let read = Ranking::from_bson_with(stored_row.clone(), |_raw, _found| {
        calls += 1;
        ranking_schema::Verdict::Reject
    })
    .unwrap();
    assert_eq!(calls, 0);
    assert_eq!(
        read.scores.keys().collect::<Vec<_>>(),
        ["zeta", "mid", "alpha"]
    );
    assert_eq!(read.labels.iter().collect::<Vec<_>>(), ["b", "a"]);
    let back = written(&read);
    assert_eq!(back, stored_row);
    // Two documents are equal whatever the order of their keys, so the order is read apart.
    assert_eq!(keys_under(&back, "aliased"), ["zeta", "alpha"]);
    assert_eq!(keys_under(&back, "byName"), ["zeta", "alpha"]);
    assert_eq!(keys_under(&back, "scores"), ["zeta", "mid", "alpha"]);
}

#[test]
fn an_entry_and_an_item_are_listed_at_their_own_path_in_the_rows_order() {
    let mut stored_row = stored_as_text();
    stored_row.insert("labels", vec![Bson::from("b"), Bson::Int32(2_i32)]);
    let refused =
        Ranking::from_bson_with(stored_row, |_raw, _found| ranking_schema::Verdict::Reject)
            .unwrap_err();
    assert_eq!(
        refused.issues.iter().map(placed).collect::<Vec<_>>(),
        [
            "aliased.zeta: I32",
            "byName.zeta.number: I32",
            "labels[1]: String",
            "scores.zeta: I32",
        ]
    );
}

#[test]
fn a_fixed_row_keeps_each_maps_keys_in_the_order_it_stored_them() {
    let fixed = Ranking::from_bson_with(stored_as_text(), |raw, found| {
        for issue in found {
            if number_as_text(raw, issue) != Resolution::Settled {
                return ranking_schema::Verdict::Reject;
            }
        }
        ranking_schema::Verdict::Fixed
    })
    .unwrap();
    let piped = Ranking::from_bson_piped(stored_as_text(), &[&number_as_text]).unwrap();
    assert_eq!(fixed, piped);
    assert_eq!(piped.by_name.get("zeta"), Some(&Tracked { number: 1_i32 }));
    let back = written(&piped);
    assert_eq!(keys_under(&back, "aliased"), ["zeta", "alpha"]);
    assert_eq!(keys_under(&back, "byName"), ["zeta", "alpha"]);
    assert_eq!(keys_under(&back, "scores"), ["zeta", "mid", "alpha"]);
    assert_eq!(
        back.get_document("scores").unwrap().get("zeta"),
        Some(&Bson::Int32(9_i32))
    );
}
