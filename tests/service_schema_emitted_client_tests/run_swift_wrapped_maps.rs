//! A map under a key Swift reads through a wrapper, a number, a plain enum and a `bool`, in a
//! tuple's slot, in a tuple struct's slot, as another map's value, in a variant's one slot and in
//! the tuple of a variant of several slots, read and written by the emitted Swift types compiled
//! with warnings as errors, held by a type that declares a parameter and by one that declares
//! none. Each payload the serde read reads prints what it prints, and a model such a map holds is
//! judged by `validate()` as the Rust `validate()` judges it.

#![cfg(feature = "swift")]

use core::fmt::Write as _;
use core::hash::Hash;
use std::collections::HashMap;

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use tixschema::model_schema;

use super::run_swift_read::normalized;
use super::runtime::judged;
use super::wrapped_maps_models::{
    AdjacentDrawer, AdjacentStowedDrawer, Cabinet, Cubby, Drawer, Latch, Satchel, StowedCabinet,
    StowedCubby, StowedDrawer, TaggedDrawer, TaggedStowedDrawer, UntaggedDrawer,
    UntaggedStowedDrawer, adjacent_drawer_swift, adjacent_stowed_drawer_swift, cabinet_swift,
    cubby_swift, drawer_swift, latch_swift, satchel_swift, stowed_cabinet_swift,
    stowed_cubby_swift, stowed_drawer_swift, tagged_drawer_swift, tagged_stowed_drawer_swift,
    untagged_drawer_swift, untagged_stowed_drawer_swift,
};

/// The payload serde writes for the value `$held`, which breaks a rule, held to what the Rust
/// `validate()` answers of it.
macro_rules! refused {
    ($name:expr, $read_as:literal, $held:expr) => {{
        let held = $held;
        refused_row($name, $read_as, &held, held.validate())
    }};
}

/// The payloads of each variant of `$held`, an enum under no tag attribute or under a tag and a
/// content key, read as `$read_as`, each named for `$tagging`.
macro_rules! drawer_rows {
    ($held:ty, $read_as:literal, $tagging:literal) => {{
        type Held = $held;
        let text = "a".to_owned();
        vec![
            read_row(
                concat!("one slot that holds models, ", $tagging),
                $read_as,
                &Held::Bagged(one(1, "l")),
            ),
            read_row(
                concat!("one slot under a bool, ", $tagging),
                $read_as,
                &Held::Flagged(flagged(&text)),
            ),
            read_row(
                concat!("several slots, ", $tagging),
                $read_as,
                &Held::Keyed(numbered(&text), flagged(&text), latched(&text)),
            ),
            read_row(
                concat!("one slot under a plain enum, ", $tagging),
                $read_as,
                &Held::Latched(latched(&text)),
            ),
            read_row(
                concat!("one slot under a number, ", $tagging),
                $read_as,
                &Held::Numbered(numbered(&text)),
            ),
            read_row(
                concat!("several slots that hold models, ", $tagging),
                $read_as,
                &Held::Packed(one(true, "l"), text.clone()),
            ),
            read_row(
                concat!("a tuple of named members, ", $tagging),
                $read_as,
                &Held::Stacked {
                    pair: (numbered(&text), flagged(&text)),
                },
            ),
            refused!(
                concat!("one slot that holds models, unlabelled, ", $tagging),
                $read_as,
                Held::Bagged(one(1, ""))
            ),
            refused!(
                concat!("several slots that hold models, unlabelled, ", $tagging),
                $read_as,
                Held::Packed(one(true, ""), text)
            ),
        ]
    }};
}

/// The payloads of each variant of `$held`, an untagged enum, read as `$read_as`, each named for
/// `$held`'s parameter.
macro_rules! untagged_drawer_rows {
    ($held:ty, $read_as:literal, $parameter:literal) => {{
        type Held = $held;
        let text = "a".to_owned();
        vec![
            read_row(
                concat!("one slot, untagged", $parameter),
                $read_as,
                &Held::Latched(latched(&text)),
            ),
            read_row(
                concat!("a tuple in one slot, untagged", $parameter),
                $read_as,
                &Held::Paired((latched(&text), 1)),
            ),
            read_row(
                concat!("one slot that holds models, untagged", $parameter),
                $read_as,
                &Held::Sealed(one(Latch::Inner, "l")),
            ),
            read_row(
                concat!("a tuple of named members, untagged", $parameter),
                $read_as,
                &Held::Stacked {
                    pair: (latched(&text), 2),
                },
            ),
            refused!(
                concat!(
                    "one slot that holds models, unlabelled, untagged",
                    $parameter
                ),
                $read_as,
                Held::Sealed(one(Latch::Inner, ""))
            ),
        ]
    }};
}

