//! A date in a tuple's slot, as a map's value, in a variant's slots and as a type argument, read
//! and written by the emitted Swift types compiled with warnings as errors, held by a type that
//! declares a parameter and by one that declares none. Each payload the serde read reads prints
//! what it prints, each date in the text chrono writes and each date under `as_number` as the
//! number serde writes, and a date that is no date is refused, as the serde read refuses it.

#![cfg(feature = "swift")]

use std::collections::HashMap;

use serde::Serialize;
use serde::de::DeserializeOwned;

use super::held_dates_models::{
    AdjacentCountedTide, AdjacentPlainTide, Chime, Chimed, CountedTide, FirstHolder, Pannier,
    Pendulum, PlainChart, PlainTide, SecondHolder, StowedChart, StowedTicker, Ticker, Tock,
    TwinHolder, UntaggedCountedTide, UntaggedPlainTide, adjacent_counted_tide_swift,
    adjacent_plain_tide_swift, chime_swift, chimed_swift, counted_tide_swift, first,
    first_holder_swift, pannier_swift, pendulum_swift, plain_chart_swift, plain_tide_swift, second,
    second_holder_swift, stowed_chart_swift, stowed_ticker_swift, ticker_swift, tock_swift,
    twin_holder_swift, untagged_counted_tide_swift, untagged_plain_tide_swift,
};
use super::run_swift_read::normalized;
use super::runtime::judged;

/// Names the runtime to run, for a machine that has one somewhere other than `PATH`.
const RUNTIME_VAR: &str = "TIXSCHEMA_SWIFT";

/// What a run reads a payload with, and prints the answer with: the value written back, or each
/// message of a refusal.
const HARNESS: &str = r#"
func row<Read: Codable>(_ name: String, _ type: Read.Type, _ payload: String) {
  do {
    let held = try JSONDecoder().decode(type, from: Data(payload.utf8))
    let encoder = JSONEncoder()
    encoder.outputFormatting = [.sortedKeys]
    print("\(name): read \(String(data: try encoder.encode(held), encoding: .utf8)!)")
  } catch DecodingError.dataCorrupted(let context) {
    print("\(name): refused")
    for line in context.debugDescription.components(separatedBy: "; ") { print("    \(line)") }
  } catch {
    print("\(name): \(error)")
  }
}
"#;

/// The text chrono writes for [`first`].
const FIRST_WRITTEN: &str = r#""2024-01-02T03:04:05.678Z""#;

/// What every refused payload holds in place of [`FIRST_WRITTEN`].
const NO_DATE: &str = r#""yesterday""#;

/// What the Swift read prints for a payload that holds [`NO_DATE`] as a date.
const BAD_DATE: &str = "refused\n    bad ISO8601 date: yesterday";

/// One payload the run reads: its name, the Swift type that reads it, the payload, and what the
/// Swift read is held to.
struct Row {
    expected: String,
    name: &'static str,
    payload: String,
    read_as: &'static str,
}

/// The payload serde writes for `held`, held to what the serde read prints of it: the value
/// written back.
fn read_row<Held>(name: &'static str, read_as: &'static str, held: &Held) -> Row
where
    Held: DeserializeOwned + Serialize,
{
    let payload = serde_json::to_string(held).unwrap();
    let read: Held = serde_json::from_str(&payload).unwrap();
    Row {
        expected: format!("{name}: read {}\n", serde_json::to_string(&read).unwrap()),
        name,
        payload,
        read_as,
    }
}

/// The payload serde writes for `held` with its first date [`NO_DATE`], which the serde read
/// refuses, held to [`BAD_DATE`].
fn refused_row<Held>(name: &'static str, read_as: &'static str, held: &Held) -> Row
where
    Held: DeserializeOwned + Serialize,
{
    let written = serde_json::to_string(held).unwrap();
    assert!(written.contains(FIRST_WRITTEN), "{written}");
    let payload = written.replacen(FIRST_WRITTEN, NO_DATE, 1);
    let refused = serde_json::from_str::<Held>(&payload).err().unwrap();
    assert!(
        refused
            .to_string()
            .starts_with("input contains invalid characters"),
        "{refused}: {payload}"
    );
    Row {
        expected: format!("{name}: {BAD_DATE}\n"),
        name,
        payload,
        read_as,
    }
}

