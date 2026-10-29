//! A map keyed by a date, read and written by the emitted Swift types compiled with warnings as
//! errors: as a field, as a member of a variant under each tagging and as a brand's value, and in a
//! list, in a list of optionals, in an optional list and in an optional, held by a type that
//! declares a parameter and by one that declares none. Each payload the serde read reads prints
//! what it prints, each key in the text chrono writes, and a key that is no date is refused, as the
//! serde read refuses it.

#![cfg(feature = "swift")]

use core::fmt::Write as _;
use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::Serialize;
use serde::de::DeserializeOwned;

use super::date_map_keys_models::{
    AdjacentShift, AdjacentWatch, Chronicle, Convoy, Daybook, Daybooks, Diary, Logbook, Moment,
    Reminder, Sailing, Shift, StowedLogbook, TaggedShift, TaggedWatch, Tidetable, Timetable,
    UntaggedShift, UntaggedWatch, Watch, adjacent_shift_swift, adjacent_watch_swift,
    chronicle_swift, convoy_swift, daybook_swift, daybooks_swift, diary_swift, first,
    logbook_swift, moment_swift, reminder_swift, sailing_swift, second, shift_swift,
    stowed_logbook_swift, tagged_shift_swift, tagged_watch_swift, tidetable_swift, timetable_swift,
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
  } catch DecodingError.typeMismatch(let type, let context) {
    print("\(name): no \(type): \(context.debugDescription)")
  } catch {
    print("\(name): \(error)")
  }
}
"#;