/// The payloads of the variant of `$held`, an enum under a tag alone, read as `$read_as`, each
/// named for `$held`'s parameter.
macro_rules! tagged_drawer_rows {
    ($held:ty, $read_as:literal, $parameter:literal) => {{
        type Held = $held;
        let text = "a".to_owned();
        vec![
            read_row(
                concat!("named members, a tag alone", $parameter),
                $read_as,
                &Held::Stacked {
                    latches_by_latch: latched(&latched(&text)),
                    pair: (latched(&text), 3),
                    sealed: (one(Latch::Outer, "l"), 4),
                },
            ),
            refused!(
                concat!("named members, unlabelled, a tag alone", $parameter),
                $read_as,
                Held::Stacked {
                    latches_by_latch: latched(&latched(&text)),
                    pair: (latched(&text), 3),
                    sealed: (one(Latch::Outer, ""), 4),
                }
            ),
        ]
    }};
}

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

/// A map under a number key in a tuple's slot and as the value of a map Swift keys natively.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Plainly {
    pub nested: HashMap<String, HashMap<u32, String>>,
    pub numbered_pair: (HashMap<u32, String>, u8),
}

/// A map under a number key in the tuple of a variant of several slots and in a variant's one slot.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum Split {
    Keyed(HashMap<u32, String>, u8),
    Numbered(HashMap<u32, String>),
}

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

/// A map of `held` under the keys `1` and `2`.
fn numbered<Held>(held: &Held) -> HashMap<u32, Held>
where
    Held: Clone,
{
    HashMap::from([(1, held.clone()), (2, held.clone())])
}

/// A map of `held` under both keys of a `bool`.
fn flagged<Held>(held: &Held) -> HashMap<bool, Held>
where
    Held: Clone,
{
    HashMap::from([(false, held.clone()), (true, held.clone())])
}

/// A map of `held` under both latches.
fn latched<Held>(held: &Held) -> HashMap<Latch, Held>
where
    Held: Clone,
{
    HashMap::from([(Latch::Inner, held.clone()), (Latch::Outer, held.clone())])
}

/// A satchel labelled `label` under `key`: one entry, so the Rust `validate()` reports the satchels
/// of two maps in the order Swift walks them.
fn one<Key>(key: Key, label: &str) -> HashMap<Key, Satchel>
where
    Key: Eq + Hash,
{
    HashMap::from([(
        key,
        Satchel {
            label: label.to_owned(),
        },
    )])
}

/// [`Cubby`] whose satchels are labelled `label`.
fn cubby(label: &str) -> Cubby {
    let text = "a".to_owned();
    Cubby {
        flags: (flagged(&text), 1),
        flags_by_number: numbered(&flagged(&text)),
        latches: (latched(&text), 2),
        latches_by_latch: latched(&latched(&text)),
        numbers: (numbered(&text), 3),
        numbers_by_name: HashMap::from([("n".to_owned(), numbered(&text))]),
        satchels: (one(1, label), 4),
        satchels_by_name: HashMap::from([("s".to_owned(), one(2, label))]),
    }
}

/// [`StowedCubby`] whose satchels are labelled `label`.
fn stowed_cubby(label: &str) -> StowedCubby<String> {
    let text = "b".to_owned();
    StowedCubby {
        flags: (flagged(&text), 1),
        flags_by_number: numbered(&flagged(&text)),
        latches: (latched(&text), 2),
        latches_by_latch: latched(&latched(&text)),
        numbers: (numbered(&text), "c".to_owned()),
        numbers_by_name: HashMap::from([("n".to_owned(), numbered(&text))]),
        satchels: (one(1, label), "d".to_owned()),
        satchels_by_name: HashMap::from([("s".to_owned(), one(2, label))]),
    }
}

