//! A flattened `Option` member beside a member of its holder's own, alone in its holder and in a
//! variant under a tag key, a map of dates alone in its holder, and a struct flattened bare beside
//! a member of its holder's own, each written by the emitted Swift types compiled with warnings as
//! errors, against what serde writes: read from serde's payload where the member holds a value,
//! and built in code where it holds none.

#![cfg(feature = "swift")]

use alloc::collections::BTreeMap;
use core::fmt::Write as _;

use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::Value;

use super::runtime::judged;
use super::swift_flattened_options_models::{
    Crowning, FLAKING, Flaking, LAPPING, Lapping, SENNIT, Sennit, crowning_swift, flaking_swift,
    lapping_swift, sennit_swift, tabling_swift,
};
#[cfg(feature = "chrono")]
use super::swift_flattened_options_models::{SWAGING, Swaging, swaging_swift};

/// Names the runtime to run, for a machine that has one somewhere other than `PATH`.
const RUNTIME_VAR: &str = "TIXSCHEMA_SWIFT";

/// What the run declares beside the emitted types: the text of a value written with its keys
/// sorted, and the read of a payload.
const HELPERS: &str = "
func written<Read: Encodable>(_ value: Read) -> String {
  let encoder = JSONEncoder()
  encoder.outputFormatting = [.sortedKeys]
  return String(data: try! encoder.encode(value), encoding: .utf8)!
}

func read<Read: Decodable>(_ type: Read.Type, _ payload: String) -> Read {
  try! JSONDecoder().decode(type, from: Data(payload.utf8))
}
";

/// A holder printed by the run: its name, the Swift that builds it, and the text serde writes of
/// it.
struct Row {
    name: &'static str,
    serde: String,
    swift: String,
}

impl Row {
    /// The holder `value`, built in Swift by `swift`.
    fn built<T>(name: &'static str, swift: &str, value: &T) -> Self
    where
        T: Serialize,
    {
        Self {
            name,
            serde: written(value),
            swift: swift.to_owned(),
        }
    }

    /// The holder read as `T`, named `read_as` in Swift, from `json`.
    fn read<T>(name: &'static str, read_as: &str, json: &str) -> Self
    where
        T: DeserializeOwned + Serialize,
    {
        Self {
            name,
            serde: written(&serde_json::from_str::<T>(json).unwrap()),
            swift: format!("read({read_as}.self, #\"{json}\"#)"),
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

/// Each holder read from a payload that holds the flattened member, then each built with the
/// flattened `Option` absent, which the Swift read takes from no payload that lacks its keys.
fn rows() -> Vec<Row> {
    let mut rows = vec![
        Row::read::<Lapping>("lapping", "Lapping", LAPPING),
        Row::read::<Flaking>("flaking", "Flaking", FLAKING),
        Row::read::<Sennit>("sennit", "Sennit", SENNIT),
        Row::read::<Crowning>("crowning", "Crowning", LAPPING),
        Row::built(
            "lapping absent",
            r#"Lapping(note: "n", tabling: nil)"#,
            &Lapping {
                tabling: None,
                note: "n".to_owned(),
            },
        ),
        Row::built(
            "flaking absent",
            "Flaking(tabling: nil)",
            &Flaking { tabling: None },
        ),
        Row::built(
            "sennit absent",
            r#"Sennit.laid(SennitLaidPayload(tabling: nil, note: "n"))"#,
            &Sennit::Laid {
                tabling: None,
                note: "n".to_owned(),
            },
        ),
    ];
    #[cfg(feature = "chrono")]
    rows.extend([
        Row::read::<Swaging>("swaging", "Swaging", SWAGING),
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
        crowning_swift::swift_definition(),
    ];
    #[cfg(feature = "chrono")]
    definitions.push(swaging_swift::swift_definition());
    let main = rows.iter().fold(String::new(), |mut main, row| {
        let _ = writeln!(main, "print(\"{}: \\(written({}))\")", row.name, row.swift);
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
/// its keys beside its holder's own.
#[test]
fn a_flattened_option_writes_in_swift_what_serde_writes() {
    let rows = rows();
    let expected = rows.iter().fold(String::new(), |mut expected, row| {
        let _ = writeln!(expected, "{}: {}", row.name, row.serde);
        expected
    });
    let shared = "lapping: {\"label\":\"north\",\"note\":\"n\"}\n\
         flaking: {\"label\":\"north\"}\n\
         sennit: {\"kind\":\"Laid\",\"label\":\"north\",\"note\":\"n\"}\n\
         crowning: {\"label\":\"north\",\"note\":\"n\"}\n\
         lapping absent: {\"note\":\"n\"}\n\
         flaking absent: {}\n\
         sennit absent: {\"kind\":\"Laid\",\"note\":\"n\"}\n";
    #[cfg(feature = "chrono")]
    let dates = "swaging: {\"north\":\"2024-01-02T03:04:05.678Z\"}\n\
         swaging absent: {}\n";
    #[cfg(not(feature = "chrono"))]
    let dates = "";
    assert_eq!(expected, format!("{shared}{dates}"));
    let Some(printed) = printed(&rows) else {
        return;
    };
    assert_eq!(printed, expected);
}
