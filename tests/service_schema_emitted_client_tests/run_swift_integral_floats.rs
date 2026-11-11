//! A number written with a fraction or an exponent, `1.0`, `1e2`, `1E2`, `0.0`, `-0.0` and `-0`,
//! read for a fixed-width integer by the emitted Swift types compiled with warnings as errors, as a
//! member, a list's item, a map's value and under an `Option`: each is refused at its coding path
//! in serde's words, as serde refuses it. Every integer serde reads there is read, each refusal
//! Foundation made before is made as it was, and a type parameter filled with a float, a text or
//! an optional integer reads what it read.

#![cfg(all(feature = "swift", feature = "serde"))]

use alloc::collections::BTreeMap;

use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::Value;

use super::integral_float_models::{
    Fathometer, Lighthouse, fathometer_swift, lighthouse_swift, spar_swift,
};
use super::run_swift_fitted_parameters::serde_answer;
use super::run_swift_read::normalized;
use super::runtime::judged;

/// Names the runtime to run, for a machine that has one somewhere other than `PATH`.
const RUNTIME_VAR: &str = "TIXSCHEMA_SWIFT";

/// What a run reads a payload with, and prints the answer with: the value written back with its
/// keys sorted, the coding path and the description of a `dataCorrupted`, and the case, the type
/// and the coding path of any other `DecodingError`, whose description Foundation words differently
/// from one version to the next.
const HARNESS: &str = r#"
func path(_ context: DecodingError.Context) -> String {
  context.codingPath.map { $0.intValue.map(String.init) ?? $0.stringValue }.joined(separator: ".")
}

func row<Read: Codable>(_ name: String, _ type: Read.Type, _ payload: String) {
  do {
    let held = try JSONDecoder().decode(type, from: Data(payload.utf8))
    let encoder = JSONEncoder()
    encoder.outputFormatting = [.sortedKeys]
    print("\(name): read \(String(data: try encoder.encode(held), encoding: .utf8)!)")
  } catch DecodingError.dataCorrupted(let context) {
    print("\(name): refused at '\(path(context))': \(context.debugDescription)")
  } catch DecodingError.typeMismatch(let type, let context) {
    print("\(name): typeMismatch(\(type)) at '\(path(context))'")
  } catch DecodingError.valueNotFound(let type, let context) {
    print("\(name): valueNotFound(\(type)) at '\(path(context))'")
  } catch DecodingError.keyNotFound(let key, let context) {
    print("\(name): keyNotFound(\(key.stringValue)) at '\(path(context))'")
  } catch {
    print("\(name): \(error)")
  }
}
"#;

/// The numbers serde refuses for an integer, each written with a fraction or an exponent, `-0`
/// among them, which serde reads as the float `-0.0`.
const WRITTEN_AS_FLOATS: [&str; 6] = ["1.0", "1e2", "1E2", "0.0", "-0.0", "-0"];

/// Each place [`Fathometer`] holds an integer: the member that holds it, and what the member holds
/// with `{}` in the integer's place.
const PLACES: [(&str, &str); 4] = [
    ("depth", "{}"),
    ("fathoms", "[7,{}]"),
    ("marks", r#"{"a":7,"b":{}}"#),
    ("tide", "{}"),
];

/// How a line the run prints is held to what serde answers.
#[derive(Clone, Copy)]
enum Compared {
    /// With every number read as a float, as Swift writes a whole `Double` with no fraction.
    AsFloats,
    /// Exactly, an integer as it is written.
    Exactly,
}

/// One payload the run reads: its name, the Swift type that reads it, the payload, the line the
/// Swift read is held to, and how.
struct Row {
    compared: Compared,
    expected: String,
    name: String,
    payload: String,
    read_as: &'static str,
}

/// `value` with the keys of every object it holds sorted, as the Swift encoder of the run writes
/// them.
fn sorted(value: Value) -> Value {
    match value {
        Value::Object(entries) => {
            let keyed: BTreeMap<String, Value> = entries
                .into_iter()
                .map(|(key, held)| (key, sorted(held)))
                .collect();
            Value::Object(keyed.into_iter().collect())
        }
        Value::Array(items) => Value::Array(items.into_iter().map(sorted).collect()),
        held @ (Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_)) => held,
    }
}

