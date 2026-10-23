//! An `i64` a member writes as the string of its digits, read by `from_value_with` as serde reads
//! it: from its digits and from a number alike, with a digit string beyond an `i64` listed as one
//! issue at the member that holds it.

use alloc::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::json;
use tixschema::model_schema;

use super::lines;

/// An `i64` in each place a member holds one.
#[model_schema(enable = ["decode_with"])]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
struct DigitLedger {
    by_name: BTreeMap<String, i64>,
    history: Vec<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    maybe: Option<i64>,
    paired: (i64, String),
    total: i64,
}

/// The ledger every read below answers.
fn ledger() -> DigitLedger {
    DigitLedger {
        by_name: BTreeMap::from([("a".to_owned(), -7_i64)]),
        history: vec![1_i64, i64::MAX],
        maybe: Some(i64::MIN),
        paired: (9_i64, "x".to_owned()),
        total: -(1_i64 << 60_i32),
    }
}

/// Reads `stored` with a callback that rejects, and answers the read with every issue listed.
fn read(stored: serde_json::Value) -> (Result<DigitLedger, Vec<String>>, u32) {
    let mut calls = 0_u32;
    let answered = DigitLedger::from_value_with(stored, |_raw, _found| {
        calls = calls.saturating_add(1);
        digit_ledger_schema::Verdict::Reject
    });
    (answered.map_err(|refused| lines(&refused)), calls)
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
    assert_eq!(
        serde_json::to_value(ledger()).unwrap(),
        stored,
        "what serde writes"
    );
    assert_eq!(read(stored), (Ok(ledger()), 0));
}

#[test]
fn a_ledger_of_numbers_is_read_as_serde_reads_it() {
    let stored = json!({
        "byName": { "a": -7_i64 },
        "history": [1_i64, i64::MAX],
        "maybe": i64::MIN,
        "paired": [9_i64, "x"],
        "total": -(1_i64 << 60_i32),
    });
    assert_eq!(read(stored), (Ok(ledger()), 0));
}

#[test]
fn a_digit_string_beyond_an_i64_is_one_issue_at_the_member_that_holds_it() {
    let (answered, calls) = read(json!({
        "byName": {},
        "history": ["1", "9223372036854775808"],
        "paired": ["9", "x"],
        "total": "seven",
    }));
    assert_eq!(
        answered,
        Err(vec![
            "history: invalid: expected Array(I64), found Array [String(\"1\"), \
             String(\"9223372036854775808\")]: invalid value: string \"9223372036854775808\", \
             expected an i64 as a string of its decimal digits"
                .to_owned(),
            "total: invalid: expected I64, found String(\"seven\"): invalid value: string \
             \"seven\", expected an i64 as a string of its decimal digits"
                .to_owned(),
        ])
    );
    assert_eq!(calls, 1, "the callback runs once");
}
