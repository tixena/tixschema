//! An externally tagged enum flattened into its holder, read by the emitted Swift types compiled
//! with warnings as errors from the one entry under a variant's name or an alias, against what
//! serde reads: beside a member of its holder's own and a key no member reads, ahead of a function
//! and of a struct, under a `rename`, the enum's `rename_all` and an `alias`, a unit variant over
//! `null`, optional, and with a model its holder judges. Every member flattened after it is handed
//! the object without that entry. Entries under two names are refused, as TypeScript and Zod
//! refuse them, where serde reads the first: Swift keeps no order of an object's keys, so the run
//! is made three times. Each holder that reads is written back as serde writes it back, and each
//! variant of the enum, built in Swift beside a member of its holder's own, optional and under a
//! function, is written as serde writes it: a unit variant as an entry of its name over `null`. The
//! enum read alone is refused where its object holds other than one key, as serde refuses it, and
//! so is an enum declared below its holder where the object it is handed holds another key, which
//! serde reads past. A unit variant under a tag and a content key is read from an object with no
//! content key alone, as Zod reads it. The holders of `kotlin_tests/flattened_models.rs` and the
//! enum of `run_kotlin_refusals.rs` are declared where the Kotlin client is built.

#![cfg(all(feature = "swift", feature = "kotlin"))]

use alloc::collections::BTreeMap;
use core::fmt::Write as _;

use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::Value;

use super::flattened_models::{
    BELTING, Belting, CLEATING, Cleating, HEAVING, Heaving, Luffing, SCUDDING, STOWING, Scudding,
    Stowing, Tacking, UNDER_AN_ALIAS, Yawing, belting_swift, bunting_swift, cleating_swift,
    heaving_swift, jibing_swift, luffing_swift, quaying_swift, scudding_swift, stowing_swift,
    tacking_swift, yawing_swift,
};
use super::run_kotlin_refusals::{Becalming, becalming_swift};
use super::runtime::judged;
use super::swift_flattened_models::{
    ANCHORING, Anchoring, DREDGING, Dredging, FENDING, Fending, anchoring_swift, careen_swift,
    counting_swift, dredging_swift, fending_swift,
};
use super::tuple_variants_models::labelling_swift;

/// Names the runtime to run, for a machine that has one somewhere other than `PATH`.
const RUNTIME_VAR: &str = "TIXSCHEMA_SWIFT";

/// What the run declares beside the emitted types: the function the holders after an enum name,
/// answering the keys of the object it is handed, the function `Yawing` names, answering the
/// payload as it was handed, the text of a value written with its keys sorted, the text of a value
/// read and written back or the refusal of its read, what prints a row read, and the coding path a
/// refusal is thrown at.
const HELPERS: &str = r#"
func withItsKeys(_ payload: Any) -> Any {
  guard let object = payload as? [String: Any] else { return payload }
  return ["keys": object.keys.sorted()]
}

func asGiven(_ payload: Any) -> Any { payload }

func written<Read: Encodable>(_ value: Read) -> String {
  let encoder = JSONEncoder()
  encoder.outputFormatting = [.sortedKeys]
  return String(data: try! encoder.encode(value), encoding: .utf8)!
}

func writtenBack<Read: Codable>(_ type: Read.Type, _ payload: String) -> String {
  do {
    return written(try JSONDecoder().decode(type, from: Data(payload.utf8)))
  } catch DecodingError.dataCorrupted(let context) {
    return context.debugDescription
  } catch {
    return "\(error)"
  }
}

func entry<Read: Decodable>(
  _ at: Int, _ type: Read.Type, _ payload: String, _ shown: (Read) -> String
) {
  do {
    let read = try JSONDecoder().decode(type, from: Data(payload.utf8))
    print("\(at): \(shown(read))")
  } catch DecodingError.dataCorrupted(let context) {
    print("\(at): \(context.debugDescription)")
  } catch {
    print("\(at): \(error)")
  }
}

