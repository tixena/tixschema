//! A date held by a type that declares a parameter, read and written by the emitted Swift types
//! compiled with warnings as errors: each payload the serde read reads prints what it prints, a
//! date in a tuple, as a map's value and in a variant's slots among them. A map keyed by a date is
//! read by `run_swift_date_map_keys.rs`, and the models of `swift_tests/held_dates_models.rs` by
//! `run_swift_held_dates.rs`.

#![cfg(feature = "swift")]

use core::marker::PhantomData;
use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tixschema::model_schema;

use super::run_swift_read::normalized;
use super::runtime::judged;

/// Names the runtime to run, for a machine that has one somewhere other than `PATH`.
const RUNTIME_VAR: &str = "TIXSCHEMA_SWIFT";

/// What a run reads a payload with, and prints the answer with: the value written back, or the
/// error the read threw. `unwrapped` is the function [`Sounding`] names.
const HARNESS: &str = r#"
func unwrapped(_ payload: Any) -> Any {
  if let entries = payload as? [String: Any], let held = entries["payload"] { return held }
  return payload
}

func row<Read: Codable>(_ name: String, _ type: Read.Type, _ payload: String) {
  do {
    let held = try JSONDecoder().decode(type, from: Data(payload.utf8))
    let encoder = JSONEncoder()
    encoder.outputFormatting = [.sortedKeys]
    print("\(name): read \(String(data: try encoder.encode(held), encoding: .utf8)!)")
  } catch {
    print("\(name): \(error)")
  }
}
"#;

/// A date in a type that declares no parameter.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Plotted {
    pub at: DateTime<Utc>,
}

/// A date, a list of dates and an optional date, beside the parameter.
#[model_schema(default_types(Stowed = String))]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Reckoning<Stowed> {
    pub at: DateTime<Utc>,
    pub held: Stowed,
    pub marks: Vec<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seen: Option<DateTime<Utc>>,
}

/// A date that names a function, beside the parameter.
#[model_schema(default_types(Stowed = String))]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Sounding<Stowed> {
    #[model_schema_prop(preprocess = ["unwrapped"])]
    pub at: DateTime<Utc>,
    pub held: Stowed,
}

/// A brand over a date that declares a parameter.
#[model_schema(default_types(Stowed = String))]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(transparent)]
pub struct Landfall<Stowed> {
    pub at: DateTime<Utc>,
    pub marker: PhantomData<Stowed>,
}

/// A date under `as_number`, beside the parameter. Compiled, and not read: serde writes it as
/// epoch milliseconds, where the Swift read takes the text of a date.
#[model_schema(default_types(Stowed = String))]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Clocked<Stowed> {
    #[model_schema_prop(as_number)]
    pub at: DateTime<Utc>,
    pub held: Stowed,
}

/// A tuple struct of a date and the parameter.
#[model_schema(default_types(Stowed = String))]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Leg<Stowed>(pub DateTime<Utc>, pub Stowed);

/// A date in a tuple and as the value of a map, beside the parameter.
#[model_schema(default_types(Stowed = String))]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Plot<Stowed> {
    pub by_name: HashMap<String, DateTime<Utc>>,
    pub by_number: HashMap<u32, DateTime<Utc>>,
    pub pair: (DateTime<Utc>, Stowed),
    pub paired_by_number: HashMap<u32, (DateTime<Utc>, Stowed)>,
}

/// A date in a variant with named members that name the parameter, in one whose members name none,
/// in a variant of several slots and in a variant of one, under no tag attribute.
#[model_schema(default_types(Stowed = String))]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum Ebb<Stowed> {
    Logged { at: DateTime<Utc>, held: Stowed },
    Noted { at: DateTime<Utc> },
    Paired(DateTime<Utc>, Stowed),
    Stamped(DateTime<Utc>),
}

/// The variants a tag alone takes, those with named members.
#[model_schema(default_types(Stowed = String))]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "kind")]
pub enum TaggedEbb<Stowed> {
    Logged { at: DateTime<Utc>, held: Stowed },
    Noted { at: DateTime<Utc> },
}