/// [`PlainChart`] with [`first`] in each place a date is held, and [`second`] beside it.
fn plain_chart() -> PlainChart {
    PlainChart {
        at: second(),
        by_name: HashMap::from([("a".to_owned(), first())]),
        by_number: HashMap::from([(1, first()), (2, second())]),
        by_turn: HashMap::from([(-1, first())]),
        dated: Pannier { held: first() },
        marks_by_name: HashMap::from([("m".to_owned(), vec![first(), second()])]),
        maybe_by_name: HashMap::from([("n".to_owned(), None), ("s".to_owned(), Some(first()))]),
        pair: (first(), 1),
        spans: (vec![first(), second()], Some(second())),
        swung: Pendulum {
            first: vec![first()],
            second: None,
        },
    }
}

/// [`StowedChart`] with [`first`] in each place a date is held.
fn stowed_chart() -> StowedChart<String> {
    StowedChart {
        at: second(),
        by_name: HashMap::from([("a".to_owned(), first())]),
        by_number: HashMap::from([(1, first())]),
        dated: Chime {
            first: first(),
            second: "c".to_owned(),
        },
        held: "h".to_owned(),
        pair: (first(), "p".to_owned()),
    }
}

/// The payloads of a struct and of a tuple struct, in a type that declares a parameter and in one
/// that declares none, beside a brand's date.
fn struct_rows() -> Vec<Row> {
    vec![
        read_row(
            "a date in a tuple, as a map's value and as a type argument",
            "PlainChart",
            &plain_chart(),
        ),
        read_row(
            "the same, with empty maps",
            "PlainChart",
            &PlainChart {
                by_name: HashMap::new(),
                by_number: HashMap::new(),
                by_turn: HashMap::new(),
                marks_by_name: HashMap::new(),
                maybe_by_name: HashMap::new(),
                spans: (Vec::new(), None),
                swung: Pendulum {
                    first: Vec::new(),
                    second: Some(second()),
                },
                ..plain_chart()
            },
        ),
        read_row(
            "a type that declares a parameter",
            "StowedChart<String>",
            &stowed_chart(),
        ),
        read_row("a tuple struct", "Tock", &Tock(first(), 2)),
        read_row("a brand", "Chimed", &Chimed(first())),
    ]
}

/// A holder's three fillings of [`Pannier`], each date in the text chrono writes.
fn holder_rows() -> Vec<Row> {
    let dated = || Pannier { held: first() };
    let numbered = || Pannier {
        held: HashMap::from([(1, "a".to_owned())]),
    };
    let paired = || Pannier {
        held: ("p".to_owned(), 1),
    };
    vec![
        read_row(
            "a date, a map and a tuple filling one generic type in a holder",
            "FirstHolder",
            &FirstHolder {
                dated: dated(),
                numbered: numbered(),
                paired: paired(),
            },
        ),
        read_row(
            "the same fillings in a second holder",
            "SecondHolder",
            &SecondHolder {
                dated: dated(),
                numbered: numbered(),
                paired: paired(),
            },
        ),
        read_row(
            "two fields of one holder filling one generic type the same way",
            "TwinHolder",
            &TwinHolder {
                dated: dated(),
                later: Pannier { held: second() },
                numbered: numbered(),
                other_numbered: Pannier {
                    held: HashMap::from([(2, "b".to_owned())]),
                },
                other_paired: Pannier {
                    held: ("q".to_owned(), 2),
                },
                paired: paired(),
            },
        ),
    ]
}

