//! A member that names a function, beside a member of its holder's own, ahead of a struct and
//! after each member that takes none of the keys it reads, and, after a struct, a struct with a
//! flattened member of its own, a member that names a function, a model with a rule and a struct
//! that reads the same key, and a struct flattened first, plain and naming a function, beside a
//! member of its holder's own, each read and written back by the emitted Swift types compiled
//! with warnings as errors, against what serde reads and writes back; and a struct and a variant
//! whose every member is flattened: a map, the one member of each, in each shape a map is read in,
//! and a holder of flattened structs alone.

#![cfg(feature = "swift")]

use alloc::collections::BTreeMap;
use core::fmt::Write as _;

use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::Value;

use super::runtime::judged;
use super::swift_flattened_models::{
    BOWSING, Bowsing, CLEWING, COUNTS, Clewing, FAKING, FRAPPING, Faking, Frapping, GRIPPING,
    Gripping, HANKING, Hanking, Kedging, LABEL, Marling, Marrying, Mousing, Notch, PARCELING,
    Parceling, Pennant, Plying, SEAMING, SNAKING, STOPPING, Seaming, Snaking, Splaying, Stopping,
    Stropping, TEXTS, Toggling, Twinning, WORMING, Whipping, Worming, bowsing_swift, clewing_swift,
    counting_swift, faking_swift, footing_swift, frapping_swift, gripping_swift, hanking_swift,
    kedging_swift, marling_swift, marrying_swift, mousing_swift, notch_swift, parceling_swift,
    pennant_swift, plying_swift, quarter_swift, reeving_swift, seaming_swift, snaking_swift,
    splaying_swift, stopping_swift, stropping_swift, toggling_swift, twinning_swift,
    whipping_swift, worming_swift,
};
#[cfg(feature = "chrono")]
use super::swift_flattened_models::{SERVING, Serving, serving_swift};

/// Names the runtime to run, for a machine that has one somewhere other than `PATH`.
const RUNTIME_VAR: &str = "TIXSCHEMA_SWIFT";

/// What the run declares beside the emitted types: the functions the models name, one answering
/// the payload as it was handed and one the keys of the object it is handed, the text of a value
/// written with its keys sorted, the read of a payload, and what a read that refuses one says, a
/// key it misses in serde's words.
const HELPERS: &str = r#"
func asGiven(_ payload: Any) -> Any { payload }

func withItsKeys(_ payload: Any) -> Any {
  guard let object = payload as? [String: Any] else { return payload }
  return ["keys": object.keys.sorted()]
}

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
    return "read"
  } catch DecodingError.dataCorrupted(let context) {
    return context.debugDescription
  } catch DecodingError.keyNotFound(let key, _) {
    return "missing field `\(key.stringValue)`"
  } catch {
    return "\(error)"
  }
}
"#;

