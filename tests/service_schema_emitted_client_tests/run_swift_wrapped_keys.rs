//! A map keyed by a brand over a string, a number, a flag and a date, by an alias of each and by a
//! plain enum, read and written by the emitted Swift types compiled with warnings as errors: as a
//! field, as a member of a variant under each tagging, as a brand's value and under a list, an
//! optional and a map, in a type that declares a parameter and in one that declares none. Each
//! payload the serde read reads prints what it prints, and a key that is no number is refused, as
//! the serde read refuses it.

#![cfg(feature = "swift")]

use core::fmt::Write as _;
use core::hash::Hash;
use std::collections::HashMap;

#[cfg(feature = "chrono")]
use chrono::{DateTime, Utc};
use serde::Serialize;
use serde::de::DeserializeOwned;

use super::run_swift_read::normalized;
use super::runtime::judged;
use super::wrapped_keys_models::{
    AdjacentConvoy, AdjacentSquadron, Armada, Berthing, Burgee, Callsign, Cleat, Convoy, Dockage,
    Hull, Lanyard, Logroll, Logroster, Signal, Squadron, StowedTaskforce, TaggedConvoy,
    TaggedSquadron, Taskforce, Unhashed, UntaggedConvoy, UntaggedSquadron, adjacent_convoy_swift,
    adjacent_squadron_swift, armada_swift, berthing_swift, burgee_swift, callsign_swift,
    cleat_swift, convoy_swift, dockage_swift, greeting_swift, hailer_swift, hull_swift,
    lanyard_swift, logroll_swift, logroster_swift, signal_swift, squadron_swift,
    stowed_taskforce_swift, tagged_convoy_swift, tagged_squadron_swift, taskforce_swift,
    unhashed_swift, untagged_convoy_swift, untagged_squadron_swift,
};
#[cfg(feature = "chrono")]
use super::wrapped_keys_models::{
    Postmark, Regatta, StowedRegatta, first, postmark_key_swift, postmark_swift, regatta_swift,
    stowed_regatta_swift,
};

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

/// What the Swift read prints for a payload whose key `1` is `x`.
const BAD_KEY: &str = "refused\n    bad key x";

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

