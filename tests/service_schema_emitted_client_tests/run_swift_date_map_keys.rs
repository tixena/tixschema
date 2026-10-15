//! A map keyed by a date, read and written by the emitted Swift types compiled with warnings as
//! errors: as a field, as a member of a variant under each tagging and as a brand's value, held by
//! a type that declares a parameter and by one that declares none. Each payload the serde read
//! reads prints what it prints, each key in the text chrono writes, and a key that is no date is
//! refused, as the serde read refuses it.

#![cfg(feature = "swift")]

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::Serialize;
use serde::de::DeserializeOwned;

use super::date_map_keys_models::{
    AdjacentShift, AdjacentWatch, Chronicle, Daybook, Diary, Sailing, Shift, TaggedShift,
    TaggedWatch, Tidetable, Timetable, UntaggedShift, UntaggedWatch, Watch, adjacent_shift_swift,
    adjacent_watch_swift, chronicle_swift, daybook_swift, diary_swift, first, sailing_swift,
    second, shift_swift, tagged_shift_swift, tagged_watch_swift, tidetable_swift, timetable_swift,
    untagged_shift_swift, untagged_watch_swift, watch_swift,
};
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

/// The key chrono writes for [`first`].
const FIRST_WRITTEN: &str = r#""2024-01-02T03:04:05.678Z""#;

/// The key every refused payload holds in place of [`FIRST_WRITTEN`].
const NO_DATE: &str = r#""yesterday""#;

/// What the Swift read prints for a payload that holds [`NO_DATE`] as a key.
const BAD_KEY: &str = "refused\n    bad key yesterday";

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

/// The payload serde writes for `held` with its first key [`NO_DATE`], which the serde read
/// refuses with `answered`, held to `printed`, what the Swift read prints of it.
fn refused_row<Held>(
    name: &'static str,
    read_as: &'static str,
    held: &Held,
    answered: &str,
    printed: &str,
) -> Row
where
    Held: DeserializeOwned + Serialize,
{
    let written = serde_json::to_string(held).unwrap();
    assert!(written.contains(FIRST_WRITTEN), "{written}");
    let payload = written.replacen(FIRST_WRITTEN, NO_DATE, 1);
    let refused = serde_json::from_str::<Held>(&payload).err().unwrap();
    assert_eq!(refused.to_string(), answered, "{payload}");
    Row {
        expected: format!("{name}: {printed}\n"),
        name,
        payload,
        read_as,
    }
}

/// A map keyed by [`first`] and [`second`], each entry holding `held`.
fn keyed<Held>(held: &Held) -> HashMap<DateTime<Utc>, Held>
where
    Held: Clone,
{
    HashMap::from([(first(), held.clone()), (second(), held.clone())])
}

/// A map keyed by [`first`] alone, holding `held`.
fn once<Held>(held: Held) -> HashMap<DateTime<Utc>, Held> {
    HashMap::from([(first(), held)])
}

/// A model that keeps its rule.
fn sailing() -> Sailing {
    Sailing {
        label: "s".to_owned(),
    }
}

/// The payloads of a struct and of a brand, in a type that declares a parameter and in one that
/// declares none.
fn struct_and_brand_rows() -> Vec<Row> {
    vec![
        read_row(
            "a struct",
            "Timetable",
            &Timetable {
                by_at: keyed(&1),
                sailings: keyed(&sailing()),
            },
        ),
        read_row(
            "a struct with empty maps",
            "Timetable",
            &Timetable {
                by_at: HashMap::new(),
                sailings: HashMap::new(),
            },
        ),
        read_row(
            "a struct that declares a parameter",
            "Tidetable<String>",
            &Tidetable {
                by_at: keyed(&"t".to_owned()),
                counts: keyed(&2),
            },
        ),
        read_row("a brand", "Daybook", &Daybook(keyed(&"d".to_owned()))),
        read_row(
            "a brand with a named member",
            "Chronicle",
            &Chronicle { by_at: keyed(&3) },
        ),
        read_row(
            "a brand that declares a parameter",
            "Diary<String>",
            &Diary(keyed(&"d".to_owned())),
        ),
    ]
}