func refusedAt<Read: Decodable>(_ type: Read.Type, _ payload: String) -> String {
  do {
    _ = try JSONDecoder().decode(type, from: Data(payload.utf8))
    return "read"
  } catch DecodingError.dataCorrupted(let context) {
    return context.codingPath.map(\.stringValue).joined(separator: ".")
  } catch {
    return "\(error)"
  }
}
"#;

/// The second of two [`Stowing`]s holds entries under two variants' names: the refusal is thrown
/// at the coding path of that holder.
const LISTED: &str = concat!(
    r#"[{"Bay":{"label":"n"},"note":"a"},"#,
    r#"{"Bay":{"label":"n"},"Dock":{"label":"s"},"note":"a"}]"#
);

/// [`Becalming`]'s unit variant with no content, with `null` for one and with a value for one,
/// and its variant that holds a value: each payload, what serde reads of it written back with its
/// keys sorted or its refusal, and what the Swift read answers of it.
const UNDER_A_CONTENT_KEY: [(&str, &str, &str); 4] = [
    (
        r#"{"kind":"Lull"}"#,
        r#"{"kind":"Lull"}"#,
        r#"{"kind":"Lull"}"#,
    ),
    (
        r#"{"kind":"Lull","held":null}"#,
        r#"{"kind":"Lull"}"#,
        "Becalming: Lull takes no value",
    ),
    (
        r#"{"kind":"Lull","held":5}"#,
        "invalid type: integer `5`, expected unit variant Becalming::Lull at line 1 column 23",
        "Becalming: Lull takes no value",
    ),
    (
        r#"{"kind":"Breeze","held":true}"#,
        r#"{"held":true,"kind":"Breeze"}"#,
        r#"{"held":true,"kind":"Breeze"}"#,
    ),
];

/// The enum and the note of a [`Stowing`], as the Swift run prints them.
const ENUM_AND_NOTE: &str = r#""\(written($0.tacking)) \($0.note)""#;

/// The enum and the keys handed the function of a [`Heaving`].
const ENUM_AND_TALLY: &str = r#""\(written($0.tacking)) \(written($0.tally))""#;

/// The enum and the note of a holder of `Careen`.
const CAREEN_AND_NOTE: &str = r#""\(written($0.careen)) \($0.note)""#;

/// A payload of a holder of a flattened enum, the type it is read as, what prints the value read
/// in Swift, the body of a closure over the holder, and what the run prints of it.
struct EntryRow {
    json: &'static str,
    read_as: &'static str,
    shown: &'static str,
    told: String,
}

/// A payload of a holder of a flattened enum, the type it is read as, and what serde writes back
/// of it.
struct WrittenRow {
    json: &'static str,
    read_as: &'static str,
    written: fn(&str) -> String,
}

/// What the run prints of `json` read as `T`: what serde reads, as `shown` tells it, or what it
/// says of a payload it refuses, ahead of the position it says it at. Where `apart` names an enum
/// and the names `json` holds entries of it under, serde reads the first, and the Swift read
/// refuses the payload.
fn told<T>(json: &str, apart: Option<(&str, &str)>, shown: fn(&T) -> String) -> String
where
    T: DeserializeOwned,
{
    let read = serde_json::from_str::<T>(json);
    if let Some((named, found)) = apart {
        assert!(read.is_ok(), "serde reads the first entry of {json}");
        return format!("more than one variant of enum {named} found in flattened data: {found}");
    }
    read.map_or_else(
        |refused| {
            let refusal = refused.to_string();
            refusal
                .split_once(" at line")
                .map_or_else(|| refusal.clone(), |(said, _position)| said.to_owned())
        },
        |value| shown(&value),
    )
}

/// The JSON serde writes of `value`.
fn json<T>(value: &T) -> String
where
    T: Serialize,
{
    serde_json::to_string(value).unwrap()
}

/// What serde writes back of `json`, read as `T`, its keys sorted, as the Swift encoder of the run
/// writes them.
fn written_back<T>(json: &str) -> String
where
    T: DeserializeOwned + Serialize,
{
    let read: T = serde_json::from_str(json).unwrap();
    let keyed: BTreeMap<String, Value> =
        serde_json::from_str(&serde_json::to_string(&read).unwrap()).unwrap();
    serde_json::to_string(&keyed).unwrap()
}