/// The payloads of a variant of one slot and of several, under each tagging that takes them.
fn variant_rows() -> Vec<Row> {
    vec![
        read_row(
            "a variant's member, no tag attribute",
            "PlainTide",
            &PlainTide::Logged { at: first() },
        ),
        read_row(
            "several slots, no tag attribute",
            "PlainTide",
            &PlainTide::Paired(first(), 1),
        ),
        read_row(
            "an optional slot, no tag attribute",
            "PlainTide",
            &PlainTide::Pending(Some(first())),
        ),
        read_row(
            "an optional slot with no date, no tag attribute",
            "PlainTide",
            &PlainTide::Pending(None),
        ),
        read_row(
            "one slot, no tag attribute",
            "PlainTide",
            &PlainTide::Stamped(first()),
        ),
        read_row(
            "a variant's member, a tag and a content key",
            "AdjacentPlainTide",
            &AdjacentPlainTide::Logged { at: first() },
        ),
        read_row(
            "several slots, a tag and a content key",
            "AdjacentPlainTide",
            &AdjacentPlainTide::Paired(first(), 1),
        ),
        read_row(
            "an optional slot, a tag and a content key",
            "AdjacentPlainTide",
            &AdjacentPlainTide::Pending(Some(first())),
        ),
        read_row(
            "an optional slot with no date, a tag and a content key",
            "AdjacentPlainTide",
            &AdjacentPlainTide::Pending(None),
        ),
        read_row(
            "one slot, a tag and a content key",
            "AdjacentPlainTide",
            &AdjacentPlainTide::Stamped(first()),
        ),
        read_row(
            "a variant's member, untagged",
            "UntaggedPlainTide",
            &UntaggedPlainTide::Logged { at: first() },
        ),
        read_row(
            "one slot, untagged",
            "UntaggedPlainTide",
            &UntaggedPlainTide::Stamped(first()),
        ),
    ]
}

/// The payloads of a date under `as_number` in each slot tixschema takes the flag on.
fn counted_rows() -> Vec<Row> {
    vec![
        read_row(
            "one slot under as_number, no tag attribute",
            "CountedTide",
            &CountedTide::Counted(first()),
        ),
        read_row(
            "an optional slot under as_number, no tag attribute",
            "CountedTide",
            &CountedTide::Maybe(Some(first())),
        ),
        read_row(
            "an optional slot under as_number with no date, no tag attribute",
            "CountedTide",
            &CountedTide::Maybe(None),
        ),
        read_row(
            "several slots under as_number, no tag attribute",
            "CountedTide",
            &CountedTide::Spanned(first(), 1),
        ),
        read_row(
            "one slot under as_number, a tag and a content key",
            "AdjacentCountedTide",
            &AdjacentCountedTide::Counted(first()),
        ),
        read_row(
            "an optional slot under as_number, a tag and a content key",
            "AdjacentCountedTide",
            &AdjacentCountedTide::Maybe(Some(first())),
        ),
        read_row(
            "several slots under as_number, a tag and a content key",
            "AdjacentCountedTide",
            &AdjacentCountedTide::Spanned(first(), 1),
        ),
        read_row(
            "one slot under as_number, untagged",
            "UntaggedCountedTide",
            &UntaggedCountedTide::Counted(first()),
        ),
        read_row(
            "a tuple struct under as_number",
            "Ticker",
            &Ticker(first(), 1),
        ),
        read_row(
            "a tuple struct under as_number that declares a parameter",
            "StowedTicker<String>",
            &StowedTicker(first(), "t".to_owned()),
        ),
    ]
}

