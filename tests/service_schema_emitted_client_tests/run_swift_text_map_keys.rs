//! A map under a key Swift holds as the text serde writes for it, read and written by the emitted
//! Swift types compiled with warnings as errors: a `NaiveDate`, a `NaiveTime`, a `NaiveDateTime`
//! and a type parameter, as a field, as a member of a variant under each tagging and as a brand's
//! value, held by a type that declares a parameter and by one that declares none. Each payload the
//! serde read reads prints what it prints, and a model such a map holds is refused as the Rust
//! `validate()` reports it.

#![cfg(feature = "swift")]

use core::fmt::Write as _;
use core::hash::Hash;
use std::collections::HashMap;

use chrono::{NaiveDate, NaiveDateTime, NaiveTime, TimeDelta};
use serde::Serialize;
use serde::de::DeserializeOwned;

use super::run_swift_read::normalized;
use super::runtime::judged;
use super::text_map_keys_models::{
    AdjacentBulletin, AdjacentTidings, Almanac, Bulletin, Datebook, Ephemeris, Gazetteer,
    Hourbook, Logsheet, Moorage, TaggedBulletin, TaggedTidings, Tidings, UntaggedBulletin,
    UntaggedTidings, adjacent_bulletin_swift, adjacent_tidings_swift, almanac_swift,
    bulletin_swift, datebook_swift, day, ephemeris_swift, gazetteer_swift, hour, hourbook_swift,
    logsheet_swift, moment, moorage_swift, tagged_bulletin_swift, tagged_tidings_swift,
    tidings_swift, untagged_bulletin_swift, untagged_tidings_swift,
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

/// A map under each of `keys`, each entry holding `held`.
fn keyed<Key, Held>(keys: [Key; 2], held: &Held) -> HashMap<Key, Held>
where
    Key: Eq + Hash,
    Held: Clone,
{
    keys.into_iter().map(|key| (key, held.clone())).collect()
}

/// The maps a `Dated` variant and [`Ephemeris`] hold, each of two entries.
fn dated<Held>(
    held: &Held,
) -> (
    HashMap<NaiveDate, Held>,
    HashMap<NaiveTime, String>,
    HashMap<NaiveDateTime, Held>,
)
where
    Held: Clone,
{
    let later = TimeDelta::days(1);
    (
        keyed([day(), day() + later], held),
        keyed([hour(), NaiveTime::MIN], &"h".to_owned()),
        keyed([moment(), moment() + later], held),
    )
}

/// [`Almanac`] with each map of two entries, and the model under the later date labelled `label`.
fn almanac(label: &str) -> Almanac {
    let (days, hours, moments) = dated(&"d".to_owned());
    let labelled = |text: &str| Moorage {
        label: text.to_owned(),
    };
    Almanac {
        berths: HashMap::from([
            (day(), labelled("m")),
            (day() + TimeDelta::days(1), labelled(label)),
        ]),
        days,
        hours,
        moments,
    }
}

/// The payloads of a struct and of a brand, in a type that declares a parameter and in one that
/// declares none.
fn struct_and_brand_rows() -> Vec<Row> {
    let (days, hours, moments) = dated(&"d".to_owned());
    let broken = almanac("");
    vec![
        read_row("a struct", "Almanac", &almanac("n")),
        refused_row(
            "a struct whose model breaks its rule",
            "Almanac",
            &broken,
            broken.validate(),
        ),
        read_row(
            "a struct that declares a parameter",
            "Ephemeris<String>",
            &Ephemeris {
                days: days.clone(),
                hours: hours.clone(),
                moments: moments.clone(),
            },
        ),
        read_row(
            "the parameter as a key",
            "Gazetteer<String>",
            &Gazetteer {
                by_stowed: keyed(["a".to_owned(), "b".to_owned()], &1_u32),
            },
        ),
        read_row(
            "the parameter as a key, filled with a number",
            "Gazetteer<UInt32>",
            &Gazetteer {
                by_stowed: keyed([7_u32, 8], &1_u32),
            },
        ),
        read_row("a brand", "Datebook", &Datebook(days)),
        read_row("an empty brand", "Datebook", &Datebook(HashMap::new())),
        read_row("a brand with a named member", "Hourbook", &Hourbook { hours }),
        read_row(
            "a brand that declares a parameter",
            "Logsheet<String>",
            &Logsheet(moments),
        ),
    ]
}

/// The payloads of a variant under each tagging, in a type that declares a parameter and in one
/// that declares none.
fn variant_rows() -> Vec<Row> {
    let (days, hours, moments) = dated(&"d".to_owned());
    vec![
        read_row(
            "a variant under no tag",
            "Tidings",
            &Tidings::Dated {
                days: days.clone(),
                hours: hours.clone(),
                moments: moments.clone(),
            },
        ),
        read_row(
            "a variant under a tag",
            "TaggedTidings",
            &TaggedTidings::Dated {
                days: days.clone(),
                hours: hours.clone(),
                moments: moments.clone(),
            },
        ),
        read_row(
            "a variant under a tag and a content key",
            "AdjacentTidings",
            &AdjacentTidings::Dated {
                days: days.clone(),
                hours: hours.clone(),
                moments: moments.clone(),
            },
        ),
        read_row(
            "an untagged variant",
            "UntaggedTidings",
            &UntaggedTidings::Dated {
                days: days.clone(),
                hours: hours.clone(),
                moments: moments.clone(),
            },
        ),
        read_row(
            "an untagged variant after the maps",
            "UntaggedTidings",
            &UntaggedTidings::Plain {
                note: "n".to_owned(),
            },
        ),
        read_row(
            "a variant under no tag that declares a parameter",
            "Bulletin<String>",
            &Bulletin::Dated {
                days: days.clone(),
                hours: hours.clone(),
                moments: moments.clone(),
            },
        ),
        read_row(
            "a variant under a tag that declares a parameter",
            "TaggedBulletin<String>",
            &TaggedBulletin::Dated {
                days: days.clone(),
                hours: hours.clone(),
                moments: moments.clone(),
            },
        ),
        read_row(
            "a variant under a tag and a content key that declares a parameter",
            "AdjacentBulletin<String>",
            &AdjacentBulletin::Dated {
                days: days.clone(),
                hours: hours.clone(),
                moments: moments.clone(),
            },
        ),
        read_row(
            "an untagged variant that declares a parameter",
            "UntaggedBulletin<String>",
            &UntaggedBulletin::Dated {
                days,
                hours,
                moments,
            },
        ),
        read_row(
            "an untagged variant after the maps that declares a parameter",
            "UntaggedBulletin<String>",
            &UntaggedBulletin::Plain {
                note: "n".to_owned(),
            },
        ),
    ]
}

/// The text of every model compiles with warnings as errors, and every payload reads and writes in
/// Swift as the serde read reads and writes it: each key as the text serde writes for it. A model
/// such a map holds that breaks its rule is refused as the Rust `validate()` reports it.
#[test]
fn each_map_under_a_key_held_as_its_text_reads_and_writes_in_swift_as_serde_does() {
    let mut rows = struct_and_brand_rows();
    rows.extend(variant_rows());
    let expected: String = rows.iter().map(|row| row.expected.as_str()).collect();
    assert!(
        expected.contains(
            "a struct whose model breaks its rule: refused\n    \
             'berths.2024-01-03.label': too short: minimum length is 1, got 0\n"
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
        moorage_swift::swift_definition(),
        almanac_swift::swift_definition(),
        ephemeris_swift::swift_definition(),
        gazetteer_swift::swift_definition(),
        tidings_swift::swift_definition(),
        tagged_tidings_swift::swift_definition(),
        adjacent_tidings_swift::swift_definition(),
        untagged_tidings_swift::swift_definition(),
        bulletin_swift::swift_definition(),
        tagged_bulletin_swift::swift_definition(),
        adjacent_bulletin_swift::swift_definition(),
        untagged_bulletin_swift::swift_definition(),
        datebook_swift::swift_definition(),
        hourbook_swift::swift_definition(),
        logsheet_swift::swift_definition(),
        HARNESS.to_owned(),
        calls.join("\n"),
    ]
    .join("\n\n");
    let Some(run) = judged(
        "swift-text-map-keys",
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
