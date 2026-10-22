//! A member that names a function, beside a member of its holder's own, ahead of a struct and
//! after each member that takes none of the keys it reads, and, after a struct, a struct with a
//! flattened member of its own, a member that names a function, a model with a rule and a struct
//! that reads the same key, and a struct flattened first, plain and naming a function, beside a
//! member of its holder's own, each read and written back by the emitted Swift types compiled
//! with warnings as errors, against what serde reads and writes back. Every holder has a member of
//! its own: the read and the write of a struct whose every member is flattened declare a container
//! they do not use, which `-warnings-as-errors` refuses, so a map, the one member of its struct,
//! and a holder of no member of its own are pinned by their text alone in
//! `tests/swift_tests/flattened.rs`.

#![cfg(feature = "swift")]

use alloc::collections::BTreeMap;

use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::Value;

use super::runtime::judged;
use super::swift_flattened_models::{
    BOWSING, Bowsing, FRAPPING, Frapping, GRIPPING, Gripping, HANKING, Hanking, Kedging, LABEL,
    Marling, Marrying, Mousing, Notch, Pennant, Plying, STOPPING, Splaying, TEXTS, Twinning,
    Whipping, bowsing_swift, counting_swift, footing_swift, frapping_swift, gripping_swift,
    hanking_swift, kedging_swift, marling_swift, marrying_swift, mousing_swift, notch_swift,
    pennant_swift, plying_swift, reeving_swift, splaying_swift, twinning_swift, whipping_swift,
};

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
    let source = [
        "import Foundation".to_owned(),
        definitions.join("\n\n"),
        HELPERS.to_owned(),
        with_payloads(MAIN),
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
