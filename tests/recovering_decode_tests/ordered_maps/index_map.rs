//! `IndexMap` and `IndexSet` fields, as they are and behind aliases seen above.

mod declared;

use indexmap::{IndexMap, IndexSet};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tixschema::model_schema;

use self::declared::{RankedEntries, RankedLabels, RankedScores};
#[cfg(feature = "jsonschema")]
use self::declared::{ranked_entries_schema, ranked_labels_schema, ranked_scores_schema};
use super::lines;
use ranking_schema::{Expected, Issue, Resolution};

#[model_schema(decode_with)]
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Tracked {
    number: i32,
}

/// Model types and plain values in an `IndexMap`, and plain values in an `IndexSet`.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
struct Ranking {
    by_name: IndexMap<String, Tracked>,
    labels: IndexSet<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    more: Option<IndexMap<String, Vec<i32>>>,
    scores: IndexMap<String, i32>,
}

/// The same types behind aliases seen above.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct AliasedRanking {
    entries: RankedEntries,
    labels: RankedLabels,
    scores: RankedScores,
}

/// A whole number an older writer stored as text.
fn number_as_text(raw: &mut Value, issue: &Issue<Value>) -> Resolution {
    let Issue::Invalid {
        path,
        expected: Expected::I32,
        found: Value::String(text),
        reason: _reason,
    } = issue
    else {
        return Resolution::NotTouched;
    };
    let settled = text
        .parse::<i32>()
        .is_ok_and(|number| path.set_in_value(raw, Value::from(number)));
    if settled {
        Resolution::Settled
    } else {
        Resolution::Rejected
    }
}

/// What a refused read lists, sorted: a `Value` holds its keys in the order its `serde_json`
/// is built to keep.
fn sorted(mut listed: Vec<String>) -> Vec<String> {
    listed.sort();
    listed
}

#[test]
fn an_entry_of_an_index_map_and_an_item_of_an_index_set_are_listed_at_their_own_path() {
    let stored = json!({
        "byName": { "zeta": { "number": "1" }, "alpha": { "number": 2_i32 } },
        "labels": ["b", 2_i32, "a"],
        "more": { "zeta": [1_i32, "x"], "alpha": 7_i32 },
        "scores": { "zeta": "9", "alpha": 2_i32 },
    });
    let read = Ranking::from_value_with(stored, |_raw, _found| ranking_schema::Verdict::Reject);
    assert_eq!(
        sorted(lines(&read.unwrap_err())),
        [
            "byName.zeta.number: invalid: expected I32, found String(\"1\"): invalid type: string \"1\", expected i32",
            "labels[1]: invalid: expected String, found Number(2): invalid type: integer `2`, expected a string",
            "more.alpha: invalid: expected Array(I32), found Number(7): not an array",
            "more.zeta[1]: invalid: expected I32, found String(\"x\"): invalid type: string \"x\", expected i32",
            "scores.zeta: invalid: expected I32, found String(\"9\"): invalid type: string \"9\", expected i32",
        ]
    );
    let unkeyed = json!({ "byName": [], "labels": {}, "scores": "none" });
    let refused = Ranking::from_value_with(unkeyed, |_raw, _found| ranking_schema::Verdict::Reject);
    assert_eq!(
        lines(&refused.unwrap_err()),
        [
            "byName: invalid: expected Map(Model(\"Tracked\")), found Array []: not an object",
            "labels: invalid: expected Array(String), found Object {}: not an array",
            "scores: invalid: expected Map(I32), found String(\"none\"): not an object",
        ]
    );
}

/// The second read is plain serde: the map holds its keys in the order the value does.
#[test]
fn resolvers_settle_an_index_maps_entries_and_the_map_keeps_the_values_order() {
    let stored = json!({
        "byName": { "zeta": { "number": "1" }, "alpha": { "number": 2_i32 } },
        "labels": ["b", "a"],
        "scores": { "zeta": "9", "mid": 5_i32, "alpha": 2_i32 },
    });
    let keys =
        |held: &Value| -> Vec<String> { held.as_object().unwrap().keys().cloned().collect() };
    let (names, ranked) = (keys(&stored["byName"]), keys(&stored["scores"]));
    let read = Ranking::from_value_piped(stored, &[&number_as_text]).unwrap();
    assert_eq!(read.by_name.keys().cloned().collect::<Vec<_>>(), names);
    assert_eq!(read.scores.keys().cloned().collect::<Vec<_>>(), ranked);
    assert_eq!(read.by_name.get("zeta"), Some(&Tracked { number: 1_i32 }));
    assert_eq!(read.scores.get("zeta"), Some(&9_i32));
    assert_eq!(read.labels.iter().collect::<Vec<_>>(), ["b", "a"]);
    assert_eq!(read.more, None);
}

#[test]
fn an_entry_no_resolver_settles_is_the_one_a_refused_read_holds() {
    let stored = json!({
        "byName": {},
        "labels": [],
        "scores": { "zeta": "nine", "alpha": "2" },
    });
    let refused = Ranking::from_value_piped(stored, &[&number_as_text]).unwrap_err();
    assert_eq!(
        lines(&refused),
        [
            "scores.zeta: invalid: expected I32, found String(\"nine\"): invalid type: string \"nine\", expected i32"
        ]
    );
}

/// The issue sits at the entry or the item it is in, and names what that holds.
#[test]
fn an_alias_seen_above_is_walked_as_the_index_map_or_the_index_set_it_names() {
    let stored = json!({
        "entries": { "zeta": [{ "number": 1_i32 }, { "number": "2" }] },
        "labels": ["b", true],
        "scores": { "zeta": "x", "alpha": 2_i32 },
    });
    let read = AliasedRanking::from_value_with(stored, |_raw, _found| {
        aliased_ranking_schema::Verdict::Reject
    });
    assert_eq!(
        sorted(lines(&read.unwrap_err())),
        [
            "entries.zeta[1].number: invalid: expected I32, found String(\"2\"): invalid type: string \"2\", expected i32",
            "labels[1]: invalid: expected String, found Bool(true): invalid type: boolean `true`, expected a string",
            "scores.zeta: invalid: expected I32, found String(\"x\"): invalid type: string \"x\", expected i32",
        ]
    );
    let whole = json!({
        "entries": { "zeta": [{ "number": 1_i32 }] },
        "labels": ["b", "a"],
        "scores": { "zeta": 1_i32 },
    });
    let mut calls = 0_u32;
    let kept = AliasedRanking::from_value_with(whole, |_raw, _found| {
        calls += 1;
        aliased_ranking_schema::Verdict::Reject
    })
    .unwrap();
    assert_eq!(calls, 0);
    assert_eq!(kept.labels.iter().collect::<Vec<_>>(), ["b", "a"]);
    assert_eq!(kept.scores.get("zeta"), Some(&1_i32));
    assert_eq!(
        kept.entries.get("zeta"),
        Some(&vec![Tracked { number: 1_i32 }])
    );
}