/// Each row of a holder of a flattened `Tacking` or `Jibing`: beside a member of its own,
/// optional, ahead of a function and of a struct, under an alias, and declared below its holder.
fn tacking_rows() -> Vec<EntryRow> {
    let stowing = |json, apart| EntryRow {
        json,
        read_as: "Stowing",
        shown: ENUM_AND_NOTE,
        told: told(json, apart, |read: &Stowing| {
            format!("{} {}", self::json(&read.tacking), read.note)
        }),
    };
    let heaving = |json, apart| EntryRow {
        json,
        read_as: "Heaving",
        shown: ENUM_AND_TALLY,
        told: told(json, apart, |read: &Heaving| {
            format!("{} {}", self::json(&read.tacking), self::json(&read.tally))
        }),
    };
    let mut rows: Vec<EntryRow> = STOWING
        .into_iter()
        .enumerate()
        .map(|(at, json)| stowing(json, (at == 3).then_some(("Tacking", "Bay, Dock"))))
        .collect();
    rows.extend(BELTING.map(|json| EntryRow {
        json,
        read_as: "Belting",
        shown: ENUM_AND_NOTE,
        told: told(json, None, |read: &Belting| {
            format!("{} {}", self::json(&read.tacking), read.note)
        }),
    }));
    rows.extend(HEAVING.into_iter().enumerate().map(|(at, json)| {
        let apart = match at {
            1 | 2 => Some(("Tacking", "Bay, Dock")),
            3 => Some(("Tacking", "Bay, Quay")),
            _ => None,
        };
        heaving(json, apart)
    }));
    rows.extend(CLEATING.map(|json| EntryRow {
        json,
        read_as: "Cleating",
        shown: r#""\(written($0.jibing)) \(written($0.tally))""#,
        told: told(json, None, |read: &Cleating| {
            format!("{} {}", self::json(&read.jibing), self::json(&read.tally))
        }),
    }));
    rows.push(EntryRow {
        json: SCUDDING,
        read_as: "Scudding",
        shown: r#""\(written($0.hatch)) \(written($0.labelling)) \(written($0.tally))""#,
        told: told(SCUDDING, None, |read: &Scudding| {
            format!(
                "{} {} {}",
                json(&read.hatch),
                json(&read.labelling),
                json(&read.tally)
            )
        }),
    });
    rows.push(stowing(UNDER_AN_ALIAS[0], None));
    rows.push(heaving(UNDER_AN_ALIAS[1], None));
    rows.push(EntryRow {
        json: STOWING[0],
        read_as: "Luffing",
        shown: r#""\(written($0.quaying)) \($0.note)""#,
        told: told(STOWING[0], None, |read: &Luffing| {
            format!("{} {}", json(&read.quaying), read.note)
        }),
    });
    rows
}

/// Each row of a holder of a flattened `Careen`, whose model the holder judges: beside a member
/// of its own, ahead of a function, and optional.
fn careen_rows() -> Vec<EntryRow> {
    let mut rows: Vec<EntryRow> = ANCHORING
        .into_iter()
        .enumerate()
        .map(|(at, json)| EntryRow {
            json,
            read_as: "Anchoring",
            shown: CAREEN_AND_NOTE,
            told: told(
                json,
                (at == 4).then_some(("Careen", "Calm, Dock")),
                |read: &Anchoring| format!("{} {}", self::json(&read.careen), read.note),
            ),
        })
        .collect();
    rows.push(EntryRow {
        json: DREDGING,
        read_as: "Dredging",
        shown: r#""\(written($0.careen)) \(written($0.tally))""#,
        told: told(DREDGING, None, |read: &Dredging| {
            format!("{} {}", json(&read.careen), json(&read.tally))
        }),
    });
    rows.extend(FENDING.into_iter().enumerate().map(|(at, json)| EntryRow {
        json,
        read_as: "Fending",
        shown: CAREEN_AND_NOTE,
        told: told(
            json,
            (at == 3).then_some(("Careen", "Bay, Quay")),
            |read: &Fending| format!("{} {}", self::json(&read.careen), read.note),
        ),
    }));
    rows
}

