//! A brand whose value Swift reads as one type and converts, a map under a key Swift wraps or a
//! date, read and written by the emitted Swift types compiled with warnings as errors: each payload
//! the serde read reads prints what it prints, and a brand that breaks its bound is refused as the
//! Rust `validate()` reports it. A brand over a list of such maps is left out: its Swift text does
//! not compile for a reason of its own.

#![cfg(feature = "swift")]

use core::fmt::Write as _;
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

/// What a run reads a payload with, and prints the answer with: the value written back, or each
/// message of a refusal. `unwrapped` is the function [`Ashore`] names.
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
  } catch DecodingError.dataCorrupted(let context) {
    print("\(name): refused")
    for line in context.debugDescription.components(separatedBy: "; ") { print("    \(line)") }
  } catch {
    print("\(name): \(error)")
  }
}
"#;

/// A brand over a map under a key Swift wraps.
#[model_schema(no_display)]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(transparent)]
pub struct Hoarded(pub HashMap<u32, String>);

/// A brand with named members over a map under a key Swift wraps.
#[model_schema(no_display)]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(transparent)]
pub struct Coffer {
    pub by_number: HashMap<u32, String>,
}

/// A brand over a map of the parameter under a key Swift wraps.
#[model_schema(default_types(Stowed = String))]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(transparent)]
pub struct Stash<Stowed>(pub HashMap<u32, Stowed>);

/// A brand over a list of maps under a key Swift wraps. Written by serde, and left out of the run.
#[model_schema(no_display)]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(transparent)]
pub struct Hoards(pub Vec<HashMap<u32, String>>);

/// A brand over a date.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(transparent)]
pub struct PlainLandfall(pub DateTime<Utc>);

/// A brand over a date that names a function.
#[model_schema(preprocess = ["unwrapped"])]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(transparent)]
pub struct Ashore(pub DateTime<Utc>);

/// A brand over a list of dates, which counts them.
#[model_schema(no_display, minLength = 1)]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(transparent)]
pub struct Landings(pub Vec<DateTime<Utc>>);

/// A brand over a list of optional dates.
#[model_schema(no_display)]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(transparent)]
pub struct Sightings(pub Vec<Option<DateTime<Utc>>>);

/// A brand over a list of dates that declares a parameter, which counts them.
#[model_schema(no_display, maxLength = 2, default_types(Stowed = String))]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(transparent)]
pub struct Arrivals<Stowed> {
    pub marker: PhantomData<Stowed>,
    pub marks: Vec<DateTime<Utc>>,
}

/// Each brand held as an optional member: the macro refuses a brand over an `Option` in this build.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Lockers {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arrivals: Option<Arrivals<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hoarded: Option<Hoarded>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub landfall: Option<PlainLandfall>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stash: Option<Stash<String>>,
}

/// One payload the run reads: its name, the Swift type that reads it, the payload, and what the
/// Swift read is held to.
struct Row {
    expected: String,
    name: &'static str,
    payload: String,
    read_as: &'static str,
}

/// The payload with what it holds under `payload` in place of its own: the function [`Ashore`]
/// names.
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

