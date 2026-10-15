//! A tuple and a map under a key Swift reads through a wrapper, held by a type that declares a
//! parameter, read and written by the emitted Swift types compiled with warnings as errors: each
//! payload the serde read reads prints what it prints, a map in a tuple's slot and in a variant's
//! slot included, and a model such a map holds is judged by `validate()` as the Rust `validate()`
//! judges it.

#![cfg(feature = "swift")]

use core::fmt::Write as _;
use std::collections::HashMap;

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

/// A plain enum a map is keyed by.
#[model_schema()]
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub enum Hatch {
    Aft,
    Fore,
}

/// A model with a rule, filled with a parameter.
#[model_schema(default_types(Contents = String))]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Barrel<Contents> {
    pub contents: Contents,
    #[model_schema_prop(minLength = 1)]
    pub label: String,
}

/// The parameter in a tuple, as the value of a map under a key Swift wraps, in a model such a map
/// holds, in a map a tuple holds and in a tuple a model is filled with; beside a tuple and a map
/// that name none.
#[model_schema(default_types(Stowed = String))]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Trove<Stowed> {
    pub barrel: Barrel<(Stowed, u8)>,
    pub by_barrel: HashMap<u32, Barrel<Stowed>>,
    pub by_count: HashMap<u32, u8>,
    pub by_number: HashMap<u32, Stowed>,
    pub marks: (u32, u8),
    pub pair: (Stowed, u32),
    pub paired_by_number: HashMap<u32, (Stowed, u8)>,
}

/// A map under a key Swift wraps, held in a tuple's slot.
#[model_schema(default_types(Stowed = String))]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Nook<Stowed> {
    pub numbered_pair: (HashMap<u32, Stowed>, u8),
}

/// A tuple struct whose slots are a tuple of the parameter and a number.
#[model_schema(default_types(Stowed = String))]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Brace<Stowed>(pub (Stowed, u8), pub u32);

/// A tuple struct whose slot is a map under a key Swift wraps.
#[model_schema(default_types(Stowed = String))]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Bunk<Stowed>(pub HashMap<u32, Stowed>, pub u8);

/// Each variant that holds a tuple or a map of the parameter, under no tag attribute. `Keyed` and
/// `Numbered` hold a map under a key Swift wraps in a slot.
#[model_schema(default_types(Stowed = String))]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum Parted<Stowed> {
    Halves(Stowed, u8),
    Keyed(HashMap<u32, Stowed>, u8),
    Numbered(HashMap<u32, Stowed>),
    Paired((Stowed, u32)),
    Stacked {
        by_number: HashMap<u32, Stowed>,
        pair: (Stowed, u32),
    },
}

/// The variant a tag alone takes, one with named members. serde reads it from what it buffers
/// beside the tag, where it refuses a number key written as text, so the map is keyed by a plain
/// enum.
#[model_schema(default_types(Stowed = String))]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "kind")]
pub enum TaggedParted<Stowed> {
    Stacked {
        by_hatch: HashMap<Hatch, Stowed>,
        pair: (Stowed, u32),
    },
}

/// [`Parted`] under a tag and a content key.
#[model_schema(default_types(Stowed = String))]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "kind", content = "held")]
pub enum AdjacentParted<Stowed> {
    Halves(Stowed, u8),
    Keyed(HashMap<u32, Stowed>, u8),
    Numbered(HashMap<u32, Stowed>),
    Paired((Stowed, u32)),
    Stacked {
        by_number: HashMap<u32, Stowed>,
        pair: (Stowed, u32),
    },
}

/// The variants an untagged enum takes, one slot and named members, each map keyed by a plain enum
/// as in [`TaggedParted`].
#[model_schema(default_types(Stowed = String))]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum UntaggedParted<Stowed> {
    Hatched(HashMap<Hatch, Stowed>),
    Paired((Stowed, u32)),
    Stacked {
        by_hatch: HashMap<Hatch, Stowed>,
        pair: (Stowed, u32),
    },
}

/// One payload the run reads: its name, the Swift type that reads it, the payload, and what the
/// Swift read is held to.
struct Row {
    expected: String,
    name: &'static str,
    payload: String,
    read_as: &'static str,
}

/// A barrel labelled `label` holding `contents`.
fn barrel<Contents>(label: &str, contents: Contents) -> Barrel<Contents> {
    Barrel {
        contents,
        label: label.to_owned(),
    }
}

/// A trove whose barrel in the map is labelled `label`.
fn trove(label: &str) -> Trove<String> {
    Trove {
        barrel: barrel("b", ("c".to_owned(), 1)),
        by_barrel: HashMap::from([(2, barrel(label, "d".to_owned()))]),
        by_count: HashMap::from([(3, 4)]),
        by_number: HashMap::from([(5, "e".to_owned())]),
        marks: (6, 7),
        pair: ("f".to_owned(), 8),
        paired_by_number: HashMap::from([(9, ("g".to_owned(), 10))]),
    }
}

/// The payload serde writes for `held`, which the serde read reads, held to what that read prints:
/// the value written back.
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