/// What `swift -warnings-as-errors` prints of `main` over the emitted definitions of every holder
/// and what it flattens, or `None` where no toolchain is reachable.
fn printed(main: &str) -> Option<String> {
    let source = [
        "import Foundation".to_owned(),
        labelling_swift::swift_definition(),
        bunting_swift::swift_definition(),
        tacking_swift::swift_definition(),
        jibing_swift::swift_definition(),
        stowing_swift::swift_definition(),
        belting_swift::swift_definition(),
        yawing_swift::swift_definition(),
        heaving_swift::swift_definition(),
        cleating_swift::swift_definition(),
        scudding_swift::swift_definition(),
        luffing_swift::swift_definition(),
        quaying_swift::swift_definition(),
        counting_swift::swift_definition(),
        careen_swift::swift_definition(),
        anchoring_swift::swift_definition(),
        dredging_swift::swift_definition(),
        fending_swift::swift_definition(),
        becalming_swift::swift_definition(),
        HELPERS.to_owned(),
        main.to_owned(),
    ]
    .join("\n\n");
    let run = judged(
        "swift-flattened-enums",
        RUNTIME_VAR,
        "swift",
        &["-warnings-as-errors"],
        "main.swift",
        &source,
    )?;
    assert!(
        run.succeeded,
        "`swift -warnings-as-errors main.swift` failed.\n--- stdout ---\n{}\n--- stderr ---\n{}\n--- source ---\n{source}",
        run.stdout, run.stderr
    );
    Some(run.stdout)
}

/// A flattened externally tagged enum is read from the one entry of the object it is handed under
/// the name a variant is read under, and every member after it is handed the object without that
/// entry, as serde hands them: beside a member of its holder's own and a key no member reads,
/// ahead of a function and of a struct, under a `rename`, the enum's `rename_all` and an `alias`,
/// a unit variant over `null`, and optional. A payload with no such entry is refused, and an
/// optional enum reads `nil`. A model the holder judges is judged once, with the holder. Entries
/// under two names are refused at the holder's coding path, the names in the order the enum
/// declares its variants, on every run. An enum declared below its holder is read from the object
/// the holder hands it.
#[test]
fn a_flattened_enum_reads_in_swift_the_one_entry_under_a_variants_name() {
    let mut rows = tacking_rows();
    rows.extend(careen_rows());
    let mut expected = String::new();
    let mut main = String::new();
    for (at, row) in rows.iter().enumerate() {
        let _ = writeln!(expected, "{at}: {}", row.told);
        let _ = writeln!(
            main,
            "entry({at}, {}.self, #\"{}\"#) {{ {} }}",
            row.read_as, row.json, row.shown
        );
    }
    let _ = writeln!(
        main,
        "print(\"path: \\(refusedAt([Stowing].self, #\"{LISTED}\"#))\")"
    );
    expected.push_str("path: Index 1\n");
    assert_eq!(
        expected,
        "0: {\"Bay\":{\"label\":\"n\"}} a\n\
         1: {\"Bay\":{\"label\":\"n\"}} a\n\
         2: {\"Bay\":{\"label\":\"n\"}} a\n\
         3: more than one variant of enum Tacking found in flattened data: Bay, Dock\n\
         4: \"Calm\" a\n\
         5: {\"Held\":\"x\"} a\n\
         6: no variant of enum Tacking found in flattened data\n\
         7: {\"Bay\":{\"label\":\"n\"}} a\n\
         8: null a\n\
         9: null a\n\
         10: {\"Bay\":{\"label\":\"n\"}} {\"keys\":[\"north\"]}\n\
         11: more than one variant of enum Tacking found in flattened data: Bay, Dock\n\
         12: more than one variant of enum Tacking found in flattened data: Bay, Dock\n\
         13: more than one variant of enum Tacking found in flattened data: Bay, Quay\n\
         14: {\"pier\":{\"label\":\"n\"}} {\"keys\":[\"north\"]}\n\
         15: {\"open_bay\":{\"label\":\"n\"}} {\"keys\":[\"north\"]}\n\
         16: no variant of enum Jibing found in flattened data\n\
         17: {\"Bay\":{\"label\":\"n\"}} {\"label\":\"m\"} {\"keys\":[\"north\"]}\n\
         18: {\"Dock\":{\"label\":\"s\"}} a\n\
         19: {\"Dock\":{\"label\":\"s\"}} {\"keys\":[\"north\"]}\n\
         20: {\"Bay\":{\"label\":\"n\"}} a\n\
         21: {\"Bay\":{\"label\":\"n\"}} a\n\
         22: 'Bay.label': too short: minimum length is 1, got 0\n\
         23: \"Calm\" a\n\
         24: {\"Dock\":{\"label\":\"s\"}} a\n\
         25: more than one variant of enum Careen found in flattened data: Calm, Dock\n\
         26: {\"Dock\":{\"label\":\"s\"}} {\"keys\":[\"north\"]}\n\
         27: null a\n\
         28: {\"Bay\":{\"label\":\"n\"}} a\n\
         29: 'Bay.label': too short: minimum length is 1, got 0\n\
         30: more than one variant of enum Careen found in flattened data: Bay, Quay\n\
         path: Index 1\n"
    );
    for run in ["first", "second", "third"] {
        let Some(printed) = printed(&main) else {
            return;
        };
        assert_eq!(printed, expected, "the {run} run");
    }
}