/// The payloads of a variant under each tagging, in a type that declares a parameter and in one
/// that declares none.
fn variant_rows() -> Vec<Row> {
    vec![
        read_row(
            "a variant under no tag",
            "Watch",
            &Watch::Logged {
                by_at: keyed(&1),
                sailings: keyed(&sailing()),
            },
        ),
        read_row(
            "a variant under a tag",
            "TaggedWatch",
            &TaggedWatch::Logged {
                by_at: keyed(&1),
                sailings: keyed(&sailing()),
            },
        ),
        read_row(
            "a variant under a tag and a content key",
            "AdjacentWatch",
            &AdjacentWatch::Logged {
                by_at: keyed(&1),
                sailings: keyed(&sailing()),
            },
        ),
        read_row(
            "an untagged variant",
            "UntaggedWatch",
            &UntaggedWatch::Logged {
                by_at: keyed(&1),
                sailings: keyed(&sailing()),
            },
        ),
        read_row(
            "an untagged variant after the maps",
            "UntaggedWatch",
            &UntaggedWatch::Plain {
                note: "n".to_owned(),
            },
        ),
        read_row(
            "a variant under no tag that declares a parameter",
            "Shift<String>",
            &Shift::Logged {
                by_at: keyed(&"t".to_owned()),
                counts: keyed(&2),
            },
        ),
        read_row(
            "a variant under a tag that declares a parameter",
            "TaggedShift<String>",
            &TaggedShift::Logged {
                by_at: keyed(&"t".to_owned()),
                counts: keyed(&2),
            },
        ),
        read_row(
            "a variant under a tag and a content key that declares a parameter",
            "AdjacentShift<String>",
            &AdjacentShift::Logged {
                by_at: keyed(&"t".to_owned()),
                counts: keyed(&2),
            },
        ),
        read_row(
            "an untagged variant that declares a parameter",
            "UntaggedShift<String>",
            &UntaggedShift::Logged {
                by_at: keyed(&"t".to_owned()),
                counts: keyed(&2),
            },
        ),
        read_row(
            "an untagged variant after the maps that declares a parameter",
            "UntaggedShift<String>",
            &UntaggedShift::Plain {
                note: "n".to_owned(),
            },
        ),
    ]
}

/// The payloads of a struct and of a brand whose first key is no date, each refused by the serde
/// read and by the Swift read.
fn refused_struct_and_brand_rows() -> Vec<Row> {
    vec![
        refused_row(
            "a struct under a key that is no date",
            "Timetable",
            &Timetable {
                by_at: once(1),
                sailings: HashMap::new(),
            },
            "input contains invalid characters at line 1 column 21",
            BAD_KEY,
        ),
        refused_row(
            "a struct under a key that is no date, over a model",
            "Timetable",
            &Timetable {
                by_at: HashMap::new(),
                sailings: once(sailing()),
            },
            "input contains invalid characters at line 1 column 35",
            BAD_KEY,
        ),
        refused_row(
            "a struct that declares a parameter under a key that is no date",
            "Tidetable<String>",
            &Tidetable {
                by_at: once("t".to_owned()),
                counts: HashMap::new(),
            },
            "input contains invalid characters at line 1 column 21",
            BAD_KEY,
        ),
        refused_row(
            "a brand under a key that is no date",
            "Daybook",
            &Daybook(once("d".to_owned())),
            "input contains invalid characters at line 1 column 12",
            BAD_KEY,
        ),
        refused_row(
            "a brand with a named member under a key that is no date",
            "Chronicle",
            &Chronicle { by_at: once(3) },
            "input contains invalid characters at line 1 column 12",
            BAD_KEY,
        ),
        refused_row(
            "a brand that declares a parameter under a key that is no date",
            "Diary<String>",
            &Diary(once("d".to_owned())),
            "input contains invalid characters at line 1 column 12",
            BAD_KEY,
        ),
    ]
}

