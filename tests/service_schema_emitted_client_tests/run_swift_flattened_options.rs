//! A flattened `Option` member beside a member of its holder's own, alone in its holder and in a
//! variant under a tag key, a struct of two keys, a struct declared below its holder, a map of
//! dates alone in its holder, a struct flattened bare beside a member of its holder's own, a model
//! `validate()` walks and a struct that reads the keys no member around it took, a struct read from
//! what a function answers, and an enum of each tagging and one declared below its holder, each
//! read and written back by the emitted Swift types compiled with warnings as errors, against what
//! serde reads and writes: read from serde's payload where the member holds a value and where it
//! holds none of its type's keys, and refused where the model it holds breaks its rule or the
//! function answers what its type does not read. Where the object holds a key of the type that
//! does not read, serde reads `None` and the Swift read refuses the payload, as TypeScript and Zod
//! refuse it: a struct's key, an entry under a name an externally tagged enum's variant is read
//! under, and the tag or the content key of an enum under a tag key. A type whose keys are not
//! known, an untagged enum and an enum declared below its holder among them, is read as `nil`
//! there, as serde reads it, and so is the map of dates where an entry is no date. The map of
//! dates absent is built in code.

#![cfg(feature = "swift")]

use alloc::collections::BTreeMap;
use core::fmt::Write as _;

use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::Value;

use super::runtime::judged;
use super::swift_flattened_options_models::{
    Crowning, EASING, Easing, FLAKING, FLAKING_ABSENT, FLAKING_MISTYPED, Fidding, Flaking, HEELING,
    Heeling, KINKING_SHORT, Kinking, LAPPING, LAPPING_ABSENT, LAPPING_MISTYPED, LOWERING, Lapping,
    Lowering, PAINTER, PITCHING, Painter, Pitching, Ratline, SENNIT, SENNIT_ABSENT,
    SENNIT_MISTYPED, SQUARING, Sennit, Squaring, TARRING_SHORT, Tarring, Veering, becket_swift,
    crowning_swift, easing_swift, fidding_swift, flaking_swift, gasket_swift, grommet_swift,
    heeling_swift, kinking_swift, lapping_swift, lowering_swift, nippering_swift, painter_swift,
    pitching_swift, plaiting_swift, ratline_swift, sennit_swift, shackle_swift, squaring_swift,
    tabling_swift, tarring_swift, thimble_swift, toggle_swift, veering_swift,
};
#[cfg(feature = "chrono")]
use super::swift_flattened_options_models::{SWAGING, SWAGING_MISTYPED, Swaging, swaging_swift};

/// Names the runtime to run, for a machine that has one somewhere other than `PATH`.
const RUNTIME_VAR: &str = "TIXSCHEMA_SWIFT";

/// What the run declares beside the emitted types: the function a model names, answering the
/// payload as it was handed, the text of a value written with its keys sorted, the read of a
/// payload, and what a read that refuses one says, a key it misses in serde's words and a value
/// of the wrong type by its coding path.
const HELPERS: &str = "
func asGiven(_ payload: Any) -> Any { payload }

func written<Read: Encodable>(_ value: Read) -> String {
  let encoder = JSONEncoder()
  encoder.outputFormatting = [.sortedKeys]
  return String(data: try! encoder.encode(value), encoding: .utf8)!
}

func read<Read: Decodable>(_ type: Read.Type, _ payload: String) -> Read {
  try! JSONDecoder().decode(type, from: Data(payload.utf8))
}

func refusal<Read: Decodable>(_ type: Read.Type, _ payload: String) -> String {
  do {
    _ = try JSONDecoder().decode(type, from: Data(payload.utf8))
    return \"read\"
  } catch DecodingError.dataCorrupted(let context) {
    return context.debugDescription
  } catch DecodingError.keyNotFound(let key, _) {
    return \"missing field `\\(key.stringValue)`\"
  } catch DecodingError.typeMismatch(_, let context) {
    return \"mismatch at '\\(context.codingPath.map { $0.stringValue }.joined(separator: \".\"))'\"
  } catch {
    return \"\\(error)\"
  }
}
";