/// The payload of a [`Fathometer`] that holds `7` in every place but those `written` names, each
/// member there written as it says, and left out where it says `None`.
fn sounded(written: &[(&str, Option<String>)]) -> String {
    let members: Vec<String> = [
        ("depth", "7"),
        ("fathoms", "[7]"),
        ("marks", r#"{"a":7}"#),
        ("tide", "7"),
    ]
    .into_iter()
    .filter_map(|(member, held)| {
        let replaced = written.iter().find(|(named, _)| *named == member);
        match replaced {
            Some((_, Some(text))) => Some(format!("\"{member}\":{text}")),
            Some((_, None)) => None,
            None => Some(format!("\"{member}\":{held}")),
        }
    })
    .collect();
    format!("{{{}}}", members.join(","))
}

/// `payload`, which serde reads, held to what serde writes back of it, its keys sorted.
fn read_row<Read>(name: String, read_as: &'static str, payload: String, compared: Compared) -> Row
where
    Read: DeserializeOwned + Serialize,
{
    let read: Read = serde_json::from_str(&payload).unwrap();
    let written = sorted(serde_json::to_value(&read).unwrap());
    Row {
        compared,
        expected: format!("{name}: read {written}\n"),
        name,
        payload,
        read_as,
    }
}

/// `payload`, which serde refuses, held to the coding path and the words serde refuses it with.
fn refused_row(name: String, payload: String) -> Row {
    let expected = serde_answer::<Fathometer>(&name, &payload);
    assert!(expected.contains(": refused at '"), "serde reads {payload}");
    Row {
        compared: Compared::Exactly,
        expected,
        name,
        payload,
        read_as: "Fathometer",
    }
}

/// `payload`, which serde refuses, held to what Foundation answers of it, as it answered before.
fn foundation_row(name: &str, payload: String, answered: &str) -> Row {
    assert!(
        serde_answer::<Fathometer>(name, &payload).contains(": refused at '"),
        "serde reads {payload}"
    );
    Row {
        compared: Compared::Exactly,
        expected: format!("{name}: {answered}\n"),
        name: name.to_owned(),
        payload,
        read_as: "Fathometer",
    }
}

/// Each number of [`WRITTEN_AS_FLOATS`] in each of [`PLACES`], refused as serde refuses it.
fn refused_as_floats() -> Vec<Row> {
    WRITTEN_AS_FLOATS
        .into_iter()
        .flat_map(|number| {
            PLACES.into_iter().map(move |(member, place)| {
                refused_row(
                    format!("{number} in {member}"),
                    sounded(&[(member, Some(place.replace("{}", number)))]),
                )
            })
        })
        .collect()
}

/// The least and the greatest `i64`, the greatest `u32`, `0` and `-1` in every place an `i64` is
/// held, and the greatest `u32` as the `u32`, each read as serde reads it.
fn read_integers() -> Vec<Row> {
    let mut rows: Vec<Row> = [
        i64::MIN.to_string(),
        i64::MAX.to_string(),
        u32::MAX.to_string(),
        "0".to_owned(),
        "-1".to_owned(),
    ]
    .into_iter()
    .map(|number| {
        let payload = sounded(&[
            ("fathoms", Some(format!("[{number}]"))),
            ("marks", Some(format!(r#"{{"a":{number}}}"#))),
            ("tide", Some(number.clone())),
        ]);
        read_row::<Fathometer>(
            format!("{number} as each i64"),
            "Fathometer",
            payload,
            Compared::Exactly,
        )
    })
    .collect();
    rows.push(read_row::<Fathometer>(
        "the greatest u32 as the u32".to_owned(),
        "Fathometer",
        sounded(&[("depth", Some(u32::MAX.to_string()))]),
        Compared::Exactly,
    ));
    rows.push(read_row::<Fathometer>(
        "an absent option".to_owned(),
        "Fathometer",
        sounded(&[("tide", None)]),
        Compared::Exactly,
    ));
    rows
}

/// The refusals Foundation and `SwiftSchemaFit` made before: a number outside the range of the
/// integer and one with a fraction other than zero, in serde's words, and a value of another type,
/// a `null` and an absent key, in Foundation's.
fn refused_as_before() -> Vec<Row> {
    let mut rows: Vec<Row> = [
        ("depth", "-1", "-1"),
        ("depth", "4294967296", "4294967296"),
        ("depth", "1.5", "1.5"),
        ("depth", "1e20", "1e20"),
        ("fathoms", "1.5", "[7,1.5]"),
        ("marks", "-1.5", r#"{"a":-1.5}"#),
        ("tide", "9223372036854775808", "9223372036854775808"),
    ]
    .into_iter()
    .map(|(member, number, written)| {
        refused_row(
            format!("{number} in {member}"),
            sounded(&[(member, Some(written.to_owned()))]),
        )
    })
    .collect();
    rows.extend([
        foundation_row(
            "a text in depth",
            sounded(&[("depth", Some(r#""7""#.to_owned()))]),
            "typeMismatch(UInt32) at 'depth'",
        ),
        foundation_row(
            "a bool in depth",
            sounded(&[("depth", Some("true".to_owned()))]),
            "typeMismatch(UInt32) at 'depth'",
        ),
        foundation_row(
            "null in depth",
            sounded(&[("depth", Some("null".to_owned()))]),
            "valueNotFound(UInt32) at 'depth'",
        ),
        foundation_row(
            "no depth",
            sounded(&[("depth", None)]),
            "keyNotFound(depth) at ''",
        ),
        foundation_row(
            "a text in fathoms",
            sounded(&[("fathoms", Some(r#"["7"]"#.to_owned()))]),
            "typeMismatch(Int64) at 'fathoms.0'",
        ),
        foundation_row(
            "null in marks",
            sounded(&[("marks", Some(r#"{"a":null}"#.to_owned()))]),
            "valueNotFound(Int64) at 'marks.a'",
        ),
    ]);
    rows
}

/// A type parameter filled with a float reads a number written with an exponent and one with a
/// fraction, one filled with a text reads a text of a float, and one filled with an optional
/// integer reads an integer and `null`.
fn read_parameters() -> Vec<Row> {
    [
        (
            "a float with an exponent, a text and an integer",
            r#"{"light":{"held":1e2},"mark":{"held":1},"name":{"held":"1.0"}}"#,
        ),
        (
            "a float with a fraction, a text and null",
            r#"{"light":{"held":1.50},"mark":{"held":null},"name":{"held":"1e2"}}"#,
        ),
    ]
    .into_iter()
    .map(|(name, payload)| {
        read_row::<Lighthouse>(
            name.to_owned(),
            "Lighthouse",
            payload.to_owned(),
            Compared::AsFloats,
        )
    })
    .collect()
}

/// The text of every model compiles with warnings as errors, and the Swift read of every payload
/// answers what serde answers of it, but for the refusals Foundation words in its own.
#[test]
fn a_number_written_as_a_float_is_refused_for_an_integer_where_serde_refuses_it_in_swift() {
    let mut rows = refused_as_floats();
    rows.extend(read_integers());
    rows.extend(refused_as_before());
    rows.extend(read_parameters());
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
        fathometer_swift::swift_definition(),
        spar_swift::swift_definition(),
        lighthouse_swift::swift_definition(),
        HARNESS.to_owned(),
        calls.join("\n"),
    ]
    .join("\n\n");
    let Some(run) = judged(
        "swift-integral-floats",
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