/// [`Ebb`] under a tag and a content key.
#[model_schema(default_types(Stowed = String))]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "kind", content = "held")]
pub enum AdjacentEbb<Stowed> {
    Logged { at: DateTime<Utc>, held: Stowed },
    Noted { at: DateTime<Utc> },
    Paired(DateTime<Utc>, Stowed),
    Stamped(DateTime<Utc>),
}

/// The variants an untagged enum takes, named members and one slot.
#[model_schema(default_types(Stowed = String))]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum UntaggedEbb<Stowed> {
    Logged { at: DateTime<Utc>, held: Stowed },
    Noted { at: DateTime<Utc> },
    Stamped(DateTime<Utc>),
}

/// One payload the run reads: its name, the Swift type that reads it, the payload, and what the
/// Swift read is held to.
struct Row {
    expected: String,
    name: &'static str,
    payload: String,
    read_as: &'static str,
}

/// The payload of `held` with `payload` in place of its own: the function [`Sounding`] names
/// answers what it holds.
fn unwrapped(payload: Value) -> Value {
    if let Value::Object(mut entries) = payload {
        entries
            .remove("payload")
            .unwrap_or_else(|| Value::Object(entries))
    } else {
        payload
    }
}

/// The date `millis` milliseconds after the epoch.
const fn at(millis: i64) -> DateTime<Utc> {
    DateTime::from_timestamp_millis(millis).unwrap()
}

/// `payload`, which the serde read reads as a `Held`, held to what that read prints: the value
/// written back.
fn payload_row<Held>(name: &'static str, read_as: &'static str, payload: String) -> Row
where
    Held: DeserializeOwned + Serialize,
{
    let read: Held = serde_json::from_str(&payload).unwrap();
    Row {
        expected: format!("{name}: read {}\n", serde_json::to_string(&read).unwrap()),
        name,
        payload,
        read_as,
    }
}

/// The payload serde writes for `held`, held to what the serde read prints of it.
fn read_row<Held>(name: &'static str, read_as: &'static str, held: &Held) -> Row
where
    Held: DeserializeOwned + Serialize,
{
    payload_row::<Held>(name, read_as, serde_json::to_string(held).unwrap())
}

/// The payloads of a date in a struct and in a variant's struct, each where the type declares a
/// parameter and where it declares none, and in a brand that declares one.
fn rows() -> Vec<Row> {
    let first = at(1_704_164_645_678);
    let second = at(1_709_208_000_100);
    let held = || "a".to_owned();
    vec![
        read_row(
            "a struct that declares no parameter",
            "Plotted",
            &Plotted { at: first },
        ),
        read_row(
            "every date of a struct",
            "Reckoning<String>",
            &Reckoning {
                at: first,
                held: held(),
                marks: vec![first, second],
                seen: Some(second),
            },
        ),
        read_row(
            "a struct with no optional date",
            "Reckoning<String>",
            &Reckoning {
                at: first,
                held: held(),
                marks: Vec::new(),
                seen: None,
            },
        ),
        read_row(
            "a brand that declares a parameter",
            "Landfall<String>",
            &Landfall::<String> {
                at: first,
                marker: PhantomData,
            },
        ),
        payload_row::<Sounding<String>>(
            "a date a function answers",
            "Sounding<String>",
            r#"{"at":{"payload":"2024-01-02T03:04:05.678Z"},"held":"a"}"#.to_owned(),
        ),
        read_row(
            "named members that name the parameter, no tag attribute",
            "Ebb<String>",
            &Ebb::Logged {
                at: first,
                held: held(),
            },
        ),
        read_row(
            "named members that name none, no tag attribute",
            "Ebb<String>",
            &Ebb::<String>::Noted { at: second },
        ),
        read_row(
            "named members that name the parameter, a tag alone",
            "TaggedEbb<String>",
            &TaggedEbb::Logged {
                at: first,
                held: held(),
            },
        ),
        read_row(
            "named members that name none, a tag alone",
            "TaggedEbb<String>",
            &TaggedEbb::<String>::Noted { at: second },
        ),
        read_row(
            "named members that name the parameter, a tag and a content key",
            "AdjacentEbb<String>",
            &AdjacentEbb::Logged {
                at: first,
                held: held(),
            },
        ),
        read_row(
            "named members that name none, a tag and a content key",
            "AdjacentEbb<String>",
            &AdjacentEbb::<String>::Noted { at: second },
        ),
        read_row(
            "named members that name the parameter, untagged",
            "UntaggedEbb<String>",
            &UntaggedEbb::Logged {
                at: first,
                held: held(),
            },
        ),
        read_row(
            "named members that name none, untagged",
            "UntaggedEbb<String>",
            &UntaggedEbb::<String>::Noted { at: second },
        ),
    ]
}