/// The payloads of a variant under each tagging whose first key is no date, each refused by the
/// serde read and by the Swift read.
fn refused_variant_rows() -> Vec<Row> {
    vec![
        refused_row(
            "a variant under no tag under a key that is no date",
            "Watch",
            &Watch::Logged {
                by_at: once(1),
                sailings: HashMap::new(),
            },
            "input contains invalid characters at line 1 column 31",
            BAD_KEY,
        ),
        refused_row(
            "a variant under a tag under a key that is no date",
            "TaggedWatch",
            &TaggedWatch::Logged {
                by_at: once(1),
                sailings: HashMap::new(),
            },
            "input contains invalid characters",
            BAD_KEY,
        ),
        refused_row(
            "a variant under a tag and a content key under a key that is no date",
            "AdjacentWatch",
            &AdjacentWatch::Logged {
                by_at: once(1),
                sailings: HashMap::new(),
            },
            "input contains invalid characters at line 1 column 45",
            BAD_KEY,
        ),
        refused_row(
            "an untagged variant under a key that is no date",
            "UntaggedWatch",
            &UntaggedWatch::Logged {
                by_at: once(1),
                sailings: HashMap::new(),
            },
            "data did not match any variant of untagged enum UntaggedWatch",
            "DecodingError.typeMismatch: expected value of type UntaggedWatch. \
             Debug description: no untagged member matched",
        ),
        refused_row(
            "a variant under no tag that declares a parameter under a key that is no date",
            "Shift<String>",
            &Shift::Logged {
                by_at: once("t".to_owned()),
                counts: HashMap::new(),
            },
            "input contains invalid characters at line 1 column 31",
            BAD_KEY,
        ),
        refused_row(
            "a variant under a tag that declares a parameter under a key that is no date",
            "TaggedShift<String>",
            &TaggedShift::Logged {
                by_at: once("t".to_owned()),
                counts: HashMap::new(),
            },
            "input contains invalid characters",
            BAD_KEY,
        ),
        refused_row(
            "a variant under a tag and a content key that declares a parameter under a key that \
             is no date",
            "AdjacentShift<String>",
            &AdjacentShift::Logged {
                by_at: once("t".to_owned()),
                counts: HashMap::new(),
            },
            "input contains invalid characters at line 1 column 45",
            BAD_KEY,
        ),
        refused_row(
            "an untagged variant that declares a parameter under a key that is no date",
            "UntaggedShift<String>",
            &UntaggedShift::Logged {
                by_at: once("t".to_owned()),
                counts: HashMap::new(),
            },
            "data did not match any variant of untagged enum UntaggedShift",
            "DecodingError.typeMismatch: expected value of type UntaggedShift<String>. \
             Debug description: no untagged member matched",
        ),
    ]
}

/// The text of every model compiles with warnings as errors, and every payload reads and writes in
/// Swift as the serde read reads and writes it: each key in the text chrono writes, and a key that
/// is no date refused.
#[test]
fn each_map_keyed_by_a_date_reads_and_writes_in_swift_as_serde_does() {
    let mut rows = struct_and_brand_rows();
    rows.extend(variant_rows());
    rows.extend(refused_struct_and_brand_rows());
    rows.extend(refused_variant_rows());
    let expected: String = rows.iter().map(|row| row.expected.as_str()).collect();
    assert!(
        expected.contains("\"2024-01-02T03:04:05.678Z\":1")
            && expected.contains("\"2024-02-29T12:00:00.100Z\":1"),
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
        sailing_swift::swift_definition(),
        timetable_swift::swift_definition(),
        tidetable_swift::swift_definition(),
        watch_swift::swift_definition(),
        tagged_watch_swift::swift_definition(),
        adjacent_watch_swift::swift_definition(),
        untagged_watch_swift::swift_definition(),
        shift_swift::swift_definition(),
        tagged_shift_swift::swift_definition(),
        adjacent_shift_swift::swift_definition(),
        untagged_shift_swift::swift_definition(),
        daybook_swift::swift_definition(),
        chronicle_swift::swift_definition(),
        diary_swift::swift_definition(),
        HARNESS.to_owned(),
        calls.join("\n"),
    ]
    .join("\n\n");
    let Some(run) = judged(
        "swift-date-map-keys",
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
