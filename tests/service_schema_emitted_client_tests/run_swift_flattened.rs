//! A map flattened after a struct, beside a member of its holder's own and in a variant under a
//! tag key, a map flattened ahead of a struct, two flattened maps, a map flattened after each
//! member that takes none of the keys it reads, and a map under a key Swift wraps, in a type that
//! declares a parameter too, of models, of dates and one that names a function, and, after a
//! struct, a struct with a flattened map, a member that names a function, a model with a rule and
//! a struct that reads the same key, each read and written back by the emitted Swift types
//! compiled with warnings as errors, against what serde reads and writes back.

#![cfg(feature = "swift")]

use alloc::collections::BTreeMap;

use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::Value;

use super::runtime::judged;
use super::swift_flattened_models::{
    BOWSING, Bowsing, CLEWING, Clewing, FAKING, FRAPPING, Faking, Frapping, GRIPPING, Gripping,
    Kedging, Marling, Mousing, Notch, PARCELING, Parceling, Pennant, Plying, Quarter, STOPPING,
    Snaking, Splaying, Stopping, TEXTS, Toggling, Twinning, WARPING, WORMING, Warping, Worming,
    bowsing_swift, clewing_swift, counting_swift, faking_swift, footing_swift, frapping_swift,
    gripping_swift, hanking_swift, kedging_swift, marling_swift, mousing_swift, notch_swift,
    parceling_swift, pennant_swift, plying_swift, quarter_swift, reeving_swift, snaking_swift,
    splaying_swift, stopping_swift, toggling_swift, twinning_swift, warping_swift, worming_swift,
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

/// What prints each map read, then its holder written back, what a read of a model that breaks
/// its rule says, what a value whose two maps hold one key apart is written as and read back as,
/// then each member flattened after a struct, and what a read of a struct after a struct that
/// reads the same key says.
const MAIN: &str = r##"
let stopping = read(Stopping.self, #"STOPPING"#)
print("stopping: \(written(stopping.rest)) \(written(stopping))")
let frapping = read(Frapping.self, #"FRAPPING"#)
print("frapping: \(written(frapping.rest)) \(written(frapping))")
let kedging = read(Kedging.self, #"TEXTS"#)
print("kedging: \(written(kedging.extra)) \(written(kedging))")
let warping = read(Warping.self, #"WARPING"#)
print("warping: \(written(warping.first)) \(written(warping.second)) \(written(warping))")
let marling = read(Marling.self, #"TEXTS"#)
print("marling: \(written(marling.rest)) \(written(marling))")
let mousing = read(Mousing.self, #"TEXTS"#)
print("mousing: \(written(mousing.rest)) \(written(mousing))")
let bowsing = read(Bowsing.self, #"BOWSING"#)
print("bowsing: \(written(bowsing.rest)) \(written(bowsing))")
let clewing = read(Clewing.self, #"CLEWING"#)
if case .held(let held) = clewing { print("clewing: \(written(held.rest)) \(written(clewing))") }
let parceling = read(Parceling.self, #"PARCELING"#)
print("parceling: \(written(ParcelingRestKeyedMap(storage: parceling.rest))) \(written(parceling))")
let toggling = read(Toggling<UInt8>.self, #"PARCELING"#)
print("toggling: \(written(TogglingRestKeyedMap(storage: toggling.rest))) \(written(toggling))")
let worming = read(Worming.self, #"WORMING"#)
print("worming: \(written(worming.rest)) \(written(worming))")
let faking = read(Faking.self, #"FAKING"#)
print("faking: \(written(FakingRestKeyedMap(storage: faking.rest))) \(written(faking))")
let snaking = read(Snaking.self, #"TEXTS"#)
print("snaking: \(written(snaking.rest)) \(written(snaking))")
print("worming refused: \(refusal(Worming.self, #"BROKEN_WORMING"#))")
print("faking refused: \(refusal(Faking.self, #"BROKEN_FAKING"#))")
let apart = Warping(first: ["north": 1], note: "a", second: ["north": 2, "south": 3])
let back = read(Warping.self, written(apart))
print("apart: \(written(apart)) \(written(back.first)) \(written(back.second))")
let splaying = read(Splaying.self, #"BOWSING"#)
print("splaying: \(written(splaying.footing.rest)) \(written(splaying))")
let twinning = read(Twinning.self, #"STOPPING"#)
print("twinning: \(written(twinning.tally.keys)) \(written(twinning))")
let gripping = read(Gripping.self, #"GRIPPING"#)
print("gripping: \(written(gripping.notch)) \(written(gripping))")
print("gripping refused: \(refusal(Gripping.self, #"BROKEN_GRIPPING"#))")
print("plying refused: \(refusal(Plying.self, #"STOPPING"#))")
"##;

/// What prints a map of dates read, each date as its milliseconds, then its holder written back.
#[cfg(feature = "chrono")]
const DATED: &str = r##"
let serving = read(Serving.self, #"SERVING"#)
print("serving: \(written(serving.rest.mapValues { Int64(($0.timeIntervalSince1970 * 1000).rounded()) })) \(written(serving))")
"##;

/// No map of dates is declared without `chrono`.
#[cfg(not(feature = "chrono"))]
const DATED: &str = "";

/// A [`Worming`] whose model breaks its rule.
const BROKEN_WORMING: &str = r#"{"label":"n","north":{"mark":""},"note":"a"}"#;

/// A [`Faking`] whose model breaks its rule.
const BROKEN_FAKING: &str = r#"{"North":{"mark":""},"label":"n","note":"a"}"#;

/// A [`Gripping`] whose model breaks its rule.
const BROKEN_GRIPPING: &str = r#"{"label":"n","mark":"","note":"a"}"#;

/// `text` read as `T`.
fn read<T>(text: &str) -> T
where
    T: DeserializeOwned,
{
    serde_json::from_str(text).unwrap()
}

/// The text of `value` as serde writes it, its keys sorted and each written once with its last
/// value: serde writes a key two members hold twice, and a Swift encoder holds it once.
fn written<T>(value: &T) -> String
where
    T: Serialize,
{
    let text = serde_json::to_string(value).unwrap();
    let keyed: BTreeMap<String, Value> = serde_json::from_str(&text).unwrap();
    serde_json::to_string(&keyed).unwrap()
}

/// The pennant each broken holder flattens ahead of its map.
fn pennant() -> Pennant {
    Pennant {
        label: "n".to_owned(),
    }
}

/// A model that breaks its rule.
fn broken_notch() -> Notch {
    Notch {
        mark: String::new(),
    }
}

/// `main`, then [`DATED`], with each payload written in.
fn with_payloads(main: &str) -> String {
    let mut payloads = vec![
        ("BROKEN_WORMING", BROKEN_WORMING),
        ("BROKEN_FAKING", BROKEN_FAKING),
        ("BROKEN_GRIPPING", BROKEN_GRIPPING),
        ("STOPPING", STOPPING),
        ("FRAPPING", FRAPPING),
        ("TEXTS", TEXTS),
        ("WARPING", WARPING),
        ("BOWSING", BOWSING),
        ("CLEWING", CLEWING),
        ("PARCELING", PARCELING),
        ("WORMING", WORMING),
        ("FAKING", FAKING),
        ("GRIPPING", GRIPPING),
    ];
    #[cfg(feature = "chrono")]
    payloads.push(("SERVING", SERVING));
    payloads
        .into_iter()
        .fold(format!("{main}{DATED}"), |held, (named, payload)| {
            held.replace(named, payload)
        })
}

/// What `swift -warnings-as-errors` prints of [`MAIN`] over the emitted definitions, or `None`
/// where no toolchain is reachable.
fn printed() -> Option<String> {
    let mut definitions = vec![
        quarter_swift::swift_definition(),
        pennant_swift::swift_definition(),
        notch_swift::swift_definition(),
        reeving_swift::swift_definition(),
        hanking_swift::swift_definition(),
        stopping_swift::swift_definition(),
        frapping_swift::swift_definition(),
        kedging_swift::swift_definition(),
        warping_swift::swift_definition(),
        marling_swift::swift_definition(),
        mousing_swift::swift_definition(),
        bowsing_swift::swift_definition(),
        clewing_swift::swift_definition(),
        parceling_swift::swift_definition(),
        toggling_swift::swift_definition(),
        worming_swift::swift_definition(),
        faking_swift::swift_definition(),
        snaking_swift::swift_definition(),
        footing_swift::swift_definition(),
        splaying_swift::swift_definition(),
        counting_swift::swift_definition(),
        twinning_swift::swift_definition(),
        gripping_swift::swift_definition(),
        plying_swift::swift_definition(),
    ];
    #[cfg(feature = "chrono")]
    definitions.push(serving_swift::swift_definition());
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

/// What serde reads of a map of dates, each date as its milliseconds, and writes back.
#[cfg(feature = "chrono")]
fn dated() -> String {
    let serving: Serving = read(SERVING);
    let millis: BTreeMap<&String, i64> = serving
        .rest
        .iter()
        .map(|(key, at)| (key, at.timestamp_millis()))
        .collect();
    let line = format!("serving: {} {}\n", written(&millis), written(&serving));
    assert_eq!(
        line,
        "serving: {\"north\":1704164645678} \
         {\"label\":\"n\",\"north\":\"2024-01-02T03:04:05.678Z\",\"note\":\"a\"}\n"
    );
    line
}

#[cfg(not(feature = "chrono"))]
const fn dated() -> String {
    String::new()
}

/// What `validate()` answers, joined as a read joins it, of a [`Worming`] and a [`Faking`] whose
/// model breaks its rule, each written as the payload the run reads.
fn refusals() -> [String; 2] {
    let broken_worming = Worming {
        flag: pennant(),
        note: "a".to_owned(),
        rest: BTreeMap::from([("north".to_owned(), broken_notch())]),
    };
    let broken_faking = Faking {
        flag: pennant(),
        note: "a".to_owned(),
        rest: BTreeMap::from([(Quarter::North, broken_notch())]),
    };
    assert_eq!(written(&broken_worming), BROKEN_WORMING);
    assert_eq!(written(&broken_faking), BROKEN_FAKING);
    [
        broken_worming.validate().unwrap_err().join("; "),
        broken_faking.validate().unwrap_err().join("; "),
    ]
}

/// Each member flattened after a struct, read as serde reads it, then written back, what
/// `validate()` answers of a [`Gripping`] whose model breaks its rule, and what serde says of a
/// [`Plying`], ahead of the position it says it at.
fn after_a_struct() -> String {
    let splaying: Splaying = read(BOWSING);
    let twinning: Twinning = read(STOPPING);
    let gripping: Gripping = read(GRIPPING);
    let broken_gripping = Gripping {
        flag: pennant(),
        notch: broken_notch(),
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
        written(&splaying.footing.rest),
        written(&splaying),
        serde_json::to_string(&twinning.tally.keys).unwrap(),
        written(&twinning),
        written(&gripping.notch),
        written(&gripping),
        broken_gripping.validate().unwrap_err().join("; "),
    )
}

/// A [`Warping`] whose two maps hold one key apart, written, then the two maps it reads back as.
/// serde writes the key twice, and each map reads it back with its last value.
fn apart() -> [String; 3] {
    let apart = Warping {
        first: BTreeMap::from([("north".to_owned(), 1)]),
        note: "a".to_owned(),
        second: BTreeMap::from([("north".to_owned(), 2), ("south".to_owned(), 3)]),
    };
    let apart_text = serde_json::to_string(&apart).unwrap();
    assert_eq!(apart_text, r#"{"north":1,"note":"a","north":2,"south":3}"#);
    let back: Warping = read(&apart_text);
    [written(&apart), written(&back.first), written(&back.second)]
}

/// Each map, and each member flattened after a struct, is read from the keys serde hands it, at
/// its place among the flattened members, and each holder is written back as serde writes it back,
/// a key two members hold once with its last value. A model a map or a holder holds is judged
/// once, with its holder, under the place serde names, and a struct after a struct that reads the
/// same key is refused, as serde refuses it. A map of dates is read from the text serde writes and
/// written back as that text.
#[test]
fn a_flattened_member_reads_in_swift_the_keys_serde_hands_it() {
    let stopping: Stopping = read(STOPPING);
    let frapping: Frapping = read(FRAPPING);
    let kedging: Kedging = read(TEXTS);
    let warping: Warping = read(WARPING);
    let marling: Marling = read(TEXTS);
    let mousing: Mousing = read(TEXTS);
    let bowsing: Bowsing = read(BOWSING);
    let clewing: Clewing = read(CLEWING);
    let Clewing::Held {
        flag,
        note,
        rest: held,
    } = &clewing;
    assert_eq!((flag.label.as_str(), note.as_str()), ("n", "a"));
    let parceling: Parceling = read(PARCELING);
    let toggling: Toggling<u8> = read(PARCELING);
    let worming: Worming = read(WORMING);
    let faking: Faking = read(FAKING);
    let snaking: Snaking = read(TEXTS);
    let [worming_refused, faking_refused] = refusals();
    let [apart_written, back_first, back_second] = apart();
    let after_a_struct = after_a_struct();
    let expected = format!(
        "stopping: {} {}\nfrapping: {} {}\nkedging: {} {}\nwarping: {} {} {}\n\
         marling: {} {}\nmousing: {} {}\nbowsing: {} {}\nclewing: {} {}\nparceling: {} {}\n\
         toggling: {} {}\nworming: {} {}\nfaking: {} {}\nsnaking: {} {}\n\
         worming refused: {}\nfaking refused: {}\napart: {} {} {}\n{after_a_struct}{}",
        written(&stopping.rest),
        written(&stopping),
        written(&frapping.rest),
        written(&frapping),
        written(&kedging.extra),
        written(&kedging),
        written(&warping.first),
        written(&warping.second),
        written(&warping),
        written(&marling.rest),
        written(&marling),
        written(&mousing.rest),
        written(&mousing),
        written(&bowsing.rest),
        written(&bowsing),
        written(held),
        written(&clewing),
        written(&parceling.rest),
        written(&parceling),
        written(&toggling.rest),
        written(&toggling),
        written(&worming.rest),
        written(&worming),
        written(&faking.rest),
        written(&faking),
        written(&snaking.rest),
        written(&snaking),
        worming_refused,
        faking_refused,
        apart_written,
        back_first,
        back_second,
        dated(),
    );
    assert!(
        expected.starts_with(
            "stopping: {\"north\":2} {\"label\":\"n\",\"north\":2,\"note\":\"a\"}\n\
             frapping: {\"north\":2} {\"north\":2,\"note\":\"a\"}\n\
             kedging: {\"label\":\"n\",\"north\":\"s\"} {\"label\":\"n\",\"north\":\"s\",\"note\":\"a\"}\n\
             warping: {\"north\":2,\"south\":3} {\"north\":2,\"south\":3} {\"north\":2,\"note\":\"a\",\"south\":3}\n\
             marling: {\"label\":\"n\",\"north\":\"s\"} {\"label\":\"n\",\"north\":\"s\",\"note\":\"a\"}\n\
             mousing: {\"label\":\"n\",\"north\":\"s\"} {\"label\":\"n\",\"north\":\"s\",\"note\":\"a\"}\n\
             bowsing: {\"label\":\"n\",\"marker\":\"m\",\"north\":\"s\"} {\"label\":\"n\",\"marker\":\"m\",\"north\":\"s\",\"note\":\"a\"}\n\
             clewing: {\"north\":2} {\"kind\":\"Held\",\"label\":\"n\",\"north\":2,\"note\":\"a\"}\n\
             parceling: {\"North\":2} {\"North\":2,\"label\":\"n\",\"note\":\"a\"}\n\
             toggling: {\"North\":2} {\"North\":2,\"label\":\"n\",\"note\":\"a\"}\n\
             worming: {\"north\":{\"mark\":\"x\"}} {\"label\":\"n\",\"north\":{\"mark\":\"x\"},\"note\":\"a\"}\n\
             faking: {\"North\":{\"mark\":\"x\"}} {\"North\":{\"mark\":\"x\"},\"label\":\"n\",\"note\":\"a\"}\n\
             snaking: {\"north\":\"s\"} {\"label\":\"n\",\"north\":\"s\",\"note\":\"a\"}\n\
             worming refused: 'north.mark': too short: minimum length is 1, got 0\n\
             faking refused: 'North.mark': too short: minimum length is 1, got 0\n\
             apart: {\"north\":2,\"note\":\"a\",\"south\":3} {\"north\":2,\"south\":3} {\"north\":2,\"south\":3}\n\
             splaying: {\"north\":\"s\"} {\"label\":\"n\",\"marker\":\"m\",\"north\":\"s\",\"note\":\"a\"}\n\
             twinning: [\"north\"] {\"keys\":[\"north\"],\"label\":\"n\",\"note\":\"a\"}\n\
             gripping: {\"mark\":\"x\"} {\"label\":\"n\",\"mark\":\"x\",\"note\":\"a\"}\n\
             gripping refused: 'mark': too short: minimum length is 1, got 0\n\
             plying refused: missing field `label`\n"
        ),
        "{expected}"
    );
    let Some(printed) = printed() else {
        return;
    };
    assert_eq!(printed, expected);
}
