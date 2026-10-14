//! A std sequence other than `Vec` read and written by the emitted Swift types, compiled with
//! warnings as errors: a payload the serde read reads prints what it prints, and the items of each
//! sequence are judged by `validate()` as the Rust `validate()` judges them.

#![cfg(feature = "swift")]

use alloc::collections::{BTreeSet, BinaryHeap, VecDeque};
use core::fmt::Write as _;
use std::collections::HashSet;

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use tixschema::model_schema;

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

/// Each payload of [`Flotilla`] the serde read reads: its name and the payload.
const FLOTILLA: [(&str, &str); 3] = [
    (
        "every sequence held",
        r#"{"binary_heap":[{"mark":"a"}],"btree_set":[{"mark":"b"},{"mark":"c"}],"hash_set":[{"mark":"d"}],"layered":[["e"],[]],"spare":[1,2],"vec_deque":[{"mark":"f"},{"mark":"g"}]}"#,
    ),
    (
        "no spare",
        r#"{"binary_heap":[],"btree_set":[],"hash_set":[],"layered":[],"vec_deque":[]}"#,
    ),
    (
        "a null spare",
        r#"{"binary_heap":[],"btree_set":[],"hash_set":[],"layered":[],"spare":null,"vec_deque":[]}"#,
    ),
];

/// Each payload of [`IndexedFlotilla`] the serde read reads: its name and the payload.
#[cfg(feature = "indexmap")]
const INDEXED_FLOTILLA: [(&str, &str); 1] = [(
    "an IndexSet held",
    r#"{"index_set":[{"mark":"a"},{"mark":"b"}]}"#,
)];

/// A model with a bound, held in each sequence.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
pub struct Beacon {
    #[model_schema_prop(minLength = 1)]
    pub mark: String,
}

/// Each std sequence but `Vec`, of a model, in a list and under `Option`.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Flotilla {
    pub binary_heap: BinaryHeap<Beacon>,
    pub btree_set: BTreeSet<Beacon>,
    pub hash_set: HashSet<Beacon>,
    pub layered: Vec<HashSet<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spare: Option<BTreeSet<u32>>,
    pub vec_deque: VecDeque<Beacon>,
}

/// The `indexmap` set, of a model.
#[cfg(feature = "indexmap")]
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct IndexedFlotilla {
    pub index_set: indexmap::IndexSet<Beacon>,
}

/// One payload the run reads: its name, the Swift type that reads it, the payload, and what the
/// Swift read is held to.
struct Row {
    expected: String,
    name: &'static str,
    payload: String,
    read_as: &'static str,
}

/// A beacon with `mark`.
fn beacon(mark: &str) -> Beacon {
    Beacon {
        mark: mark.to_owned(),
    }
}

/// What the serde read prints of `payload`: the value written back, or each message of the
/// refusal.
fn printed<Read>(name: &str, payload: &str) -> String
where
    Read: DeserializeOwned + Serialize,
{
    match serde_json::from_str::<Read>(payload) {
        Ok(read) => format!("{name}: read {}\n", serde_json::to_string(&read).unwrap()),
        Err(refused) => {
            let mut said = format!("{name}: refused\n");
            for message in refused.to_string().split("; ") {
                let _ = writeln!(said, "    {message}");
            }
            said
        }
    }
}

/// A payload the serde read reads, held to what it prints.
fn read_row<Read>(name: &'static str, read_as: &'static str, payload: &str) -> Row
where
    Read: DeserializeOwned + Serialize,
{
    Row {
        expected: printed::<Read>(name, payload),
        name,
        payload: payload.to_owned(),
        read_as,
    }
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

/// Every payload the run reads.
fn rows() -> Vec<Row> {
    let empty_marks = Flotilla {
        binary_heap: BinaryHeap::from([beacon("")]),
        btree_set: BTreeSet::from([beacon("")]),
        hash_set: HashSet::from([beacon("")]),
        layered: Vec::new(),
        spare: None,
        vec_deque: VecDeque::from([beacon("f"), beacon("")]),
    };
    let mut rows: Vec<Row> = FLOTILLA
        .iter()
        .map(|&(name, payload)| read_row::<Flotilla>(name, "Flotilla", payload))
        .collect();
    rows.push(refused_row(
        "an empty mark in each sequence",
        "Flotilla",
        &empty_marks,
        empty_marks.validate(),
    ));
    rows.extend(indexed_rows());
    rows
}

/// Every payload of [`IndexedFlotilla`] the run reads.
#[cfg(feature = "indexmap")]
fn indexed_rows() -> Vec<Row> {
    let empty_mark = IndexedFlotilla {
        index_set: indexmap::IndexSet::from([beacon("a"), beacon("")]),
    };
    let mut rows: Vec<Row> = INDEXED_FLOTILLA
        .iter()
        .map(|&(name, payload)| read_row::<IndexedFlotilla>(name, "IndexedFlotilla", payload))
        .collect();
    rows.push(refused_row(
        "an empty mark in an IndexSet",
        "IndexedFlotilla",
        &empty_mark,
        empty_mark.validate(),
    ));
    rows
}

/// No `indexmap` payload where the feature is off.
#[cfg(not(feature = "indexmap"))]
const fn indexed_rows() -> Vec<Row> {
    Vec::new()
}

/// Every payload reads in Swift as the serde read reads it, each sequence being the array serde
/// writes, and an item that breaks its model's bound is refused under its place, as the Rust
/// `validate()` reports it.
#[test]
fn each_sequence_reads_and_writes_in_swift_as_serde_does() {
    let rows = rows();
    let expected: String = rows.iter().map(|row| row.expected.as_str()).collect();
    assert!(
        expected.contains(
            "an empty mark in each sequence: refused\n    'binary_heap.0.mark': too short"
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
        beacon_swift::swift_definition(),
        flotilla_swift::swift_definition(),
        #[cfg(feature = "indexmap")]
        indexed_flotilla_swift::swift_definition(),
        HARNESS.to_owned(),
        calls.join("\n"),
    ]
    .join("\n\n");
    let Some(run) = judged(
        "swift-sequences",
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
