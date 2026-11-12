//! A type parameter filled with a fixed-width integer, the widest integer Swift reads and a model,
//! read by the emitted Swift types compiled with warnings as errors, in each place a type holds
//! one. A number that does not fit is refused at the coding path, and in the words, of the serde
//! read, whose path is the field the Rust dispatcher names: `{"inner":{"held":-1}}` read as a
//! [`Mooring`] is refused at `inner.held` with ``invalid value: integer `-1`, expected u32``.
//! Every payload serde reads is read and written back as serde writes it back.

#![cfg(all(feature = "swift", feature = "serde"))]

use std::collections::HashMap;

use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::Value;
use serde_path_to_error::Segment;

use super::fitted_parameters_models::{
    Bollard, BollardDraft, CleatDraft, FenderDraft, HawserDraft, KnotDraft, KnotMooringDraft,
    Mooring, MooringDraft, Quay, QuayDraft, TetherDraft, Wharf, WharfDraft, WideMooring,
    WideMooringDraft, bollard_swift, cleat_swift, fender_swift, hawser_swift, knot_mooring_swift,
    knot_swift, mooring_swift, quay_swift, tether_swift, wharf_swift, wide_mooring_swift,
};
use super::run_swift_read::normalized;
use super::runtime::judged;

/// Names the runtime to run, for a machine that has one somewhere other than `PATH`.
const RUNTIME_VAR: &str = "TIXSCHEMA_SWIFT";

/// What a run reads a payload with, and prints the answer with: the value written back, or the
/// coding path and the description of a refusal.
const HARNESS: &str = r#"
func row<Read: Codable>(_ name: String, _ type: Read.Type, _ payload: String) {
  do {
    let held = try JSONDecoder().decode(type, from: Data(payload.utf8))
    let encoder = JSONEncoder()
    encoder.outputFormatting = [.sortedKeys]
    print("\(name): read \(String(data: try encoder.encode(held), encoding: .utf8)!)")
  } catch DecodingError.dataCorrupted(let context) {
    let path = context.codingPath.map { $0.intValue.map(String.init) ?? $0.stringValue }
    print("\(name): refused at '\(path.joined(separator: "."))': \(context.debugDescription)")
  } catch {
    print("\(name): \(error)")
  }
}
"#;

/// One payload the run reads: its name, the Swift type that reads it, the payload, and what the
/// Swift read is held to.
struct Row {
    expected: String,
    name: &'static str,
    payload: String,
    read_as: &'static str,
}

/// What serde answers of `payload` read as `Read`: the value written back, or the dotted path of
/// the value it refused, as the Rust dispatcher names it, and its words.
pub fn serde_answer<Read>(name: &str, payload: &str) -> String
where
    Read: DeserializeOwned + Serialize,
{
    let value: Value = serde_json::from_str(payload).unwrap();
    match serde_path_to_error::deserialize::<_, Read>(value) {
        Ok(read) => format!("{name}: read {}\n", serde_json::to_string(&read).unwrap()),
        Err(refused) => {
            let steps: Vec<String> = refused
                .path()
                .iter()
                .filter_map(|step| match step {
                    Segment::Seq { index } => Some(index.to_string()),
                    Segment::Map { key } => Some(key.clone()),
                    Segment::Enum { variant } => Some(variant.clone()),
                    Segment::Unknown => None,
                })
                .collect();
            format!(
                "{name}: refused at '{}': {}\n",
                steps.join("."),
                refused.inner()
            )
        }
    }
}

/// The payload serde writes for `held`, held to what serde answers of it.
fn read_row<Read>(name: &'static str, read_as: &'static str, held: &Read) -> Row
where
    Read: DeserializeOwned + Serialize,
{
    let payload = serde_json::to_string(held).unwrap();
    Row {
        expected: serde_answer::<Read>(name, &payload),
        name,
        payload,
        read_as,
    }
}

/// `payload`, which serde refuses, held to what serde answers of it.
fn refused_row<Read>(name: &'static str, read_as: &'static str, payload: &str) -> Row
where
    Read: DeserializeOwned + Serialize,
{
    let expected = serde_answer::<Read>(name, payload);
    assert!(expected.contains(": refused at '"), "serde reads {payload}");
    Row {
        expected,
        name,
        payload: payload.to_owned(),
        read_as,
    }
}

