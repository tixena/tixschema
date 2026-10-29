//! A map under a key Swift wraps held as a type argument, and an alias of such a map and of a date,
//! read and written by the emitted Swift types compiled with warnings as errors: decoded directly
//! and held by a field, in a type that declares a parameter and in one that declares none. Each
//! payload the serde read reads prints what it prints.

#![cfg(feature = "swift")]

use std::collections::HashMap;

use serde::Serialize;
use serde::de::DeserializeOwned;

use super::held_maps_models::{
    Currents, Haversack, Leadline, Plumbline, Pocketing, Rucksack, StowedHaversack,
    StowedPocketing, beaconing_swift, captioning_swift, currents_swift, haversack_swift,
    leadline_swift, plumbline_swift, pocketing_swift, rucksack_swift, sequencing_swift,
    soundings_swift, stowed_haversack_swift, stowed_pocketing_swift, tabulation_swift,
};
#[cfg(feature = "chrono")]
use super::held_maps_models::{at, moonrise_swift};
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

/// A map of `held` under the numbers `1` and `2`.
fn numbered<Held>(held: &Held) -> HashMap<u32, Held>
where
    Held: Clone,
{
    HashMap::from([(1, held.clone()), (2, held.clone())])
}

/// A map of `held` under both currents.
fn currents<Held>(held: &Held) -> HashMap<Currents, Held>
where
    Held: Clone,
{
    HashMap::from([
        (Currents::East, held.clone()),
        (Currents::West, held.clone()),
    ])
}

/// A map of `held` under both flags.
fn flagged<Held>(held: &Held) -> HashMap<bool, Held>
where
    Held: Clone,
{
    HashMap::from([(true, held.clone()), (false, held.clone())])
}

/// The text `a`.
fn text() -> String {
    "a".to_owned()
}

/// The payloads of each map held as a type argument.
fn argument_rows() -> Vec<Row> {
    vec![
        read_row(
            "a map as a type argument",
            "Haversack",
            &Haversack {
                by_current: Rucksack {
                    held: currents(&text()),
                },
                by_flag: Rucksack {
                    held: flagged(&text()),
                },
                by_name: Rucksack {
                    held: HashMap::from([("n".to_owned(), text())]),
                },
                by_number: Rucksack {
                    held: numbered(&text()),
                },
                listed: Rucksack {
                    held: vec![numbered(&text()), HashMap::new()],
                },
                maybe: Rucksack {
                    held: Some(numbered(&text())),
                },
            },
        ),
        read_row(
            "a map as a type argument, none",
            "Haversack",
            &Haversack {
                by_current: Rucksack {
                    held: HashMap::new(),
                },
                by_flag: Rucksack {
                    held: HashMap::new(),
                },
                by_name: Rucksack {
                    held: HashMap::new(),
                },
                by_number: Rucksack {
                    held: HashMap::new(),
                },
                listed: Rucksack { held: Vec::new() },
                maybe: Rucksack { held: None },
            },
        ),
        read_row(
            "a map as a type argument that declares a parameter",
            "StowedHaversack<String>",
            &StowedHaversack {
                by_number: Rucksack {
                    held: numbered(&text()),
                },
                held: text(),
            },
        ),
    ]
}

/// The payload of a map of models a rule judges, held as a type argument and named by an alias.
fn judged_rows() -> Vec<Row> {
    let kept = Leadline { label: text() };
    vec![read_row(
        "a map of models held",
        "Plumbline",
        &Plumbline {
            argument: Rucksack {
                held: numbered(&kept),
            },
            sounded: numbered(&kept),
        },
    )]
}

/// The payloads of each alias, decoded directly and held by a field.
fn alias_rows() -> Vec<Row> {
    let mut rows = vec![
        read_row(
            "an alias of a map keyed by a number",
            "Tabulation<String>",
            &numbered(&text()),
        ),
        read_row(
            "an alias of a map keyed by a plain enum",
            "Sequencing",
            &currents(&text()),
        ),
        read_row(
            "an alias of a map keyed by a flag",
            "Beaconing",
            &flagged(&text()),
        ),
        read_row(
            "an alias of a map keyed by a string",
            "Captioning",
            &HashMap::from([("n".to_owned(), text())]),
        ),
        read_row(
            "fields typed with an alias",
            "Pocketing",
            &Pocketing {
                beaconing: flagged(&text()),
                captioning: HashMap::from([("n".to_owned(), text())]),
                #[cfg(feature = "chrono")]
                moonrise: at(),
                sequencing: currents(&text()),
                tabulation: numbered(&text()),
            },
        ),
        read_row(
            "fields typed with an alias that declares a parameter",
            "StowedPocketing<String>",
            &StowedPocketing {
                tabulation: numbered(&text()),
            },
        ),
    ];
    #[cfg(feature = "chrono")]
    rows.push(read_row("an alias of a date", "Moonrise", &at()));
    rows
}

/// The text of every model compiles with warnings as errors, and every payload reads and writes in
/// Swift as the serde read reads and writes it.
#[test]
fn a_map_held_as_a_type_argument_or_named_by_an_alias_reads_and_writes_in_swift_as_serde_does() {
    let mut rows = argument_rows();
    rows.extend(alias_rows());
    rows.extend(judged_rows());
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
    let mut parts = vec![
        "import Foundation".to_owned(),
        currents_swift::swift_definition(),
        rucksack_swift::swift_definition(),
        leadline_swift::swift_definition(),
        soundings_swift::swift_definition(),
        plumbline_swift::swift_definition(),
        haversack_swift::swift_definition(),
        stowed_haversack_swift::swift_definition(),
        tabulation_swift::swift_definition(),
        sequencing_swift::swift_definition(),
        beaconing_swift::swift_definition(),
        captioning_swift::swift_definition(),
        pocketing_swift::swift_definition(),
        stowed_pocketing_swift::swift_definition(),
    ];
    #[cfg(feature = "chrono")]
    parts.push(moonrise_swift::swift_definition());
    parts.extend([HARNESS.to_owned(), calls.join("\n")]);
    let source = parts.join("\n\n");
    let Some(run) = judged(
        "swift-held-maps",
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
