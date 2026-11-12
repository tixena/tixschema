//! A number written as a float, read for a fixed-width integer by the emitted Swift types that
//! read through a payload, compiled with warnings as errors: a type with a function, a flattened
//! member with and without one, a function that answers a float for an integer and one that
//! answers an integer for a float. Each float is refused at the integer's coding path in serde's
//! words, as serde refuses it; every integer serde reads is read, and so is a float member.

#![cfg(all(feature = "swift", feature = "serde"))]

use serde::Serialize;
use serde::de::DeserializeOwned;

use super::payload_float_models::{
    Cargo, Crossed, Hold, Narrowed, Stowage, Widened, ballast_swift, cargo_swift, crossed_swift,
    hold_swift, inner_swift, narrowed_swift, stowage_swift, widened_swift,
};
use super::run_swift_fitted_parameters::serde_answer;
use super::run_swift_integral_floats::{Compared, HARNESS, Row};
use super::run_swift_read::normalized;
use super::runtime::judged;

/// Names the runtime to run, for a machine that has one somewhere other than `PATH`.
const RUNTIME_VAR: &str = "TIXSCHEMA_SWIFT";

/// The functions the models name, written by hand in Swift.
const FUNCTIONS: &str = r#"
func kept(_ payload: Any) -> Any { payload }

func widened(_ payload: Any) -> Any {
  guard var members = payload as? [String: Any], let count = members["count"] as? Int64, count == 4 else { return payload }
  members["count"] = 2.0
  return members
}

func narrowed(_ payload: Any) -> Any {
  guard var members = payload as? [String: Any], let ratio = members["ratio"] as? Double, ratio == 5.5 else { return payload }
  members["ratio"] = Int64(3)
  return members
}
"#;

/// The numbers serde refuses for an integer, each written with a fraction or an exponent, `-0`
/// among them.
const WRITTEN_AS_FLOATS: [&str; 6] = ["1.0", "1e2", "1E2", "0.0", "-0.0", "-0"];