/// The payloads of a date in a tuple, as a map's value and in a variant's slots, in a type that
/// declares a parameter.
fn held_rows() -> Vec<Row> {
    let first = at(1_704_164_645_678);
    let second = at(1_709_208_000_100);
    let held = || "a".to_owned();
    vec![
        read_row(
            "a tuple struct that declares a parameter",
            "Leg<String>",
            &Leg(first, held()),
        ),
        read_row(
            "a date in a tuple and as a map's value",
            "Plot<String>",
            &Plot {
                by_name: HashMap::from([("n".to_owned(), first), ("m".to_owned(), second)]),
                by_number: HashMap::from([(1, first), (2, second)]),
                pair: (first, held()),
                paired_by_number: HashMap::from([(2, (second, "b".to_owned()))]),
            },
        ),
        read_row(
            "several slots, no tag attribute",
            "Ebb<String>",
            &Ebb::Paired(first, held()),
        ),
        read_row(
            "one slot, no tag attribute",
            "Ebb<String>",
            &Ebb::<String>::Stamped(second),
        ),
        read_row(
            "several slots, a tag and a content key",
            "AdjacentEbb<String>",
            &AdjacentEbb::Paired(first, held()),
        ),
        read_row(
            "one slot, a tag and a content key",
            "AdjacentEbb<String>",
            &AdjacentEbb::<String>::Stamped(second),
        ),
        read_row(
            "one slot, untagged",
            "UntaggedEbb<String>",
            &UntaggedEbb::<String>::Stamped(second),
        ),
    ]
}

/// The text of every model compiles with warnings as errors, and every payload reads and writes in
/// Swift as the serde read reads and writes it, each date through the text chrono writes.
#[test]
fn each_date_of_a_type_that_declares_a_parameter_reads_and_writes_in_swift_as_serde_does() {
    let mut rows = rows();
    rows.extend(held_rows());
    let expected: String = rows.iter().map(|row| row.expected.as_str()).collect();
    assert!(
        expected.contains(r#""at":"2024-01-02T03:04:05.678Z","held":"a","marks":["#),
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
        plotted_swift::swift_definition(),
        reckoning_swift::swift_definition(),
        sounding_swift::swift_definition(),
        landfall_swift::swift_definition(),
        clocked_swift::swift_definition(),
        leg_swift::swift_definition(),
        plot_swift::swift_definition(),
        ebb_swift::swift_definition(),
        tagged_ebb_swift::swift_definition(),
        adjacent_ebb_swift::swift_definition(),
        untagged_ebb_swift::swift_definition(),
        HARNESS.to_owned(),
        calls.join("\n"),
    ]
    .join("\n\n");
    let Some(run) = judged(
        "swift-generic-dates",
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

/// serde writes a date under `as_number` as epoch milliseconds, which is what the member compiled
/// and not read is left out of the run for.
#[test]
fn a_date_left_out_of_the_run_is_written_as_serde_writes_it() {
    let written = serde_json::to_string(&Clocked {
        at: at(1_704_164_645_678),
        held: "a".to_owned(),
    });
    assert_eq!(written.unwrap(), r#"{"at":1704164645678,"held":"a"}"#);
}