/// Each holder of a flattened externally tagged enum that reads is written back as serde writes it
/// back, a unit variant as an entry of its name over `null`.
#[test]
fn a_holder_of_a_flattened_enum_writes_back_in_swift_what_serde_writes_back() {
    let held = |read_as, json, written| WrittenRow {
        json,
        read_as,
        written,
    };
    let rows = [
        held("Stowing", STOWING[0], written_back::<Stowing>),
        held("Stowing", STOWING[1], written_back::<Stowing>),
        held("Stowing", STOWING[2], written_back::<Stowing>),
        held("Stowing", STOWING[5], written_back::<Stowing>),
        held("Belting", BELTING[0], written_back::<Belting>),
        held("Belting", BELTING[1], written_back::<Belting>),
        held("Belting", BELTING[2], written_back::<Belting>),
        held("Heaving", HEAVING[0], written_back::<Heaving>),
        held("Cleating", CLEATING[0], written_back::<Cleating>),
        held("Cleating", CLEATING[1], written_back::<Cleating>),
        held("Scudding", SCUDDING, written_back::<Scudding>),
        held("Stowing", UNDER_AN_ALIAS[0], written_back::<Stowing>),
        held("Heaving", UNDER_AN_ALIAS[1], written_back::<Heaving>),
        held("Anchoring", ANCHORING[0], written_back::<Anchoring>),
        held("Anchoring", ANCHORING[3], written_back::<Anchoring>),
        held("Dredging", DREDGING, written_back::<Dredging>),
        held("Fending", FENDING[0], written_back::<Fending>),
        held("Fending", FENDING[1], written_back::<Fending>),
        held("Luffing", STOWING[0], written_back::<Luffing>),
        held("Stowing", STOWING[4], written_back::<Stowing>),
        held("Anchoring", ANCHORING[2], written_back::<Anchoring>),
    ];
    let mut expected = String::new();
    let mut main = String::new();
    for (at, row) in rows.iter().enumerate() {
        let _ = writeln!(expected, "{at}: {}", (row.written)(row.json));
        let _ = writeln!(
            main,
            "print(\"{at}: \\(written(try! JSONDecoder().decode({}.self, \
             from: Data(#\"{}\"#.utf8))))\")",
            row.read_as, row.json
        );
    }
    assert_eq!(
        expected,
        "0: {\"Bay\":{\"label\":\"n\"},\"note\":\"a\"}\n\
         1: {\"Bay\":{\"label\":\"n\"},\"note\":\"a\"}\n\
         2: {\"Bay\":{\"label\":\"n\"},\"note\":\"a\"}\n\
         3: {\"Held\":\"x\",\"note\":\"a\"}\n\
         4: {\"Bay\":{\"label\":\"n\"},\"note\":\"a\"}\n\
         5: {\"note\":\"a\"}\n\
         6: {\"note\":\"a\"}\n\
         7: {\"Bay\":{\"label\":\"n\"},\"keys\":[\"north\"]}\n\
         8: {\"keys\":[\"north\"],\"pier\":{\"label\":\"n\"}}\n\
         9: {\"keys\":[\"north\"],\"open_bay\":{\"label\":\"n\"}}\n\
         10: {\"Bay\":{\"label\":\"n\"},\"keys\":[\"north\"],\"label\":\"m\"}\n\
         11: {\"Dock\":{\"label\":\"s\"},\"note\":\"a\"}\n\
         12: {\"Dock\":{\"label\":\"s\"},\"keys\":[\"north\"]}\n\
         13: {\"Bay\":{\"label\":\"n\"},\"note\":\"a\"}\n\
         14: {\"Dock\":{\"label\":\"s\"},\"note\":\"a\"}\n\
         15: {\"Dock\":{\"label\":\"s\"},\"keys\":[\"north\"]}\n\
         16: {\"note\":\"a\"}\n\
         17: {\"Bay\":{\"label\":\"n\"},\"note\":\"a\"}\n\
         18: {\"Bay\":{\"label\":\"n\"},\"note\":\"a\"}\n\
         19: {\"Calm\":null,\"note\":\"a\"}\n\
         20: {\"Calm\":null,\"note\":\"a\"}\n"
    );
    let Some(printed) = printed(&main) else {
        return;
    };
    assert_eq!(printed, expected);
}