/// `payload` held to what serde answers of it as a `Read`, a refusal exactly and a read with
/// every number taken as a float.
fn row<Read>(name: String, read_as: &'static str, payload: String) -> Row
where
    Read: DeserializeOwned + Serialize,
{
    let expected = serde_answer::<Read>(&name, &payload);
    let compared = if expected.contains(": refused at '") {
        Compared::Exactly
    } else {
        Compared::AsFloats
    };
    Row {
        compared,
        expected,
        name,
        payload,
        read_as,
    }
}

/// `payload` held to the words serde refuses it with, at `path`, the coding path of the integer:
/// serde names no path for a value a function or a flattened member read, so the path is given.
fn refused<Read>(name: &str, read_as: &'static str, payload: String, path: &str) -> Row
where
    Read: DeserializeOwned + Serialize,
{
    let held = row::<Read>(name.to_owned(), read_as, payload);
    assert!(
        held.expected.contains(": refused at '"),
        "serde reads {}",
        held.payload
    );
    let (_, words) = held.expected.split_once("': ").unwrap();
    Row {
        expected: format!("{name}: refused at '{path}': {words}"),
        ..held
    }
}

/// Each number of [`WRITTEN_AS_FLOATS`] refused as the integer of `read_as`, in the member and in
/// a list's item.
fn refused_floats<Read>(read_as: &'static str, others: &str) -> Vec<Row>
where
    Read: DeserializeOwned + Serialize,
{
    WRITTEN_AS_FLOATS
        .into_iter()
        .flat_map(|number| {
            [
                (
                    format!("{number} in {read_as} count"),
                    format!(r#"{{"count":{number},"counts":[7],"ratio":2.5{others}}}"#),
                    "count",
                ),
                (
                    format!("{number} in {read_as} counts"),
                    format!(r#"{{"count":7,"counts":[7,{number}],"ratio":2.5{others}}}"#),
                    "counts.1",
                ),
            ]
        })
        .map(|(name, payload, path)| refused::<Read>(&name, read_as, payload, path))
        .collect()
}

/// The integers serde reads for the integer, and the floats it reads for the float member.
fn read_numbers<Read>(read_as: &'static str, others: &str) -> Vec<Row>
where
    Read: DeserializeOwned + Serialize,
{
    [
        ("1", "[1]", "2.5"),
        ("0", "[0,4294967295]", "1.0"),
        ("4294967295", "[7]", "3"),
        ("7", "[7]", "1e2"),
    ]
    .into_iter()
    .map(|(count, counts, ratio)| {
        row::<Read>(
            format!("{count} and {ratio} in {read_as}"),
            read_as,
            format!(r#"{{"count":{count},"counts":{counts},"ratio":{ratio}{others}}}"#),
        )
    })
    .collect()
}

/// The same numbers in a flattened member, with and without a function on it, refused at the
/// holder, where a flattened member's refusal is placed.
fn flattened_rows<Read>(read_as: &'static str) -> Vec<Row>
where
    Read: DeserializeOwned + Serialize,
{
    let mut rows: Vec<Row> = WRITTEN_AS_FLOATS
        .into_iter()
        .map(|number| {
            refused::<Read>(
                &format!("{number} in {read_as}"),
                read_as,
                format!(r#"{{"count":{number},"name":"n","ratio":2.5}}"#),
                "",
            )
        })
        .collect();
    rows.extend(
        [("1", "2.5"), ("0", "1.0"), ("4294967295", "3")]
            .into_iter()
            .map(|(count, ratio)| {
                row::<Read>(
                    format!("{count} and {ratio} in {read_as}"),
                    read_as,
                    format!(r#"{{"count":{count},"name":"n","ratio":{ratio}}}"#),
                )
            }),
    );
    rows
}

/// A function that answers a float for an integer is refused, one that answers an integer for a
/// float is read, and a key with a dot is not mistaken for the path it spells.
fn answered_rows() -> Vec<Row> {
    vec![
        refused::<Widened>(
            "a float answered for an integer",
            "Widened",
            r#"{"count":4}"#.to_owned(),
            "count",
        ),
        row::<Widened>(
            "an integer left as it is".to_owned(),
            "Widened",
            r#"{"count":5}"#.to_owned(),
        ),
        row::<Narrowed>(
            "an integer answered for a float".to_owned(),
            "Narrowed",
            r#"{"ratio":5.5}"#.to_owned(),
        ),
        row::<Crossed>(
            "an integer under a key with a dot".to_owned(),
            "Crossed",
            r#"{"a":{"b":2.5},"a.b":7}"#.to_owned(),
        ),
        refused::<Crossed>(
            "a float under a key with a dot",
            "Crossed",
            r#"{"a":{"b":2.5},"a.b":7.0}"#.to_owned(),
            "a.b",
        ),
    ]
}

/// The text of every model compiles with warnings as errors, and the Swift read of every payload
/// answers what serde answers of it.
#[test]
fn a_number_written_as_a_float_is_refused_for_an_integer_through_a_payload_as_serde_refuses_it() {
    let mut rows = refused_floats::<Cargo>("Cargo", "");
    rows.extend(read_numbers::<Cargo>("Cargo", ""));
    rows.extend(flattened_rows::<Hold>("Hold"));
    rows.extend(flattened_rows::<Stowage>("Stowage"));
    rows.extend(answered_rows());
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
        ballast_swift::swift_definition(),
        cargo_swift::swift_definition(),
        crossed_swift::swift_definition(),
        hold_swift::swift_definition(),
        inner_swift::swift_definition(),
        narrowed_swift::swift_definition(),
        stowage_swift::swift_definition(),
        widened_swift::swift_definition(),
        FUNCTIONS.to_owned(),
        HARNESS.to_owned(),
        calls.join("\n"),
    ]
    .join("\n\n");
    let Some(run) = judged(
        "swift-payload-floats",
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
    let printed: Vec<String> = run.stdout.lines().map(|line| format!("{line}\n")).collect();
    assert_eq!(printed.len(), rows.len(), "{}", run.stdout);
    for (row, line) in rows.iter().zip(&printed) {
        match row.compared {
            Compared::AsFloats => assert_eq!(normalized(line), normalized(&row.expected)),
            Compared::Exactly => assert_eq!(line, &row.expected),
        }
    }
}