/// What prints the keys each function was handed, then its holder written back; then each member
/// flattened after a struct, and what a read of a struct after a struct that reads the same key
/// says; then a struct flattened first beside a member of its holder's own, and what a read of
/// one that reads the key of that member says.
const MAIN: &str = r##"
let frapping = read(Frapping.self, #"FRAPPING"#)
print("frapping: \(written(frapping.tally.keys)) \(written(frapping))")
let kedging = read(Kedging.self, #"TEXTS"#)
print("kedging: \(written(kedging.extra.keys)) \(written(kedging))")
let marling = read(Marling.self, #"TEXTS"#)
print("marling: \(written(marling.tally.keys)) \(written(marling))")
let mousing = read(Mousing.self, #"TEXTS"#)
print("mousing: \(written(mousing.tally.keys)) \(written(mousing))")
let bowsing = read(Bowsing.self, #"BOWSING"#)
print("bowsing: \(written(bowsing.tally.keys)) \(written(bowsing))")
let splaying = read(Splaying.self, #"BOWSING"#)
print("splaying: \(written(splaying.footing.tally.keys)) \(written(splaying))")
let twinning = read(Twinning.self, #"STOPPING"#)
print("twinning: \(written(twinning.tally.keys)) \(written(twinning))")
let gripping = read(Gripping.self, #"GRIPPING"#)
print("gripping: \(written(gripping.notch)) \(written(gripping))")
print("gripping refused: \(refusal(Gripping.self, #"BROKEN_GRIPPING"#))")
print("plying refused: \(refusal(Plying.self, #"STOPPING"#))")
print("hanking: \(written(read(Hanking.self, #"HANKING"#)))")
print("whipping refused: \(refusal(Whipping.self, #"LABEL"#))")
print("marrying refused: \(refusal(Marrying.self, #"LABEL"#))")
"##;

/// A [`Gripping`] whose model breaks its rule.
const BROKEN_GRIPPING: &str = r#"{"label":"n","mark":"","note":"a"}"#;

/// A [`Worming`] whose model breaks its rule.
const BROKEN_WORMING: &str = r#"{"north":{"mark":""}}"#;

/// A [`Seaming`] whose model breaks its rule.
const BROKEN_SEAMING: &str = r#"{"label":"n","mark":""}"#;

/// A payload of a type whose every member is flattened, the type it is read as, and what serde
/// tells of it: the value read, written back, or what the read that refuses it says.
struct Row {
    json: &'static str,
    name: &'static str,
    read_as: &'static str,
    refused: bool,
    told: fn(&str) -> String,
}

/// What serde writes back of `json`, read as `T`, its keys sorted.
fn written_back<T>(json: &str) -> String
where
    T: DeserializeOwned + Serialize,
{
    written(&read::<T>(json))
}

/// What serde says of `json`, read as `T`, ahead of any position it says it at.
fn refused<T>(json: &str) -> String
where
    T: DeserializeOwned,
{
    let refusal = serde_json::from_str::<T>(json).err().unwrap().to_string();
    refusal.split_once(" at line").map_or_else(
        || refusal.clone(),
        |(refused, _position)| refused.to_owned(),
    )
}

/// Each type whose every member is flattened, read from its payload: each map alone, the struct
/// that names a function among them, a holder of flattened structs alone, and two whose model
/// breaks its rule.
fn alone_rows() -> Vec<Row> {
    let row = |name, read_as, json, told| Row {
        json,
        name,
        read_as,
        refused: false,
        told,
    };
    let mut rows = vec![
        row("stopping", "Stopping", COUNTS, written_back::<Stopping>),
        row("clewing", "Clewing", CLEWING, written_back::<Clewing>),
        row(
            "parceling",
            "Parceling",
            PARCELING,
            written_back::<Parceling>,
        ),
        row(
            "toggling",
            "Toggling<UInt8>",
            PARCELING,
            written_back::<Toggling<u8>>,
        ),
        row("worming", "Worming", WORMING, written_back::<Worming>),
        row("faking", "Faking", FAKING, written_back::<Faking>),
        row("snaking", "Snaking", SNAKING, written_back::<Snaking>),
        row(
            "stropping",
            "Stropping",
            STOPPING,
            written_back::<Stropping>,
        ),
        row("seaming", "Seaming", SEAMING, written_back::<Seaming>),
        Row {
            refused: true,
            ..row(
                "worming refused",
                "Worming",
                BROKEN_WORMING,
                refused::<Worming>,
            )
        },
        Row {
            refused: true,
            ..row(
                "seaming refused",
                "Seaming",
                BROKEN_SEAMING,
                refused::<Seaming>,
            )
        },
    ];
    #[cfg(feature = "chrono")]
    rows.push(row("serving", "Serving", SERVING, written_back::<Serving>));
    rows
}

/// `text` read as `T`.
fn read<T>(text: &str) -> T
where
    T: DeserializeOwned,
{
    serde_json::from_str(text).unwrap()
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

/// The keys a function was handed, as serde writes them.
fn keys(handed: &[String]) -> String {
    serde_json::to_string(handed).unwrap()
}

/// `main` with each payload written in.
fn with_payloads(main: &str) -> String {
    [
        ("BROKEN_GRIPPING", BROKEN_GRIPPING),
        ("STOPPING", STOPPING),
        ("FRAPPING", FRAPPING),
        ("TEXTS", TEXTS),
        ("BOWSING", BOWSING),
        ("GRIPPING", GRIPPING),
        ("HANKING", HANKING),
        ("LABEL", LABEL),
    ]
    .into_iter()
    .fold(main.to_owned(), |held, (named, payload)| {
        held.replace(named, payload)
    })
}

/// What `swift -warnings-as-errors` prints of `main` over `definitions` and [`HELPERS`], or `None`
/// where no toolchain is reachable.
fn printed_of(definitions: &[String], main: &str) -> Option<String> {
    let source = [
        "import Foundation".to_owned(),
        definitions.join("\n\n"),
        HELPERS.to_owned(),
        main.to_owned(),
    ]
    .join("\n\n");
    let run = judged(
        "swift-flattened",
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

/// What `swift -warnings-as-errors` prints of [`MAIN`] over the emitted definitions, or `None`
/// where no toolchain is reachable.
fn printed() -> Option<String> {
    let definitions = [
        pennant_swift::swift_definition(),
        notch_swift::swift_definition(),
        counting_swift::swift_definition(),
        frapping_swift::swift_definition(),
        kedging_swift::swift_definition(),
        reeving_swift::swift_definition(),
        hanking_swift::swift_definition(),
        marling_swift::swift_definition(),
        mousing_swift::swift_definition(),
        bowsing_swift::swift_definition(),
        footing_swift::swift_definition(),
        splaying_swift::swift_definition(),
        twinning_swift::swift_definition(),
        gripping_swift::swift_definition(),
        plying_swift::swift_definition(),
        whipping_swift::swift_definition(),
        marrying_swift::swift_definition(),
    ];
    printed_of(&definitions, &with_payloads(MAIN))
}

/// What `swift -warnings-as-errors` prints of `rows`, each read as its type over the emitted
/// definitions of the types whose every member is flattened, or `None` where no toolchain is
/// reachable.
fn printed_alone(rows: &[Row]) -> Option<String> {
    let mut definitions = vec![
        pennant_swift::swift_definition(),
        notch_swift::swift_definition(),
        counting_swift::swift_definition(),
        quarter_swift::swift_definition(),
        stopping_swift::swift_definition(),
        clewing_swift::swift_definition(),
        parceling_swift::swift_definition(),
        toggling_swift::swift_definition(),
        worming_swift::swift_definition(),
        faking_swift::swift_definition(),
        snaking_swift::swift_definition(),
        stropping_swift::swift_definition(),
        seaming_swift::swift_definition(),
    ];
    #[cfg(feature = "chrono")]
    definitions.push(serving_swift::swift_definition());
    let main = rows.iter().fold(String::new(), |mut main, row| {
        let payload = format!("({}.self, #\"{}\"#)", row.read_as, row.json);
        let told = if row.refused {
            format!("refusal{payload}")
        } else {
            format!("written(read{payload})")
        };
        let _ = writeln!(main, "print(\"{}: \\({told})\")", row.name);
        main
    });
    printed_of(&definitions, &main)
}

/// The keys each function was handed, as serde hands them, then its holder written back.
fn handed() -> String {
    let frapping: Frapping = read(FRAPPING);
    let kedging: Kedging = read(TEXTS);
    let marling: Marling = read(TEXTS);
    let mousing: Mousing = read(TEXTS);
    let bowsing: Bowsing = read(BOWSING);
    format!(
        "frapping: {} {}\nkedging: {} {}\nmarling: {} {}\nmousing: {} {}\nbowsing: {} {}\n",
        keys(&frapping.tally.keys),
        written(&frapping),
        keys(&kedging.extra.keys),
        written(&kedging),
        keys(&marling.tally.keys),
        written(&marling),
        keys(&mousing.tally.keys),
        written(&mousing),
        keys(&bowsing.tally.keys),
        written(&bowsing),
    )
}

/// Each member flattened after a struct, read as serde reads it, then written back, what
/// `validate()` answers of a [`Gripping`] whose model breaks its rule, and what serde says of a
/// [`Plying`], ahead of the position it says it at.
fn after_a_struct() -> String {
    let splaying: Splaying = read(BOWSING);
    let twinning: Twinning = read(STOPPING);
    let gripping: Gripping = read(GRIPPING);
    let broken_gripping = Gripping {
        flag: Pennant {
            label: "n".to_owned(),
        },
        notch: Notch {
            mark: String::new(),
        },
        note: "a".to_owned(),
    };
    assert_eq!(written(&broken_gripping), BROKEN_GRIPPING);
    let plying = serde_json::from_str::<Plying>(STOPPING)
        .unwrap_err()
        .to_string();
    let (plying_refused, _position) = plying.split_once(" at line").unwrap();
    format!(
        "splaying: {} {}\ntwinning: {} {}\ngripping: {} {}\ngripping refused: {}\n\
         plying refused: {plying_refused}\n",
        keys(&splaying.footing.tally.keys),
        written(&splaying),
        keys(&twinning.tally.keys),
        written(&twinning),
        written(&gripping.notch),
        written(&gripping),
        broken_gripping.validate().unwrap_err().join("; "),
    )
}

/// A struct flattened first beside a member of its holder's own whose key it does not read, read
/// and written back, then what serde says of a struct flattened first, plain and naming a
/// function, beside a member that reads its key, ahead of the position it says it at.
fn first() -> String {
    let refused = |text: String| {
        let (refused, _position) = text.split_once(" at line").unwrap();
        refused.to_owned()
    };
    let whipping = serde_json::from_str::<Whipping>(LABEL).unwrap_err();
    let marrying = serde_json::from_str::<Marrying>(LABEL).unwrap_err();
    format!(
        "hanking: {}\nwhipping refused: {}\nmarrying refused: {}\n",
        written(&read::<Hanking>(HANKING)),
        refused(whipping.to_string()),
        refused(marrying.to_string()),
    )
}

/// Each member that names a function, each member flattened after a struct and each struct
/// flattened first is read from the keys serde hands it, and each holder is written back as serde
/// writes it back. A model a holder holds is judged once, with its holder, under the place serde
/// names, and a struct that reads a key its holder or a struct ahead of it took is refused, as
/// serde refuses it.
#[test]
fn a_flattened_member_reads_in_swift_the_keys_serde_hands_it() {
    let expected = format!("{}{}{}", handed(), after_a_struct(), first());
    assert_eq!(
        expected,
        "frapping: [\"north\"] {\"keys\":[\"north\"],\"note\":\"a\"}\n\
         kedging: [\"label\",\"north\"] {\"keys\":[\"label\",\"north\"],\"label\":\"n\",\"note\":\"a\"}\n\
         marling: [\"label\",\"north\"] {\"keys\":[\"label\",\"north\"],\"label\":\"n\",\"note\":\"a\"}\n\
         mousing: [\"label\",\"north\"] {\"keys\":[\"label\",\"north\"],\"label\":\"n\",\"note\":\"a\"}\n\
         bowsing: [\"label\",\"marker\",\"north\"] \
         {\"keys\":[\"label\",\"marker\",\"north\"],\"label\":\"n\",\"marker\":\"m\",\"note\":\"a\"}\n\
         splaying: [\"north\"] {\"keys\":[\"north\"],\"label\":\"n\",\"marker\":\"m\",\"note\":\"a\"}\n\
         twinning: [\"north\"] {\"keys\":[\"north\"],\"label\":\"n\",\"note\":\"a\"}\n\
         gripping: {\"mark\":\"x\"} {\"label\":\"n\",\"mark\":\"x\",\"note\":\"a\"}\n\
         gripping refused: 'mark': too short: minimum length is 1, got 0\n\
         plying refused: missing field `label`\n\
         hanking: {\"label\":\"n\",\"marker\":\"m\"}\n\
         whipping refused: missing field `label`\n\
         marrying refused: missing field `label`\n"
    );
    let Some(printed) = printed() else {
        return;
    };
    assert_eq!(printed, expected);
}

/// Each struct and variant whose every member is flattened, a map alone in each shape a map is
/// read in and flattened structs alone, is read from its payload and written back as serde writes
/// it back, and one whose model breaks its rule is refused as serde refuses it.
#[test]
fn a_type_of_flattened_members_alone_reads_and_writes_in_swift_as_serde_does() {
    let rows = alone_rows();
    let expected = rows.iter().fold(String::new(), |mut expected, row| {
        let _ = writeln!(expected, "{}: {}", row.name, (row.told)(row.json));
        expected
    });
    let shared = "stopping: {\"north\":2,\"south\":3}\n\
         clewing: {\"kind\":\"Held\",\"north\":2}\n\
         parceling: {\"North\":2}\n\
         toggling: {\"North\":2}\n\
         worming: {\"north\":{\"mark\":\"x\"}}\n\
         faking: {\"North\":{\"mark\":\"x\"}}\n\
         snaking: {\"north\":\"s\"}\n\
         stropping: {\"keys\":[\"north\",\"note\"],\"label\":\"n\"}\n\
         seaming: {\"label\":\"n\",\"mark\":\"x\"}\n\
         worming refused: 'north.mark': too short: minimum length is 1, got 0\n\
         seaming refused: 'mark': too short: minimum length is 1, got 0\n";
    #[cfg(feature = "chrono")]
    let dates = "serving: {\"north\":\"2024-01-02T03:04:05.678Z\"}\n";
    #[cfg(not(feature = "chrono"))]
    let dates = "";
    assert_eq!(expected, format!("{shared}{dates}"));
    let Some(printed) = printed_alone(&rows) else {
        return;
    };
    assert_eq!(printed, expected);
}