/// A flattened externally tagged enum built in Swift is written as serde writes it, beside a
/// member of its holder's own, optional and under a function: a variant that holds a value as its
/// entry, and a unit variant as its name over `null`. What serde writes is read and written back
/// as it was, but a unit variant's entry under a function: the enum's own read takes it, and
/// refuses it, as the Zod schema refuses it.
#[test]
fn a_flattened_enum_writes_in_swift_as_serde_writes_it() {
    let tackings = [
        (
            Tacking::Bay {
                label: "n".to_owned(),
            },
            ".bay(TackingBayPayload(label: \"n\"))",
        ),
        (Tacking::Calm, ".calm"),
        (
            Tacking::Dock {
                label: "s".to_owned(),
            },
            ".dock(TackingDockPayload(label: \"s\"))",
        ),
        (Tacking::Held("x".to_owned()), ".held(\"x\")"),
    ];
    let note = || "a".to_owned();
    let mut rows = Vec::new();
    for (tacking, built) in tackings {
        let unit = tacking == Tacking::Calm;
        let stowing = Stowing {
            note: note(),
            tacking: tacking.clone(),
        };
        let belting = Belting {
            note: note(),
            tacking: Some(tacking.clone()),
        };
        let yawing = Yawing {
            note: note(),
            tacking,
        };
        rows.extend([
            ("Stowing", json(&stowing), built, None),
            ("Belting", json(&belting), built, None),
            (
                "Yawing",
                json(&yawing),
                built,
                unit.then_some("unknown tag Calm"),
            ),
        ]);
    }
    let unheld = Belting {
        note: note(),
        tacking: None,
    };
    rows.push(("Belting", json(&unheld), "nil", None));
    let mut expected = String::new();
    let mut main = String::new();
    for (holder, serde_wrote, built, refused) in &rows {
        let sorted = written_back::<Value>(serde_wrote);
        let _ = writeln!(
            expected,
            "{holder}: {sorted} {}",
            refused.unwrap_or(&sorted)
        );
        let _ = writeln!(
            main,
            "print(\"{holder}: \\(written({holder}(note: \"a\", tacking: {built}))) \
             \\(writtenBack({holder}.self, #\"{serde_wrote}\"#))\")"
        );
    }
    assert_eq!(
        expected,
        "Stowing: {\"Bay\":{\"label\":\"n\"},\"note\":\"a\"} \
         {\"Bay\":{\"label\":\"n\"},\"note\":\"a\"}\n\
         Belting: {\"Bay\":{\"label\":\"n\"},\"note\":\"a\"} \
         {\"Bay\":{\"label\":\"n\"},\"note\":\"a\"}\n\
         Yawing: {\"Bay\":{\"label\":\"n\"},\"note\":\"a\"} \
         {\"Bay\":{\"label\":\"n\"},\"note\":\"a\"}\n\
         Stowing: {\"Calm\":null,\"note\":\"a\"} {\"Calm\":null,\"note\":\"a\"}\n\
         Belting: {\"Calm\":null,\"note\":\"a\"} {\"Calm\":null,\"note\":\"a\"}\n\
         Yawing: {\"Calm\":null,\"note\":\"a\"} unknown tag Calm\n\
         Stowing: {\"Dock\":{\"label\":\"s\"},\"note\":\"a\"} \
         {\"Dock\":{\"label\":\"s\"},\"note\":\"a\"}\n\
         Belting: {\"Dock\":{\"label\":\"s\"},\"note\":\"a\"} \
         {\"Dock\":{\"label\":\"s\"},\"note\":\"a\"}\n\
         Yawing: {\"Dock\":{\"label\":\"s\"},\"note\":\"a\"} \
         {\"Dock\":{\"label\":\"s\"},\"note\":\"a\"}\n\
         Stowing: {\"Held\":\"x\",\"note\":\"a\"} {\"Held\":\"x\",\"note\":\"a\"}\n\
         Belting: {\"Held\":\"x\",\"note\":\"a\"} {\"Held\":\"x\",\"note\":\"a\"}\n\
         Yawing: {\"Held\":\"x\",\"note\":\"a\"} {\"Held\":\"x\",\"note\":\"a\"}\n\
         Belting: {\"note\":\"a\"} {\"note\":\"a\"}\n"
    );
    let Some(printed) = printed(&main) else {
        return;
    };
    assert_eq!(printed, expected);
}