/// The payloads of a struct and of a tuple struct, in a type that declares a parameter and in one
/// that declares none.
fn struct_rows() -> Vec<Row> {
    let text = "a".to_owned();
    vec![
        read_row(
            "the reproduction's struct",
            "Plainly",
            &Plainly {
                nested: HashMap::from([("a".to_owned(), HashMap::from([(1, "x".to_owned())]))]),
                numbered_pair: (HashMap::from([(5, "c".to_owned())]), 6),
            },
        ),
        read_row("each map in a slot and as a value", "Cubby", &cubby("l")),
        read_row(
            "each map empty",
            "Cubby",
            &Cubby {
                flags: (HashMap::new(), 1),
                flags_by_number: HashMap::from([(1, HashMap::new())]),
                latches: (HashMap::new(), 2),
                latches_by_latch: HashMap::new(),
                numbers: (HashMap::new(), 3),
                numbers_by_name: HashMap::from([("n".to_owned(), HashMap::new())]),
                satchels: (HashMap::new(), 4),
                satchels_by_name: HashMap::new(),
            },
        ),
        read_row(
            "each map in a type that declares a parameter",
            "StowedCubby<String>",
            &stowed_cubby("l"),
        ),
        read_row(
            "the slots of a tuple struct",
            "Cabinet",
            &Cabinet(numbered(&text), flagged(&text), latched(&text), one(1, "l")),
        ),
        read_row(
            "the slots of a tuple struct that declares a parameter",
            "StowedCabinet<String>",
            &StowedCabinet(numbered(&text), flagged(&text), latched(&text), one(1, "l")),
        ),
        refused!(
            "a model in a slot and as a value, unlabelled",
            "Cubby",
            cubby("")
        ),
        refused!(
            "a model in a slot and as a value, unlabelled, a parameter",
            "StowedCubby<String>",
            stowed_cubby("")
        ),
        refused!(
            "a model in a tuple struct's slot, unlabelled",
            "Cabinet",
            Cabinet(numbered(&text), flagged(&text), latched(&text), one(1, ""))
        ),
        refused!(
            "a model in a tuple struct's slot, unlabelled, a parameter",
            "StowedCabinet<String>",
            StowedCabinet(numbered(&text), flagged(&text), latched(&text), one(1, ""))
        ),
    ]
}

/// The payloads of the reproduction's enum and of each enum under each tagging.
fn variant_rows() -> Vec<Row> {
    let mut rows = vec![
        read_row(
            "the reproduction's several slots",
            "Split",
            &Split::Keyed(HashMap::from([(2, "b".to_owned())]), 7),
        ),
        read_row(
            "the reproduction's one slot",
            "Split",
            &Split::Numbered(HashMap::from([(2, "b".to_owned())])),
        ),
    ];
    rows.extend(drawer_rows!(Drawer, "Drawer", "no tag attribute"));
    rows.extend(drawer_rows!(
        StowedDrawer<String>,
        "StowedDrawer<String>",
        "no tag attribute, a parameter"
    ));
    rows.extend(drawer_rows!(
        AdjacentDrawer,
        "AdjacentDrawer",
        "a tag and a content key"
    ));
    rows.extend(drawer_rows!(
        AdjacentStowedDrawer<String>,
        "AdjacentStowedDrawer<String>",
        "a tag and a content key, a parameter"
    ));
    rows.extend(untagged_drawer_rows!(UntaggedDrawer, "UntaggedDrawer", ""));
    rows.extend(untagged_drawer_rows!(
        UntaggedStowedDrawer<String>,
        "UntaggedStowedDrawer<String>",
        ", a parameter"
    ));
    rows.extend(tagged_drawer_rows!(TaggedDrawer, "TaggedDrawer", ""));
    rows.extend(tagged_drawer_rows!(
        TaggedStowedDrawer<String>,
        "TaggedStowedDrawer<String>",
        ", a parameter"
    ));
    rows
}

/// The text of every model compiles with warnings as errors, and every payload reads and writes in
/// Swift as the serde read reads and writes it, each map as the object serde writes. A model such a
/// map holds that breaks its rule is refused under its place, as the Rust `validate()` reports it.
#[test]
fn each_map_under_a_key_swift_wraps_reads_and_writes_in_swift_as_serde_does_wherever_it_is_held() {
    let mut rows = struct_rows();
    rows.extend(variant_rows());
    let expected: String = rows.iter().map(|row| row.expected.as_str()).collect();
    assert!(
        expected.contains(r#""numbered_pair":[{"5":"c"},6]"#)
            && expected.contains("'satchels.0.1.label': too short"),
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
        plainly_swift::swift_definition(),
        split_swift::swift_definition(),
        latch_swift::swift_definition(),
        satchel_swift::swift_definition(),
        cubby_swift::swift_definition(),
        stowed_cubby_swift::swift_definition(),
        cabinet_swift::swift_definition(),
        stowed_cabinet_swift::swift_definition(),
        drawer_swift::swift_definition(),
        stowed_drawer_swift::swift_definition(),
        adjacent_drawer_swift::swift_definition(),
        adjacent_stowed_drawer_swift::swift_definition(),
        untagged_drawer_swift::swift_definition(),
        untagged_stowed_drawer_swift::swift_definition(),
        tagged_drawer_swift::swift_definition(),
        tagged_stowed_drawer_swift::swift_definition(),
        HARNESS.to_owned(),
        calls.join("\n"),
    ]
    .join("\n\n");
    let Some(run) = judged(
        "swift-wrapped-maps",
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