/// A map of `held` under the key `1`.
fn at_one<Held>(held: Held) -> HashMap<u32, Held> {
    HashMap::from([(1, held)])
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

/// The payload serde writes for `held`, which breaks a bound, held to what the Rust `validate()`
/// answers of it.
fn refused_row<Held>(
    name: &'static str,
    read_as: &'static str,
    held: &Held,
    answered: Result<(), Vec<String>>,
) -> Row
where
    Held: DeserializeOwned + Serialize,
{
    let payload = serde_json::to_string(held).unwrap();
    assert!(serde_json::from_str::<Held>(&payload).is_err(), "{payload}");
    let mut expected = format!("{name}: refused\n");
    for message in answered.unwrap_err() {
        let _ = writeln!(expected, "    {message}");
    }
    Row {
        expected,
        name,
        payload,
        read_as,
    }
}

/// The dates `marks` held by [`Arrivals`].
const fn arrivals(marks: Vec<DateTime<Utc>>) -> Arrivals<String> {
    Arrivals {
        marker: PhantomData,
        marks,
    }
}

/// Every payload the run reads: a brand over each value Swift converts, alone and held as an
/// optional member, and each brand that counts its dates beside its bound.
fn rows() -> Vec<Row> {
    let first = at(1_704_164_645_678);
    let second = at(1_709_208_000_100);
    let held = || at_one("a".to_owned());
    let landings = Landings(Vec::new());
    let crowded = arrivals(vec![first, second, first]);
    vec![
        read_row("a map", "Hoarded", &Hoarded(held())),
        read_row("an empty map", "Hoarded", &Hoarded(HashMap::new())),
        read_row(
            "a map under a named member",
            "Coffer",
            &Coffer { by_number: held() },
        ),
        read_row("a map of the parameter", "Stash<String>", &Stash(held())),
        read_row("a date", "PlainLandfall", &PlainLandfall(first)),
        payload_row::<Ashore>(
            "a date a function answers",
            "Ashore",
            r#"{"payload":"2024-01-02T03:04:05.678Z"}"#.to_owned(),
        ),
        read_row(
            "a list of dates",
            "Landings",
            &Landings(vec![first, second]),
        ),
        refused_row(
            "a list of dates, empty",
            "Landings",
            &landings,
            landings.validate(),
        ),
        read_row(
            "a list of optional dates",
            "Sightings",
            &Sightings(vec![Some(first), None]),
        ),
        read_row(
            "a list of dates that declares a parameter",
            "Arrivals<String>",
            &arrivals(vec![first, second]),
        ),
        refused_row(
            "a list of dates that declares a parameter, crowded",
            "Arrivals<String>",
            &crowded,
            crowded.validate(),
        ),
        read_row(
            "each brand as an optional member",
            "Lockers",
            &Lockers {
                arrivals: Some(arrivals(vec![second])),
                hoarded: Some(Hoarded(held())),
                landfall: Some(PlainLandfall(first)),
                stash: Some(Stash(held())),
            },
        ),
        read_row(
            "no brand as an optional member",
            "Lockers",
            &Lockers {
                arrivals: None,
                hoarded: None,
                landfall: None,
                stash: None,
            },
        ),
    ]
}

/// The text of every model compiles with warnings as errors, and every payload reads and writes in
/// Swift as the serde read reads and writes it: a map through the struct written for its keys, a
/// date through the text chrono writes. A brand that breaks its bound is refused as the Rust
/// `validate()` reports it.
#[test]
fn each_brand_whose_value_swift_converts_reads_and_writes_in_swift_as_serde_does() {
    let rows = rows();
    let expected: String = rows.iter().map(|row| row.expected.as_str()).collect();
    assert!(
        expected.contains(
            "a list of dates, empty: refused\n    too short: minimum length is 1, got 0\n"
        ),
        "what the Rust `validate()` answers: {expected}"
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
        hoarded_swift::swift_definition(),
        coffer_swift::swift_definition(),
        stash_swift::swift_definition(),
        plain_landfall_swift::swift_definition(),
        ashore_swift::swift_definition(),
        landings_swift::swift_definition(),
        sightings_swift::swift_definition(),
        arrivals_swift::swift_definition(),
        lockers_swift::swift_definition(),
        HARNESS.to_owned(),
        calls.join("\n"),
    ]
    .join("\n\n");
    let Some(run) = judged(
        "swift-brand-reads",
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

/// serde writes a brand over a list of maps under a key Swift wraps as a list of objects, which is
/// what the run would read; it is left out of it.
#[test]
fn a_brand_left_out_of_the_run_is_written_as_serde_writes_it() {
    assert_eq!(
        serde_json::to_string(&Hoards(vec![at_one("a".to_owned())])).unwrap(),
        r#"[{"1":"a"}]"#
    );
}