/// A holder printed by the run: its name, the Swift that prints it, what serde tells of it, and
/// what the Swift read tells in its place where it refuses what serde reads.
struct Row {
    decided: Option<&'static str>,
    name: &'static str,
    serde: String,
    swift: String,
}

impl Row {
    /// The holder `value`, built in Swift by `swift`.
    #[cfg(feature = "chrono")]
    fn built<T>(name: &'static str, swift: &str, value: &T) -> Self
    where
        T: Serialize,
    {
        Self {
            decided: None,
            name,
            serde: written(value),
            swift: format!("written({swift})"),
        }
    }

    /// The holder read as `T`, named `read_as` in Swift, from `json`, which serde reads and the
    /// Swift read refuses with `refused`.
    fn decided<T>(name: &'static str, read_as: &str, json: &str, refused: &'static str) -> Self
    where
        T: DeserializeOwned + Serialize,
    {
        Self {
            decided: Some(refused),
            name,
            serde: written(&serde_json::from_str::<T>(json).unwrap()),
            swift: format!("refusal({read_as}.self, #\"{json}\"#)"),
        }
    }

    /// The holder read as `T`, named `read_as` in Swift, from `json`.
    fn read<T>(name: &'static str, read_as: &str, json: &str) -> Self
    where
        T: DeserializeOwned + Serialize,
    {
        Self {
            decided: None,
            name,
            serde: written(&serde_json::from_str::<T>(json).unwrap()),
            swift: format!("written(read({read_as}.self, #\"{json}\"#))"),
        }
    }

    /// What the read of `json` as `T`, named `read_as` in Swift, says as it refuses the payload,
    /// ahead of any position serde says it at.
    fn refused<T>(name: &'static str, read_as: &str, json: &str) -> Self
    where
        T: DeserializeOwned,
    {
        let refusal = serde_json::from_str::<T>(json).err().unwrap().to_string();
        Self {
            decided: None,
            name,
            serde: refusal.split_once(" at line").map_or_else(
                || refusal.clone(),
                |(refused, _position)| refused.to_owned(),
            ),
            swift: format!("refusal({read_as}.self, #\"{json}\"#)"),
        }
    }
}

/// The text of `value` as serde writes it, its keys sorted, as the Swift encoder of the run
/// writes them.
fn written<T>(value: &T) -> String
where
    T: Serialize,
{
    let text = serde_json::to_string(value).unwrap();
    let keyed: BTreeMap<String, Value> = serde_json::from_str(&text).unwrap();
    serde_json::to_string(&keyed).unwrap()
}

/// Each holder of an enum read from each payload of its own: the enum absent, held, and there with
/// what it does not read.
fn enum_rows() -> Vec<Row> {
    vec![
        Row::read::<Easing>("easing absent", "Easing", EASING[0]),
        Row::read::<Easing>("easing", "Easing", EASING[1]),
        Row::decided::<Easing>(
            "easing mistyped",
            "Easing",
            EASING[2],
            "mismatch at 'Round.turns'",
        ),
        Row::decided::<Easing>(
            "easing unit mistyped",
            "Easing",
            EASING[3],
            "unknown tag Calm",
        ),
        Row::read::<Lowering>("lowering absent", "Lowering", LOWERING[0]),
        Row::read::<Lowering>("lowering", "Lowering", LOWERING[1]),
        Row::read::<Lowering>("lowering unit", "Lowering", LOWERING[2]),
        Row::decided::<Lowering>(
            "lowering mistyped",
            "Lowering",
            LOWERING[3],
            "mismatch at 'turns'",
        ),
        Row::decided::<Lowering>(
            "lowering unnamed",
            "Lowering",
            LOWERING[4],
            "unknown tag Gale",
        ),
        Row::read::<Lowering>("lowering untagged", "Lowering", LOWERING[5]),
        Row::read::<Squaring>("squaring absent", "Squaring", SQUARING[0]),
        Row::read::<Squaring>("squaring", "Squaring", SQUARING[1]),
        Row::read::<Squaring>("squaring unit", "Squaring", SQUARING[2]),
        Row::decided::<Squaring>(
            "squaring mistyped",
            "Squaring",
            SQUARING[3],
            "mismatch at 'body.turns'",
        ),
        Row::decided::<Squaring>(
            "squaring unnamed",
            "Squaring",
            SQUARING[4],
            "unknown tag Gale",
        ),
        Row::decided::<Squaring>(
            "squaring untagged",
            "Squaring",
            SQUARING[5],
            "missing field `kind`",
        ),
        Row::read::<Heeling>("heeling absent", "Heeling", HEELING[0]),
        Row::read::<Heeling>("heeling", "Heeling", HEELING[1]),
        Row::read::<Heeling>("heeling mistyped", "Heeling", HEELING[2]),
        Row::read::<Pitching>("pitching absent", "Pitching", PITCHING[0]),
        Row::read::<Pitching>("pitching", "Pitching", PITCHING[1]),
        Row::read::<Pitching>("pitching mistyped", "Pitching", PITCHING[2]),
    ]
}

/// Each holder read from a payload that holds the flattened member, then from one that holds
/// none of its type's keys, then from one that holds a key its type does not read, and each whose
/// model breaks its rule; then each holder of an enum.
fn rows() -> Vec<Row> {
    let mistyped = "mismatch at 'label'";
    let mut rows = vec![
        Row::read::<Lapping>("lapping", "Lapping", LAPPING),
        Row::read::<Flaking>("flaking", "Flaking", FLAKING),
        Row::read::<Sennit>("sennit", "Sennit", SENNIT),
        Row::read::<Painter>("painter", "Painter", PAINTER),
        Row::read::<Ratline>("ratline", "Ratline", FLAKING),
        Row::read::<Crowning>("crowning", "Crowning", LAPPING),
        Row::read::<Kinking>("kinking", "Kinking", FLAKING),
        Row::read::<Tarring>("tarring", "Tarring", LAPPING),
        Row::read::<Fidding>("fidding", "Fidding", LAPPING),
        Row::read::<Veering>("veering", "Veering", LAPPING),
        Row::read::<Lapping>("lapping absent", "Lapping", LAPPING_ABSENT),
        Row::read::<Flaking>("flaking absent", "Flaking", FLAKING_ABSENT),
        Row::read::<Sennit>("sennit absent", "Sennit", SENNIT_ABSENT),
        Row::read::<Painter>("painter absent", "Painter", FLAKING_ABSENT),
        Row::read::<Ratline>("ratline absent", "Ratline", FLAKING_ABSENT),
        Row::read::<Kinking>("kinking absent", "Kinking", FLAKING_ABSENT),
        Row::read::<Tarring>("tarring absent", "Tarring", LAPPING_ABSENT),
        Row::read::<Fidding>("fidding absent", "Fidding", FLAKING_ABSENT),
        Row::read::<Fidding>("fidding unread", "Fidding", LAPPING_ABSENT),
        Row::read::<Ratline>("ratline mistyped", "Ratline", FLAKING_MISTYPED),
        Row::decided::<Lapping>("lapping mistyped", "Lapping", LAPPING_MISTYPED, mistyped),
        Row::decided::<Flaking>("flaking mistyped", "Flaking", FLAKING_MISTYPED, mistyped),
        Row::decided::<Sennit>("sennit mistyped", "Sennit", SENNIT_MISTYPED, mistyped),
        Row::decided::<Kinking>("kinking mistyped", "Kinking", FLAKING_MISTYPED, mistyped),
        Row::decided::<Tarring>("tarring mistyped", "Tarring", LAPPING_MISTYPED, mistyped),
        Row::decided::<Painter>(
            "painter partial",
            "Painter",
            FLAKING,
            "missing field `turns`",
        ),
        Row::refused::<Kinking>("kinking refused", "Kinking", KINKING_SHORT),
        Row::refused::<Tarring>("tarring refused", "Tarring", TARRING_SHORT),
        Row::refused::<Veering>("veering refused", "Veering", LAPPING_ABSENT),
    ];
    rows.extend(enum_rows());
    #[cfg(feature = "chrono")]
    rows.extend([
        Row::read::<Swaging>("swaging", "Swaging", SWAGING),
        Row::read::<Swaging>("swaging mistyped", "Swaging", SWAGING_MISTYPED),
        Row::built(
            "swaging absent",
            "Swaging(stamps: nil)",
            &Swaging { stamps: None },
        ),
    ]);
    rows
}

/// What `swift -warnings-as-errors` prints of each of `rows` written, or `None` where no
/// toolchain is reachable.
fn printed(rows: &[Row]) -> Option<String> {
    let mut definitions = vec![
        tabling_swift::swift_definition(),
        lapping_swift::swift_definition(),
        flaking_swift::swift_definition(),
        sennit_swift::swift_definition(),
        gasket_swift::swift_definition(),
        painter_swift::swift_definition(),
        ratline_swift::swift_definition(),
        nippering_swift::swift_definition(),
        crowning_swift::swift_definition(),
        plaiting_swift::swift_definition(),
        kinking_swift::swift_definition(),
        tarring_swift::swift_definition(),
        fidding_swift::swift_definition(),
        veering_swift::swift_definition(),
        thimble_swift::swift_definition(),
        grommet_swift::swift_definition(),
        becket_swift::swift_definition(),
        toggle_swift::swift_definition(),
        easing_swift::swift_definition(),
        lowering_swift::swift_definition(),
        squaring_swift::swift_definition(),
        heeling_swift::swift_definition(),
        pitching_swift::swift_definition(),
        shackle_swift::swift_definition(),
    ];
    #[cfg(feature = "chrono")]
    definitions.push(swaging_swift::swift_definition());
    let main = rows.iter().fold(String::new(), |mut main, row| {
        let _ = writeln!(main, "print(\"{}: \\({})\")", row.name, row.swift);
        main
    });
    let source = [
        "import Foundation".to_owned(),
        definitions.join("\n\n"),
        HELPERS.to_owned(),
        main,
    ]
    .join("\n\n");
    let run = judged(
        "swift-flattened-options",
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

/// A flattened `Option` that holds a value writes its keys beside its holder's own, and one that
/// holds none writes nothing, alone in its holder and in a variant under a tag key too, as serde
/// writes them; a map of dates writes the text serde writes, and a struct flattened bare writes
/// its keys beside its holder's own. A flattened `Option` is read as `nil` where the object holds
/// none of its type's keys, as serde reads it, and a model it holds that breaks its rule refuses
/// the payload in serde's words; one that names a function is refused where its type does not
/// read what the function answers. Where the object holds a key its type does not read, a key
/// held as a number or one of two keys, a variant's entry, tag or content that does not read, or
/// a content with no tag, serde reads `None` and the Swift read refuses the payload, but for a
/// type whose keys are not known and a map of dates one of whose entries is no date, each read as
/// `nil` as serde reads it: an untagged enum and an enum declared below its holder among them. An
/// enum under a tag key alone is `nil` where its tag is absent, a member of a variant there or not.
#[test]
fn a_flattened_option_reads_and_writes_in_swift_what_serde_reads_and_writes() {
    let rows = rows();
    let told = |says: &dyn Fn(&Row) -> String| {
        rows.iter().fold(String::new(), |mut told, row| {
            let _ = writeln!(told, "{}: {}", row.name, says(row));
            told
        })
    };
    let shared = "lapping: {\"label\":\"north\",\"note\":\"n\"}\n\
         flaking: {\"label\":\"north\"}\n\
         sennit: {\"kind\":\"Laid\",\"label\":\"north\",\"note\":\"n\"}\n\
         painter: {\"label\":\"north\",\"turns\":2}\n\
         ratline: {\"label\":\"north\"}\n\
         crowning: {\"label\":\"north\",\"note\":\"n\"}\n\
         kinking: {\"label\":\"north\"}\n\
         tarring: {\"label\":\"north\",\"note\":\"n\"}\n\
         fidding: {\"label\":\"north\",\"note\":\"n\"}\n\
         veering: {\"label\":\"north\",\"note\":\"n\"}\n\
         lapping absent: {\"note\":\"n\"}\n\
         flaking absent: {}\n\
         sennit absent: {\"kind\":\"Laid\",\"note\":\"n\"}\n\
         painter absent: {}\n\
         ratline absent: {}\n\
         kinking absent: {}\n\
         tarring absent: {\"note\":\"n\"}\n\
         fidding absent: {}\n\
         fidding unread: {}\n\
         ratline mistyped: {}\n\
         lapping mistyped: {\"note\":\"n\"}\n\
         flaking mistyped: {}\n\
         sennit mistyped: {\"kind\":\"Laid\",\"note\":\"n\"}\n\
         kinking mistyped: {}\n\
         tarring mistyped: {\"note\":\"n\"}\n\
         painter partial: {}\n\
         kinking refused: 'label': too short: minimum length is 2, got 1\n\
         tarring refused: 'label': too short: minimum length is 2, got 1\n\
         veering refused: missing field `label`\n\
         easing absent: {\"note\":\"n\"}\n\
         easing: {\"Round\":{\"turns\":2},\"note\":\"n\"}\n\
         easing mistyped: {\"note\":\"n\"}\n\
         easing unit mistyped: {\"note\":\"n\"}\n\
         lowering absent: {\"note\":\"n\"}\n\
         lowering: {\"kind\":\"Round\",\"note\":\"n\",\"turns\":2}\n\
         lowering unit: {\"kind\":\"Calm\",\"note\":\"n\"}\n\
         lowering mistyped: {\"note\":\"n\"}\n\
         lowering unnamed: {\"note\":\"n\"}\n\
         lowering untagged: {\"note\":\"n\"}\n\
         squaring absent: {\"note\":\"n\"}\n\
         squaring: {\"body\":{\"turns\":2},\"kind\":\"Round\",\"note\":\"n\"}\n\
         squaring unit: {\"kind\":\"Calm\",\"note\":\"n\"}\n\
         squaring mistyped: {\"note\":\"n\"}\n\
         squaring unnamed: {\"note\":\"n\"}\n\
         squaring untagged: {\"note\":\"n\"}\n\
         heeling absent: {\"note\":\"n\"}\n\
         heeling: {\"note\":\"n\",\"turns\":2}\n\
         heeling mistyped: {\"note\":\"n\"}\n\
         pitching absent: {\"note\":\"n\"}\n\
         pitching: {\"Round\":{\"turns\":2},\"note\":\"n\"}\n\
         pitching mistyped: {\"note\":\"n\"}\n";
    #[cfg(feature = "chrono")]
    let dates = "swaging: {\"north\":\"2024-01-02T03:04:05.678Z\"}\n\
         swaging mistyped: {}\n\
         swaging absent: {}\n";
    #[cfg(not(feature = "chrono"))]
    let dates = "";
    assert_eq!(told(&|row| row.serde.clone()), format!("{shared}{dates}"));
    let Some(printed) = printed(&rows) else {
        return;
    };
    assert_eq!(
        printed,
        told(&|row| row.decided.map_or_else(|| row.serde.clone(), str::to_owned))
    );
}