/// The payloads whose date in each place is no date, each refused by the serde read and by the
/// Swift read.
fn refused_rows() -> Vec<Row> {
    let only = |chart: PlainChart| PlainChart {
        by_name: HashMap::new(),
        by_number: HashMap::new(),
        by_turn: HashMap::new(),
        marks_by_name: HashMap::new(),
        maybe_by_name: HashMap::new(),
        spans: (Vec::new(), None),
        swung: Pendulum {
            first: Vec::new(),
            second: None,
        },
        ..chart
    };
    vec![
        refused_row(
            "a map's value that is no date",
            "PlainChart",
            &PlainChart {
                by_name: HashMap::from([("a".to_owned(), first())]),
                ..only(plain_chart())
            },
        ),
        refused_row(
            "a value of a map under a key Swift wraps that is no date",
            "PlainChart",
            &PlainChart {
                by_number: HashMap::from([(1, first())]),
                ..only(plain_chart())
            },
        ),
        refused_row(
            "a type argument that is no date",
            "PlainChart",
            &only(plain_chart()),
        ),
        refused_row(
            "a slot of a tuple that is no date",
            "StowedChart<String>",
            &StowedChart {
                by_name: HashMap::new(),
                by_number: HashMap::new(),
                dated: Chime {
                    first: second(),
                    second: "c".to_owned(),
                },
                ..stowed_chart()
            },
        ),
        refused_row(
            "one slot that is no date, no tag attribute",
            "PlainTide",
            &PlainTide::Stamped(first()),
        ),
        refused_row(
            "several slots, one no date, a tag and a content key",
            "AdjacentPlainTide",
            &AdjacentPlainTide::Paired(first(), 1),
        ),
    ]
}

/// The text of every model compiles with warnings as errors, and every payload reads and writes in
/// Swift as the serde read reads and writes it: each date in the text chrono writes, each one under
/// `as_number` as a number, and one that is no date refused.
#[test]
fn each_date_held_in_a_slot_a_map_or_a_type_argument_reads_and_writes_in_swift_as_serde_does() {
    let mut rows = struct_rows();
    rows.extend(holder_rows());
    rows.extend(variant_rows());
    rows.extend(counted_rows());
    rows.extend(refused_rows());
    let expected: String = rows.iter().map(|row| row.expected.as_str()).collect();
    assert!(
        expected.contains(r#""pair":["2024-01-02T03:04:05.678Z",1]"#)
            && expected.contains(r#"{"Counted":1704164645678}"#),
        "what the serde read writes: {expected}"
    );
    let calls: Vec<String> = rows
        .iter()
        .map(|row| {
            format!(
                "row(\"{}\", {}.self, #\"{}\"#)",
                row.name, row.read_as, row.payload
            )
        })
        .collect();
    let source = [
        "import Foundation".to_owned(),
        pannier_swift::swift_definition(),
        pendulum_swift::swift_definition(),
        chime_swift::swift_definition(),
        first_holder_swift::swift_definition(),
        second_holder_swift::swift_definition(),
        twin_holder_swift::swift_definition(),
        plain_chart_swift::swift_definition(),
        stowed_chart_swift::swift_definition(),
        plain_tide_swift::swift_definition(),
        adjacent_plain_tide_swift::swift_definition(),
        untagged_plain_tide_swift::swift_definition(),
        tock_swift::swift_definition(),
        chimed_swift::swift_definition(),
        counted_tide_swift::swift_definition(),
        adjacent_counted_tide_swift::swift_definition(),
        untagged_counted_tide_swift::swift_definition(),
        ticker_swift::swift_definition(),
        stowed_ticker_swift::swift_definition(),
        HARNESS.to_owned(),
        calls.join("\n"),
    ]
    .join("\n\n");
    let Some(run) = judged(
        "swift-held-dates",
        RUNTIME_VAR,
        "swift",
        &["-warnings-as-errors"],
        "main.swift",
        &source,
    ) else {
        return;
    };
    assert!(
        run.succeeded,
        "`swift -warnings-as-errors main.swift` failed.\n--- stdout ---\n{}\n--- stderr ---\n{}\n--- source ---\n{source}",
        run.stdout, run.stderr
    );
    assert_eq!(normalized(&run.stdout), normalized(&expected));
}