/// The payloads serde reads, each type parameter filled at its ends.
fn read_rows() -> Vec<Row> {
    vec![
        read_row(
            "a u32 that fits",
            "Mooring",
            &MooringDraft {
                inner: TetherDraft { held: 4 }.validate().unwrap(),
            },
        ),
        read_row(
            "a u32 at its ceiling",
            "Mooring",
            &MooringDraft {
                inner: TetherDraft { held: u32::MAX }.validate().unwrap(),
            },
        ),
        read_row(
            "an i64 at its floor",
            "WideMooring",
            &WideMooringDraft {
                inner: TetherDraft { held: i64::MIN }.validate().unwrap(),
            },
        ),
        read_row(
            "a model",
            "KnotMooring",
            &KnotMooringDraft {
                inner: TetherDraft {
                    held: KnotDraft {
                        label: "a".to_owned(),
                    }
                    .validate()
                    .unwrap(),
                }
                .validate()
                .unwrap(),
            },
        ),
        read_row(
            "a u8 in each place",
            "Bollard",
            &BollardDraft {
                lines: HawserDraft {
                    keyed: HashMap::from([("a".to_owned(), u8::MAX)]),
                    listed: vec![0, 1],
                    maybe: Some(7),
                }
                .validate()
                .unwrap(),
            },
        ),
        read_row(
            "a u8 under an option, none",
            "Bollard",
            &BollardDraft {
                lines: HawserDraft {
                    keyed: HashMap::new(),
                    listed: Vec::new(),
                    maybe: None,
                }
                .validate()
                .unwrap(),
            },
        ),
        read_row(
            "a u16 in a variant's one slot",
            "Wharf",
            &WharfDraft {
                cleat: CleatDraft::Bare(u16::MAX).validate().unwrap(),
            },
        ),
        read_row(
            "a u16 in a variant of two",
            "Wharf",
            &WharfDraft {
                cleat: CleatDraft::Paired(0, "a".to_owned()).validate().unwrap(),
            },
        ),
        read_row(
            "an i8 in a tuple struct",
            "Quay",
            &QuayDraft {
                fender: FenderDraft(i8::MIN).validate().unwrap(),
            },
        ),
    ]
}

/// The payloads serde refuses: a number beyond the integer, under its floor, and with a fraction.
fn refused_rows() -> Vec<Row> {
    vec![
        refused_row::<Mooring>(
            "a negative number for a u32",
            "Mooring",
            r#"{"inner":{"held":-1}}"#,
        ),
        refused_row::<Mooring>(
            "a number beyond a u32",
            "Mooring",
            r#"{"inner":{"held":4294967296}}"#,
        ),
        refused_row::<Mooring>(
            "a fraction for a u32",
            "Mooring",
            r#"{"inner":{"held":1.5}}"#,
        ),
        refused_row::<WideMooring>(
            "a number beyond an i64",
            "WideMooring",
            r#"{"inner":{"held":9223372036854775808}}"#,
        ),
        refused_row::<Bollard>(
            "a number beyond a u8 as a map's value",
            "Bollard",
            r#"{"lines":{"keyed":{"a":256},"listed":[]}}"#,
        ),
        refused_row::<Bollard>(
            "a negative number for a u8 in a list",
            "Bollard",
            r#"{"lines":{"keyed":{},"listed":[1,-1]}}"#,
        ),
        refused_row::<Bollard>(
            "a number beyond a u8 under an option",
            "Bollard",
            r#"{"lines":{"keyed":{},"listed":[],"maybe":300}}"#,
        ),
        refused_row::<Wharf>(
            "a negative number for a u16 in a variant's one slot",
            "Wharf",
            r#"{"cleat":{"Bare":-1}}"#,
        ),
        refused_row::<Wharf>(
            "a number beyond a u16 in a variant of two",
            "Wharf",
            r#"{"cleat":{"Paired":[65536,"a"]}}"#,
        ),
        refused_row::<Quay>(
            "a number beyond an i8 in a tuple struct",
            "Quay",
            r#"{"fender":128}"#,
        ),
    ]
}

/// The place and the words serde refuses the payload of the report with, which the Rust
/// dispatcher answers.
#[test]
fn serde_refuses_a_negative_number_for_a_u32_behind_a_type_parameter_at_the_member() {
    assert_eq!(
        serde_answer::<Mooring>("held", r#"{"inner":{"held":-1}}"#),
        "held: refused at 'inner.held': invalid value: integer `-1`, expected u32\n"
    );
}

/// The text of every model compiles with warnings as errors, and the Swift read of every payload
/// answers what serde answers of it.
#[test]
fn a_type_parameter_is_refused_where_serde_refuses_it_and_read_where_it_reads_it_in_swift() {
    let mut rows = read_rows();
    rows.extend(refused_rows());
    let expected: String = rows.iter().map(|row| row.expected.as_str()).collect();
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
        tether_swift::swift_definition(),
        mooring_swift::swift_definition(),
        wide_mooring_swift::swift_definition(),
        knot_swift::swift_definition(),
        knot_mooring_swift::swift_definition(),
        hawser_swift::swift_definition(),
        bollard_swift::swift_definition(),
        cleat_swift::swift_definition(),
        wharf_swift::swift_definition(),
        fender_swift::swift_definition(),
        quay_swift::swift_definition(),
        HARNESS.to_owned(),
        calls.join("\n"),
    ]
    .join("\n\n");
    let Some(run) = judged(
        "swift-fitted-parameters",
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
