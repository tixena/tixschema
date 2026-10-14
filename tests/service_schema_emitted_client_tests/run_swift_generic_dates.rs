//! A date held by a type that declares a parameter, read and written by the emitted Swift types
//! compiled with warnings as errors: each payload the serde read reads prints what it prints. A
//! brand over a date and a map keyed by a date are left out: their Swift text does not compile for
//! reasons of its own, whether or not the type declares a parameter.

#![cfg(feature = "swift")]

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

/// A date under `as_number`, beside the parameter. Compiled, and not read: serde writes it as
/// epoch milliseconds, where the Swift read takes the text of a date.
#[model_schema(default_types(Stowed = String))]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Clocked<Stowed> {
    #[model_schema_prop(as_number)]
    pub at: DateTime<Utc>,
    pub held: Stowed,
}

/// A tuple struct of a date and the parameter. Compiled, and not read: Swift reads a date in a
/// tuple through `Date`'s own coding, a number, where serde writes text.
#[model_schema(default_types(Stowed = String))]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Leg<Stowed>(pub DateTime<Utc>, pub Stowed);

/// A date in a tuple and as the value of a map, beside the parameter. Compiled, and not read, as
/// [`Leg`].
#[model_schema(default_types(Stowed = String))]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Plot<Stowed> {
    pub by_name: HashMap<String, DateTime<Utc>>,
    pub by_number: HashMap<u32, DateTime<Utc>>,
    pub pair: (DateTime<Utc>, Stowed),
    pub paired_by_number: HashMap<u32, (DateTime<Utc>, Stowed)>,
}

/// A date in a variant with named members that name the parameter and in one whose members name
/// none, under no tag attribute. `Paired` and `Stamped` are compiled and not read, as [`Leg`].
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

/// The variants an untagged enum takes, named members and one slot. `Stamped` is compiled and not
/// read, as [`Leg`].
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

/// Every payload the run reads: a date in a struct and in a variant's struct, each where the type
/// declares a parameter and where it declares none.
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

/// The text of every model compiles with warnings as errors, and every payload reads and writes in
/// Swift as the serde read reads and writes it, each date through the text chrono writes.
#[test]
fn each_date_of_a_type_that_declares_a_parameter_reads_and_writes_in_swift_as_serde_does() {
    let rows = rows();
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

/// serde writes a date in a tuple, as a map's value and in a variant's slots as the text of the
/// date, and one under `as_number` as epoch milliseconds, which is what the members compiled and
/// not read are left out of the run for.
#[test]
fn a_date_left_out_of_the_run_is_written_as_serde_writes_it() {
    let first = at(1_704_164_645_678);
    for (written, expected) in [
        (
            serde_json::to_string(&Clocked {
                at: first,
                held: "a".to_owned(),
            }),
            r#"{"at":1704164645678,"held":"a"}"#,
        ),
        (
            serde_json::to_string(&Leg(first, "a".to_owned())),
            r#"["2024-01-02T03:04:05.678Z","a"]"#,
        ),
        (
            serde_json::to_string(&Plot {
                by_name: HashMap::from([("n".to_owned(), first)]),
                by_number: HashMap::from([(1, first)]),
                pair: (first, "a".to_owned()),
                paired_by_number: HashMap::from([(2, (first, "b".to_owned()))]),
            }),
            r#"{"by_name":{"n":"2024-01-02T03:04:05.678Z"},"by_number":{"1":"2024-01-02T03:04:05.678Z"},"pair":["2024-01-02T03:04:05.678Z","a"],"paired_by_number":{"2":["2024-01-02T03:04:05.678Z","b"]}}"#,
        ),
        (
            serde_json::to_string(&Ebb::Paired(first, "a".to_owned())),
            r#"{"Paired":["2024-01-02T03:04:05.678Z","a"]}"#,
        ),
        (
            serde_json::to_string(&Ebb::<String>::Stamped(first)),
            r#"{"Stamped":"2024-01-02T03:04:05.678Z"}"#,
        ),
        (
            serde_json::to_string(&AdjacentEbb::Paired(first, "a".to_owned())),
            r#"{"kind":"Paired","held":["2024-01-02T03:04:05.678Z","a"]}"#,
        ),
        (
            serde_json::to_string(&AdjacentEbb::<String>::Stamped(first)),
            r#"{"kind":"Stamped","held":"2024-01-02T03:04:05.678Z"}"#,
        ),
        (
            serde_json::to_string(&UntaggedEbb::<String>::Stamped(first)),
            r#""2024-01-02T03:04:05.678Z""#,
        ),
    ] {
        assert_eq!(written.unwrap(), expected);
    }
}
