//! An `i64` a member writes as a JSON number, read by `from_value_with` as serde reads it, and from
//! the string of its digits a stored record may hold, with no callback: a string that is no `i64`
//! is one issue where it is held.

use alloc::collections::BTreeMap;
use core::cell::RefCell;
use core::ops::Range;

use serde::{Deserialize, Serialize};
use serde_json::json;
use tixschema::model_schema;

use super::lines;

/// An `i64` in each place a member holds one.
#[model_schema(enable = ["decode_with"])]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
struct WideLedger {
    by_name: BTreeMap<String, i64>,
    history: Vec<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    maybe: Option<i64>,
    paired: (i64, String),
    total: i64,
}

/// An alias of an `i64`, declared above the ledger that holds it.
#[model_schema()]
type LedgerRevision = i64;

/// An `i64` through an alias, as a range's bound, as a `Result`'s arm, under a `literal`, below a
/// lock and beside the type's own parameter.
#[model_schema(enable = ["decode_with"], default_types(Key = String))]
#[derive(Debug, Deserialize, Serialize)]
struct WideReachedLedger<Key: Ord> {
    aliased: LedgerRevision,
    #[model_schema_prop(literal = 214)]
    fixed: i64,
    keyed: BTreeMap<Key, i64>,
    locked: Vec<RefCell<i64>>,
    ranged: Range<i64>,
    resulted: Result<i64, String>,
}

/// The ledger every read below answers.
fn ledger() -> WideLedger {
    WideLedger {
        by_name: BTreeMap::from([("a".to_owned(), -7_i64)]),
        history: vec![1_i64, i64::MAX],
        maybe: Some(i64::MIN),
        paired: (9_i64, "x".to_owned()),
        total: -(1_i64 << 60_i32),
    }
}

/// Reads `stored` with a callback that rejects, and answers the read with every issue listed.
fn read(stored: serde_json::Value) -> (Result<WideLedger, Vec<String>>, u32) {
    let mut calls = 0_u32;
    let answered = WideLedger::from_value_with(stored, |_raw, _found| {
        calls = calls.saturating_add(1);
        wide_ledger_schema::Verdict::Reject
    });
    (answered.map_err(|refused| lines(&refused)), calls)
}

#[test]
fn a_ledger_of_numbers_is_read_as_serde_writes_it_and_reaches_no_callback() {
    let stored = json!({
        "byName": { "a": -7_i64 },
        "history": [1_i64, i64::MAX],
        "maybe": i64::MIN,
        "paired": [9_i64, "x"],
        "total": -(1_i64 << 60_i32),
    });
    assert_eq!(
        serde_json::to_value(ledger()).unwrap(),
        stored,
        "what serde writes"
    );
    assert_eq!(read(stored), (Ok(ledger()), 0));
}

#[test]
fn a_ledger_of_digit_strings_is_read_and_reaches_no_callback() {
    let stored = json!({
        "byName": { "a": "-7" },
        "history": ["1", "9223372036854775807"],
        "maybe": "-9223372036854775808",
        "paired": ["9", "x"],
        "total": "-1152921504606846976",
    });
    assert_eq!(read(stored), (Ok(ledger()), 0));
}

#[test]
fn a_string_that_is_no_i64_is_one_issue_where_it_is_held() {
    let (answered, calls) = read(json!({
        "byName": {},
        "history": ["1", "9223372036854775808"],
        "paired": ["9", "x"],
        "total": "seven",
    }));
    assert_eq!(
        answered,
        Err(vec![
            "history[1]: invalid: expected I64, found String(\"9223372036854775808\"): invalid type: \
             string \"9223372036854775808\", expected i64"
                .to_owned(),
            "total: invalid: expected I64, found String(\"seven\"): invalid type: string \"seven\", \
             expected i64"
                .to_owned(),
        ])
    );
    assert_eq!(calls, 1, "the callback runs once");
}

/// What a ledger of `stored` reads as, written back, with every issue listed and the number of
/// times the callback ran.
fn reached(stored: serde_json::Value) -> (Result<serde_json::Value, Vec<String>>, u32) {
    let mut calls = 0_u32;
    let answered = WideReachedLedger::<String>::from_value_with(stored, |_raw, _found| {
        calls = calls.saturating_add(1);
        wide_reached_ledger_schema::Verdict::Reject
    });
    (
        answered
            .map(|read| serde_json::to_value(read).unwrap())
            .map_err(|refused| lines(&refused)),
        calls,
    )
}

#[test]
fn an_i64_in_each_place_its_holder_reaches_is_read_from_a_number_and_from_its_digits() {
    let numbers = json!({
        "aliased": -(1_i64 << 60_i32),
        "fixed": 214_i64,
        "keyed": { "a": -7_i64 },
        "locked": [i64::MAX],
        "ranged": { "start": 1_i64, "end": 2_i64 },
        "resulted": { "Ok": 7_i64 },
    });
    assert_eq!(
        reached(numbers.clone()),
        (Ok(numbers.clone()), 0),
        "the numbers serde reads, written back"
    );
    let digits = json!({
        "aliased": "-1152921504606846976",
        "fixed": "214",
        "keyed": { "a": "-7" },
        "locked": ["9223372036854775807"],
        "ranged": { "start": "1", "end": "2" },
        "resulted": { "Ok": "7" },
    });
    assert_eq!(
        reached(digits),
        (Ok(numbers), 0),
        "the digits, written back as the numbers serde writes"
    );
}