/// What the run prints of each payload of `rows` read alone as a [`Tacking`]: the value serde
/// reads, written back, or, for a payload serde refuses, the refusal the Swift read throws.
fn tacking_told(rows: &[(&str, Option<&str>)]) -> String {
    let mut expected = String::new();
    for (at, &(json, refusal)) in rows.iter().enumerate() {
        let read = serde_json::from_str::<Tacking>(json);
        let told = match refusal {
            Some(said) => {
                assert!(read.is_err(), "serde refuses {json} read as a Tacking");
                said.to_owned()
            }
            None => self::json(&read.unwrap()),
        };
        let _ = writeln!(expected, "{at}: {told}");
    }
    expected
}

/// An externally tagged enum read alone is read from an object of one key, or a unit variant from
/// its name, as serde reads it: an object of several keys, two variants' entries or a variant's
/// entry beside another key, is refused with `more than one key for Tacking`, and an empty object
/// with `empty object for Tacking`, as serde refuses each, on every run.
#[test]
fn an_externally_tagged_enum_reads_in_swift_an_object_of_one_key_alone() {
    let rows = [
        (
            r#"{"Bay":{"label":"n"},"Dock":{"label":"s"}}"#,
            Some("more than one key for Tacking"),
        ),
        (
            r#"{"Bay":{"label":"n"},"north":2}"#,
            Some("more than one key for Tacking"),
        ),
        ("{}", Some("empty object for Tacking")),
        (r#"{"Bay":{"label":"n"}}"#, None),
        (r#""Calm""#, None),
        (r#"{"Held":"x"}"#, None),
    ];
    let expected = tacking_told(&rows);
    assert_eq!(
        expected,
        "0: more than one key for Tacking\n\
         1: more than one key for Tacking\n\
         2: empty object for Tacking\n\
         3: {\"Bay\":{\"label\":\"n\"}}\n\
         4: \"Calm\"\n\
         5: {\"Held\":\"x\"}\n"
    );
    let mut main = String::new();
    for (at, (json, _refusal)) in rows.iter().enumerate() {
        let _ = writeln!(
            main,
            "entry({at}, Tacking.self, #\"{json}\"#) {{ written($0) }}"
        );
    }
    for run in ["first", "second", "third"] {
        let Some(printed) = printed(&main) else {
            return;
        };
        assert_eq!(printed, expected, "the {run} run");
    }
}

/// A unit variant under a tag and a content key is read in Swift where the object holds no content
/// key, and an object that holds one is refused with `DecodingError.dataCorrupted`, `null` among
/// its values, as Zod refuses it: its schema of the variant is a strict object of the tag alone.
/// serde reads the variant over `null`. The variant that holds a value reads as it did.
#[test]
fn a_unit_variant_under_a_content_key_is_read_in_swift_where_the_object_holds_no_content() {
    let serde_told = UNDER_A_CONTENT_KEY.map(|(payload, _, _)| {
        serde_json::from_str::<Becalming>(payload).map_or_else(
            |refusal| refusal.to_string(),
            |read| written_back::<Value>(&json(&read)),
        )
    });
    assert_eq!(
        serde_told,
        UNDER_A_CONTENT_KEY.map(|(_, serde, _)| serde.to_owned()),
        "serde no longer answers each payload as it did"
    );
    let zod = Becalming::zod_schema();
    assert!(
        zod.contains("z.strictObject({\n  kind: z.literal(\"Lull\"),\n})"),
        "Zod no longer reads the unit variant as an object of its tag alone: {zod}"
    );
    let mut expected = String::new();
    let mut main = String::new();
    for (at, (payload, _, swift)) in UNDER_A_CONTENT_KEY.into_iter().enumerate() {
        let _ = writeln!(expected, "{at}: {swift}");
        let _ = writeln!(
            main,
            "entry({at}, Becalming.self, #\"{payload}\"#) {{ written($0) }}"
        );
    }
    let Some(printed) = printed(&main) else {
        return;
    };
    assert_eq!(printed, expected, "the Swift read is not the answer above");
}

/// The README's sentence on the Swift read of a unit variant under a content key shows the
/// payloads and the refusal the run above measures.
#[test]
fn the_readme_shows_the_swift_read_of_a_unit_variant_under_a_content_key() {
    let section = include_str!("../../README.md")
        .split_once("#### A Swift read refuses what `validate()` refuses")
        .and_then(|(_, rest)| rest.split_once("### Literal Values"))
        .map(|(section, _)| section)
        .unwrap();
    for (payload, _, swift) in &UNDER_A_CONTENT_KEY[..3] {
        assert!(
            section.contains(&format!("`{payload}`")),
            "not in the README: {payload}"
        );
        assert!(
            swift.starts_with('{') || section.contains(&format!("`{swift}`")),
            "not in the README: {swift}"
        );
    }
}

/// An externally tagged enum declared below the holder that flattens it is read from the object
/// the holder hands it: as serde reads it where that object holds the enum's entry alone, and
/// refused where it holds any other key, which serde reads past.
#[test]
fn an_enum_declared_below_its_holder_is_refused_in_swift_beside_another_key() {
    let beside = STOWING[2];
    let serde_read = [STOWING[0], beside].map(|payload| {
        told(payload, None, |read: &Luffing| {
            format!("{} {}", json(&read.quaying), read.note)
        })
    });
    assert_eq!(serde_read, ["{\"Bay\":{\"label\":\"n\"}} a"; 2]);
    let mut main = String::new();
    for (at, payload) in [STOWING[0], beside].into_iter().enumerate() {
        let _ = writeln!(
            main,
            "entry({at}, Luffing.self, #\"{payload}\"#) {{ \"\\(written($0.quaying)) \\($0.note)\" }}"
        );
    }
    for run in ["first", "second", "third"] {
        let Some(printed) = printed(&main) else {
            return;
        };
        assert_eq!(
            printed,
            format!("0: {}\n1: more than one key for Quaying\n", serde_read[0]),
            "the {run} run"
        );
    }
}