/// The payload serde writes for `held` with its first key `written` replaced by `x`, which the
/// serde read refuses, held to [`BAD_KEY`].
fn refused_row<Held>(name: &'static str, read_as: &'static str, held: &Held, written: &str) -> Row
where
    Held: DeserializeOwned + Serialize,
{
    let text = serde_json::to_string(held).unwrap();
    let key = format!(r#""{written}":"#);
    assert!(text.contains(&key), "{text}");
    let payload = text.replacen(&key, r#""x":"#, 1);
    assert!(serde_json::from_str::<Held>(&payload).is_err(), "{payload}");
    Row {
        expected: format!("{name}: {BAD_KEY}\n"),
        name,
        payload,
        read_as,
    }
}

/// A map of `held` under the keys `first` and `second`.
fn under<Key, Held>(first: Key, second: Key, held: &Held) -> HashMap<Key, Held>
where
    Key: Hash + Eq,
    Held: Clone,
{
    HashMap::from([(first, held.clone()), (second, held.clone())])
}

/// A map of `held` under two callsigns.
fn callsigns<Held>(held: &Held) -> HashMap<Callsign, Held>
where
    Held: Clone,
{
    under(
        Callsign("alpha".to_owned()),
        Callsign("bravo".to_owned()),
        held,
    )
}

/// A map of `held` under the hulls `1` and `2`.
fn hulls<Held>(held: &Held) -> HashMap<Hull, Held>
where
    Held: Clone,
{
    under(1, 2, held)
}

/// [`Taskforce`] with every map holding entries.
fn taskforce() -> Taskforce {
    Taskforce {
        berthing: under(Berthing::Port, Berthing::Starboard, &1),
        callsign: callsigns(&1),
        hailer: under(
            Callsign("charlie".to_owned()),
            Callsign("delta".to_owned()),
            &2,
        ),
        greeting: under("echo".to_owned(), "foxtrot".to_owned(), &3),
        hull: hulls(&4),
        lanyard: under(Lanyard(Burgee(7)), Lanyard(Burgee(8)), &5),
        burgee: under(Burgee(5), Burgee(6), &6),
        signal: under(Signal(true), Signal(false), &7),
    }
}

/// [`Taskforce`] with every map empty.
fn empty_taskforce() -> Taskforce {
    Taskforce {
        berthing: HashMap::new(),
        callsign: HashMap::new(),
        hailer: HashMap::new(),
        greeting: HashMap::new(),
        hull: HashMap::new(),
        lanyard: HashMap::new(),
        burgee: HashMap::new(),
        signal: HashMap::new(),
    }
}

/// The payloads of each struct, brand and list or optional of maps.
fn struct_rows() -> Vec<Row> {
    let text = "t".to_owned();
    let mut rows = vec![
        read_row("each key as a field", "Taskforce", &taskforce()),
        read_row(
            "each key as a field, empty",
            "Taskforce",
            &empty_taskforce(),
        ),
        read_row(
            "each key as a field that declares a parameter",
            "StowedTaskforce<String>",
            &StowedTaskforce {
                callsign: callsigns(&text),
                hull: hulls(&text),
                burgee: under(Burgee(1), Burgee(2), &text),
                counts: under(Callsign("a".to_owned()), Callsign("b".to_owned()), &1),
            },
        ),
        read_row(
            "a list, an optional and a map of maps",
            "Armada",
            &Armada {
                listed: vec![callsigns(&1), HashMap::new()],
                maybe: Some(hulls(&2)),
                nested: HashMap::from([(1, callsigns(&3))]),
            },
        ),
        read_row(
            "a list, an optional and a map of maps, none",
            "Armada",
            &Armada {
                listed: Vec::new(),
                maybe: None,
                nested: HashMap::new(),
            },
        ),
        read_row(
            "a brand that derives no Hash",
            "Unhashed",
            &Unhashed("u".to_owned()),
        ),
        read_row("a brand over a map", "Logroll", &Logroll(callsigns(&1))),
        read_row(
            "a brand over a map that declares a parameter",
            "Logroster<String>",
            &Logroster(hulls(&text)),
        ),
        refused_row(
            "a number brand under a key that is no number",
            "Taskforce",
            &Taskforce {
                burgee: under(Burgee(1), Burgee(2), &6),
                ..empty_taskforce()
            },
            "1",
        ),
        refused_row(
            "an alias of a number under a key that is no number",
            "Taskforce",
            &Taskforce {
                hull: hulls(&4),
                ..empty_taskforce()
            },
            "1",
        ),
        refused_row(
            "a brand over a flag under a key that is no flag",
            "Taskforce",
            &Taskforce {
                signal: under(Signal(true), Signal(false), &7),
                ..empty_taskforce()
            },
            "true",
        ),
        refused_row(
            "an alias of a brand of a number under a key that is no number",
            "Taskforce",
            &Taskforce {
                lanyard: under(Lanyard(Burgee(1)), Lanyard(Burgee(2)), &7),
                ..empty_taskforce()
            },
            "1",
        ),
    ];
    #[cfg(feature = "chrono")]
    rows.extend(date_rows());
    rows
}

/// The payloads of a map keyed by a date through a brand and through an alias.
#[cfg(feature = "chrono")]
fn date_rows() -> Vec<Row> {
    let later = DateTime::<Utc>::from_timestamp_millis(1_709_208_000_100).unwrap();
    let held = Regatta {
        aliased: under(first(), later, &1),
        branded: under(Postmark(first()), Postmark(later), &2),
    };
    let stowed = StowedRegatta {
        aliased: under(first(), later, &"t".to_owned()),
        branded: under(Postmark(first()), Postmark(later), &"u".to_owned()),
    };
    vec![
        read_row("a date brand and a date alias", "Regatta", &held),
        read_row(
            "a date brand and a date alias that declare a parameter",
            "StowedRegatta<String>",
            &stowed,
        ),
        {
            let written = serde_json::to_string(&Regatta {
                aliased: HashMap::from([(first(), 1)]),
                branded: HashMap::new(),
            })
            .unwrap();
            let payload = written.replacen("2024-01-02T03:04:05.678Z", "yesterday", 1);
            serde_json::from_str::<Regatta>(&payload).unwrap_err();
            Row {
                expected:
                    "a date alias under a key that is no date: refused\n    bad key yesterday\n"
                        .to_owned(),
                name: "a date alias under a key that is no date",
                payload,
                read_as: "Regatta",
            }
        },
    ]
}

/// The payload of a [`Dockage`] with an empty label under each key, which the Swift read refuses
/// with the messages the Rust `validate()` answers, each key named as it is written.
fn refused_by_rule_row() -> Row {
    let empty = Cleat {
        label: String::new(),
    };
    let held = Dockage {
        callsign: HashMap::from([(Callsign("alpha".to_owned()), empty.clone())]),
        hull: HashMap::from([(7, empty.clone())]),
        signal: HashMap::from([(Signal(true), empty)]),
    };
    let messages = held.validate().unwrap_err();
    assert_eq!(messages.len(), 3, "{messages:?}");
    let name = "an empty label under a brand, a flag and an alias";
    let lines = messages.iter().fold(String::new(), |mut lines, line| {
        writeln!(lines, "    {line}").unwrap();
        lines
    });
    Row {
        expected: format!("{name}: refused\n{lines}"),
        name,
        payload: serde_json::to_string(&held).unwrap(),
        read_as: "Dockage",
    }
}

/// The payloads of each variant that holds the maps, in a type that declares a parameter and in
/// one that declares none.
fn variant_rows() -> Vec<Row> {
    let docked = || (callsigns(&1), callsigns(&2));
    let stowed = || (callsigns(&"t".to_owned()), callsigns(&3));
    let mut rows = Vec::new();
    for (name, read_as, payload) in [
        (
            "a variant, no tag",
            "Squadron",
            serde_json::to_string(&Squadron::Docked {
                callsign: docked().0,
                hailer: docked().1,
            }),
        ),
        (
            "a variant, a tag alone",
            "TaggedSquadron",
            serde_json::to_string(&TaggedSquadron::Docked {
                callsign: docked().0,
                hailer: docked().1,
            }),
        ),
        (
            "a variant, a tag and a content key",
            "AdjacentSquadron",
            serde_json::to_string(&AdjacentSquadron::Docked {
                callsign: docked().0,
                hailer: docked().1,
            }),
        ),
        (
            "a variant, untagged",
            "UntaggedSquadron",
            serde_json::to_string(&UntaggedSquadron::Docked {
                callsign: docked().0,
                hailer: docked().1,
            }),
        ),
        (
            "a variant, no tag, a parameter",
            "Convoy<String>",
            serde_json::to_string(&Convoy::Docked {
                callsign: stowed().0,
                hailer: stowed().1,
            }),
        ),
        (
            "a variant, a tag alone, a parameter",
            "TaggedConvoy<String>",
            serde_json::to_string(&TaggedConvoy::Docked {
                callsign: stowed().0,
                hailer: stowed().1,
            }),
        ),
        (
            "a variant, a tag and a content key, a parameter",
            "AdjacentConvoy<String>",
            serde_json::to_string(&AdjacentConvoy::Docked {
                callsign: stowed().0,
                hailer: stowed().1,
            }),
        ),
        (
            "a variant, untagged, a parameter",
            "UntaggedConvoy<String>",
            serde_json::to_string(&UntaggedConvoy::Docked {
                callsign: stowed().0,
                hailer: stowed().1,
            }),
        ),
    ] {
        rows.push(Row {
            expected: format!("{name}: read {}\n", payload.as_ref().unwrap()),
            name,
            payload: payload.unwrap(),
            read_as,
        });
    }
    rows
}

/// The text of every model compiles with warnings as errors, and every payload reads and writes in
/// Swift as the serde read reads and writes it: each key from the text serde writes for it, and a
/// key its inner type refuses refused.
#[test]
fn each_map_keyed_by_a_brand_or_an_alias_reads_and_writes_in_swift_as_serde_does() {
    let mut rows = struct_rows();
    rows.push(refused_by_rule_row());
    rows.extend(variant_rows());
    let expected: String = rows.iter().map(|row| row.expected.as_str()).collect();
    assert!(expected.contains(r#""alpha":1"#), "{expected}");
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
        cleat_swift::swift_definition(),
        callsign_swift::swift_definition(),
        burgee_swift::swift_definition(),
        signal_swift::swift_definition(),
        lanyard_swift::swift_definition(),
        unhashed_swift::swift_definition(),
        greeting_swift::swift_definition(),
        hull_swift::swift_definition(),
        hailer_swift::swift_definition(),
        berthing_swift::swift_definition(),
        taskforce_swift::swift_definition(),
        dockage_swift::swift_definition(),
        stowed_taskforce_swift::swift_definition(),
        armada_swift::swift_definition(),
        squadron_swift::swift_definition(),
        tagged_squadron_swift::swift_definition(),
        adjacent_squadron_swift::swift_definition(),
        untagged_squadron_swift::swift_definition(),
        convoy_swift::swift_definition(),
        tagged_convoy_swift::swift_definition(),
        adjacent_convoy_swift::swift_definition(),
        untagged_convoy_swift::swift_definition(),
        logroll_swift::swift_definition(),
        logroster_swift::swift_definition(),
    ];
    #[cfg(feature = "chrono")]
    parts.extend([
        postmark_swift::swift_definition(),
        postmark_key_swift::swift_definition(),
        regatta_swift::swift_definition(),
        stowed_regatta_swift::swift_definition(),
    ]);
    parts.extend([HARNESS.to_owned(), calls.join("\n")]);
    let source = parts.join("\n\n");
    let Some(run) = judged(
        "swift-wrapped-keys",
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