/// What a run prints of a payload whose keys read as one key: that it is refused with a message
/// that names a key, which of the keys is the first one read being no order the payload fixes.
const COLLISION_HARNESS: &str = r#"
func collision<Read: Codable>(_ name: String, _ type: Read.Type, _ payload: String) {
  do {
    _ = try JSONDecoder().decode(type, from: Data(payload.utf8))
    print("\(name): read")
  } catch DecodingError.dataCorrupted(let context) {
    let named = context.debugDescription.hasPrefix("repeated key 2024-01-02T03:04:05.678")
    print("\(name): refused, naming a key: \(named)")
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

/// The payloads whose keys read as one key in Swift, each refused by naming a key.
const COLLIDING: [(&str, &str, &str); 1] = [(
    "keys finer than a millisecond",
    "Timetable",
    r#"{"by_at":{"2024-01-02T03:04:05.678100Z":1,"2024-01-02T03:04:05.678900Z":2},"sailings":{}}"#,
)];

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
        read_row(
            "a list, a list of optionals, an optional list and an optional",
            "Logbook",
            &Logbook {
                listed: vec![keyed(&1), HashMap::new()],
                listed_or_none: vec![Some(keyed(&2)), None],
                maybe: Some(keyed(&3)),
                maybe_listed: Some(vec![keyed(&4)]),
                sailing_lists: vec![keyed(&sailing())],
            },
        ),
        read_row(
            "a list, a list of optionals, an optional list and an optional, none",
            "Logbook",
            &Logbook {
                listed: Vec::new(),
                listed_or_none: vec![None],
                maybe: None,
                maybe_listed: None,
                sailing_lists: Vec::new(),
            },
        ),
        read_row(
            "a list, a list of optionals, an optional list and an optional that declare a \
             parameter",
            "StowedLogbook<String>",
            &StowedLogbook {
                listed: vec![keyed(&"t".to_owned())],
                listed_or_none: vec![None, Some(keyed(&"u".to_owned()))],
                maybe: Some(keyed(&"v".to_owned())),
                maybe_listed: Some(vec![keyed(&"w".to_owned())]),
            },
        ),
        read_row(
            "a brand over a list",
            "Daybooks",
            &Daybooks(vec![keyed(&"d".to_owned())]),
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
        refused_row(
            "a list under a key that is no date",
            "Logbook",
            &Logbook {
                listed: vec![once(1)],
                listed_or_none: Vec::new(),
                maybe: None,
                maybe_listed: None,
                sailing_lists: Vec::new(),
            },
            "input contains invalid characters at line 1 column 23",
            BAD_KEY,
        ),
        refused_row(
            "an optional under a key that is no date",
            "StowedLogbook<String>",
            &StowedLogbook {
                listed: Vec::new(),
                listed_or_none: Vec::new(),
                maybe: Some(once("v".to_owned())),
                maybe_listed: None,
            },
            "input contains invalid characters at line 1 column 53",
            BAD_KEY,
        ),
        refused_row(
            "a brand over a list under a key that is no date",
            "Daybooks",
            &Daybooks(vec![once("d".to_owned())]),
            "input contains invalid characters at line 1 column 13",
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
            "no UntaggedWatch: no untagged member matched",
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
            "no UntaggedShift<String>: no untagged member matched",
        ),
    ]
}

/// A model whose map is keyed by dates and whose value breaks its rule: the payload serde writes
/// for `held`, refused by the Swift read with each message the Rust `validate()` answers, which
/// names the key as serde writes it.
fn validated_row<Held>(
    name: &'static str,
    read_as: &'static str,
    held: &Held,
    answered: Result<(), Vec<String>>,
    key: &str,
) -> Row
where
    Held: Serialize,
{
    let messages = answered.unwrap_err();
    assert!(
        messages.iter().all(|said| said.contains(key)),
        "{messages:?}"
    );
    let lines = messages.iter().fold(String::new(), |mut lines, said| {
        let _ = writeln!(lines, "    {said}");
        lines
    });
    Row {
        expected: format!("{name}: refused\n{lines}"),
        name,
        payload: serde_json::to_string(held).unwrap(),
        read_as,
    }
}

/// A model with the one date `at`, read from `payload` and written back as `written`.
fn moment_row(name: &'static str, payload: &str, written: &str) -> Row {
    assert!(serde_json::from_str::<Moment>(payload).is_ok(), "{payload}");
    Row {
        expected: format!("{name}: read {written}\n"),
        name,
        payload: payload.to_owned(),
        read_as: "Moment",
    }
}

/// A model with an optional date under `as_number`, read from `payload` as serde reads it.
fn reminder_row(name: &'static str, payload: &str) -> Row {
    let read: Reminder = serde_json::from_str(payload).unwrap();
    Row {
        expected: format!("{name}: read {}\n", serde_json::to_string(&read).unwrap()),
        name,
        payload: payload.to_owned(),
        read_as: "Reminder",
    }
}

/// A sailing whose label breaks its rule.
fn unlabelled() -> Sailing {
    Sailing {
        label: String::new(),
    }
}

/// A date of whole seconds, which chrono writes with no fraction.
fn whole() -> DateTime<Utc> {
    DateTime::from_timestamp(1_704_164_645, 0).unwrap()
}

/// The rows whose payload breaks a rule of a model held in a map keyed by a date, refused with
/// the key named as serde writes it.
fn validated_rows() -> Vec<Row> {
    let key = "2024-01-02T03:04:05.678Z";
    let timetable = Timetable {
        by_at: HashMap::new(),
        sailings: once(unlabelled()),
    };
    let whole_timetable = Timetable {
        by_at: HashMap::new(),
        sailings: HashMap::from([(whole(), unlabelled())]),
    };
    let logged = Watch::Logged {
        by_at: HashMap::new(),
        sailings: once(unlabelled()),
    };
    let tagged = TaggedWatch::Logged {
        by_at: HashMap::new(),
        sailings: once(unlabelled()),
    };
    let adjacent = AdjacentWatch::Logged {
        by_at: HashMap::new(),
        sailings: once(unlabelled()),
    };
    let untagged = UntaggedWatch::Logged {
        by_at: HashMap::new(),
        sailings: once(unlabelled()),
    };
    let logbook = Logbook {
        listed: Vec::new(),
        listed_or_none: Vec::new(),
        maybe: None,
        maybe_listed: None,
        sailing_lists: vec![once(unlabelled())],
    };
    let convoy = Convoy {
        held: "h".to_owned(),
        sailings: once(unlabelled()),
    };
    vec![
        validated_row(
            "a struct whose key has a fraction",
            "Timetable",
            &timetable,
            timetable.validate(),
            key,
        ),
        validated_row(
            "a struct whose key is whole seconds",
            "Timetable",
            &whole_timetable,
            whole_timetable.validate(),
            "2024-01-02T03:04:05Z",
        ),
        validated_row(
            "a variant under no tag",
            "Watch",
            &logged,
            logged.validate(),
            key,
        ),
        validated_row(
            "a variant under a tag",
            "TaggedWatch",
            &tagged,
            tagged.validate(),
            key,
        ),
        validated_row(
            "a variant under a tag and a content key",
            "AdjacentWatch",
            &adjacent,
            adjacent.validate(),
            key,
        ),
        validated_row(
            "an untagged variant",
            "UntaggedWatch",
            &untagged,
            untagged.validate(),
            key,
        ),
        validated_row(
            "a list of maps",
            "Logbook",
            &logbook,
            logbook.validate(),
            key,
        ),
        validated_row(
            "a type that declares a parameter",
            "Convoy<String>",
            &convoy,
            convoy.validate(),
            key,
        ),
    ]
}

/// The rows of a date written as chrono writes it: a key of whole seconds, a date field of whole
/// seconds, of milliseconds and finer than a millisecond, which Swift's `Date` reads at the
/// millisecond, and an optional date under `as_number`.
fn written_as_serde_writes_rows() -> Vec<Row> {
    let by_whole = Timetable {
        by_at: HashMap::from([(whole(), 1)]),
        sailings: HashMap::new(),
    };
    vec![
        read_row("a key of whole seconds", "Timetable", &by_whole),
        reminder_row(
            "an optional date under as_number",
            r#"{"due":1758000000000}"#,
        ),
        reminder_row("an optional date under as_number, null", r#"{"due":null}"#),
        reminder_row("an optional date under as_number, absent", "{}"),
        moment_row(
            "a date of whole seconds",
            r#"{"at":"2024-01-02T03:04:05Z"}"#,
            r#"{"at":"2024-01-02T03:04:05Z"}"#,
        ),
        moment_row(
            "a date of milliseconds",
            r#"{"at":"2024-01-02T03:04:05.678Z"}"#,
            r#"{"at":"2024-01-02T03:04:05.678Z"}"#,
        ),
        moment_row(
            "a date finer than a millisecond",
            r#"{"at":"2024-01-02T03:04:05.678901Z"}"#,
            r#"{"at":"2024-01-02T03:04:05.678Z"}"#,
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
    rows.extend(validated_rows());
    rows.extend(written_as_serde_writes_rows());
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
    let collisions: Vec<String> = COLLIDING
        .iter()
        .map(|(name, read_as, payload)| {
            format!("collision(\"{name}\", {read_as}.self, #\"{payload}\"#)")
        })
        .collect();
    let with_collisions = COLLIDING.iter().fold(expected, |printed, (name, _, _)| {
        format!("{printed}{name}: refused, naming a key: true\n")
    });
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
        logbook_swift::swift_definition(),
        stowed_logbook_swift::swift_definition(),
        daybooks_swift::swift_definition(),
        moment_swift::swift_definition(),
        convoy_swift::swift_definition(),
        reminder_swift::swift_definition(),
        HARNESS.to_owned(),
        COLLISION_HARNESS.to_owned(),
        calls.join("\n"),
        collisions.join("\n"),
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
    assert_eq!(normalized(&run.stdout), normalized(&with_collisions));
}