/// The payload serde writes for `held`, which breaks a rule, held to what the Rust `validate()`
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

/// The payloads of a struct and of a tuple struct that hold a tuple or a map of the parameter.
fn struct_rows() -> Vec<Row> {
    let unlabelled = trove("");
    let by_number = HashMap::from([(1, "a".to_owned())]);
    vec![
        read_row("the fields of a struct", "Trove<String>", &trove("h")),
        refused_row(
            "a model a map holds, unlabelled",
            "Trove<String>",
            &unlabelled,
            unlabelled.validate(),
        ),
        read_row(
            "a map in a tuple's slot",
            "Nook<String>",
            &Nook {
                numbered_pair: (by_number.clone(), 2),
            },
        ),
        read_row(
            "the tuple of a tuple struct",
            "Brace<String>",
            &Brace(("a".to_owned(), 1), 2),
        ),
        read_row(
            "a map in a tuple struct's slot",
            "Bunk<String>",
            &Bunk(by_number, 2),
        ),
    ]
}

/// The payloads of each variant that holds a tuple or a map of the parameter, under each tagging.
fn variant_rows() -> Vec<Row> {
    let by_number = HashMap::from([(1, "a".to_owned())]);
    let by_hatch = HashMap::from([(Hatch::Fore, "a".to_owned())]);
    let pair = ("b".to_owned(), 2);
    vec![
        read_row(
            "several slots, no tag attribute",
            "Parted<String>",
            &Parted::Halves("a".to_owned(), 1),
        ),
        read_row(
            "several slots that hold a map, no tag attribute",
            "Parted<String>",
            &Parted::Keyed(by_number.clone(), 2),
        ),
        read_row(
            "one slot that holds a map, no tag attribute",
            "Parted<String>",
            &Parted::Numbered(by_number.clone()),
        ),
        read_row(
            "one slot, no tag attribute",
            "Parted<String>",
            &Parted::Paired(pair.clone()),
        ),
        read_row(
            "named members, no tag attribute",
            "Parted<String>",
            &Parted::Stacked {
                by_number: by_number.clone(),
                pair: pair.clone(),
            },
        ),
        read_row(
            "named members, a tag alone",
            "TaggedParted<String>",
            &TaggedParted::Stacked {
                by_hatch: by_hatch.clone(),
                pair: pair.clone(),
            },
        ),
        read_row(
            "several slots, a tag and a content key",
            "AdjacentParted<String>",
            &AdjacentParted::Halves("a".to_owned(), 1),
        ),
        read_row(
            "several slots that hold a map, a tag and a content key",
            "AdjacentParted<String>",
            &AdjacentParted::Keyed(by_number.clone(), 2),
        ),
        read_row(
            "one slot that holds a map, a tag and a content key",
            "AdjacentParted<String>",
            &AdjacentParted::Numbered(by_number.clone()),
        ),
        read_row(
            "one slot, a tag and a content key",
            "AdjacentParted<String>",
            &AdjacentParted::Paired(pair.clone()),
        ),
        read_row(
            "named members, a tag and a content key",
            "AdjacentParted<String>",
            &AdjacentParted::Stacked {
                by_number,
                pair: pair.clone(),
            },
        ),
        read_row(
            "one slot that holds a map, untagged",
            "UntaggedParted<String>",
            &UntaggedParted::Hatched(by_hatch.clone()),
        ),
        read_row(
            "one slot, untagged",
            "UntaggedParted<String>",
            &UntaggedParted::Paired(pair.clone()),
        ),
        read_row(
            "named members, untagged",
            "UntaggedParted<String>",
            &UntaggedParted::Stacked { by_hatch, pair },
        ),
    ]
}

/// The text of every model compiles with warnings as errors, and every payload reads and writes in
/// Swift as the serde read reads and writes it, each tuple and each map held through the struct
/// written for it at the parameters it reaches. A model a map holds that breaks its rule is
/// refused under its place, as the Rust `validate()` reports it.
#[test]
fn each_helper_of_a_type_that_declares_a_parameter_reads_and_writes_in_swift_as_serde_does() {
    let mut rows = struct_rows();
    rows.extend(variant_rows());
    let expected: String = rows.iter().map(|row| row.expected.as_str()).collect();
    assert!(
        expected.contains(
            "a model a map holds, unlabelled: refused\n    'by_barrel.2.label': too short"
        ) && expected.contains(r#"a map in a tuple's slot: read {"numbered_pair":[{"1":"a"},2]}"#),
        "what the serde read writes and the Rust `validate()` answers: {expected}"
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
        hatch_swift::swift_definition(),
        barrel_swift::swift_definition(),
        trove_swift::swift_definition(),
        nook_swift::swift_definition(),
        brace_swift::swift_definition(),
        bunk_swift::swift_definition(),
        parted_swift::swift_definition(),
        tagged_parted_swift::swift_definition(),
        adjacent_parted_swift::swift_definition(),
        untagged_parted_swift::swift_definition(),
        HARNESS.to_owned(),
        calls.join("\n"),
    ]
    .join("\n\n");
    let Some(run) = judged(
        "swift-generic-helpers",
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
