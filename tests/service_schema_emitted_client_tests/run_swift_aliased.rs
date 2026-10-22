//! A name serde reads beside the one it writes, read by the emitted Swift types compiled with
//! warnings as errors: a variant under its `alias`, under each tagging, flattened and in a plain
//! enum, and a member under its `alias`, in a struct, flattened, in a variant and in each way the
//! Swift read reads a member under its key. Each payload is written back as serde writes it back,
//! under the name serde writes, and a member read under two of its names is refused, as serde
//! refuses it.

#![cfg(all(feature = "swift", feature = "serde"))]

use core::fmt::Write as _;

use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::Value;

use super::aliased_members_models::{THOLING, Tholing, spud_swift, tholing_swift, tideway_swift};
use super::aliased_models::{
    CAREENING, Careening, FAIRLEADING, Fairleading, Fothering, Gybing, HAWSING, Hawsing, LULLING,
    Lulling, SCENDING, SURGING, SWELL, Scending, Surging, Swell, TIDING, Tiding, VEERING, Veering,
    careening_swift, fairleading_swift, fothering_swift, gybing_swift, hawsing_swift,
    lulling_swift, pennon_swift, scending_swift, surging_swift, swell_swift, tiding_swift,
    veering_swift,
};
use super::dart_flattened_aliased_models::{
    DRIFTING, Drifting, RIDING, Riding, SWINGING, Swinging, drifting_swift, riding_swift,
    swinging_swift,
};
use super::runtime::judged;

/// Names the runtime to run, for a machine that has one somewhere other than `PATH`.
const RUNTIME_VAR: &str = "TIXSCHEMA_SWIFT";

/// What the run declares beside the emitted types: the functions the models name, and what prints
/// a row, the value read and written back with its keys sorted, or that the read is refused, and
/// what says why: the description of a corrupted payload, the refusal of a member under two of its
/// names and of a value `validate()` refuses among them.
const HELPERS: &str = r#"
func unpadded(_ payload: Any) -> Any {
  guard let text = payload as? String else { return payload }
  return text.trimmingCharacters(in: .whitespaces)
}

func withItsKeys(_ payload: Any) -> Any {
  guard let object = payload as? [String: Any] else { return payload }
  return ["keys": object.keys.sorted()]
}

func writtenBack<Read: Codable>(_ type: Read.Type, _ payload: String) throws -> String {
  let encoder = JSONEncoder()
  encoder.outputFormatting = [.sortedKeys]
  let read = try JSONDecoder().decode(type, from: Data(payload.utf8))
  return String(data: try encoder.encode(read), encoding: .utf8)!
}

func row<Read: Codable>(_ name: String, _ type: Read.Type, _ payload: String) {
  let told = (try? writtenBack(type, payload)) ?? "refused"
  print("\(name) \(payload): \(told)")
}

func said<Read: Codable>(_ name: String, _ type: Read.Type, _ payload: String) {
  let told: String
  do {
    told = try writtenBack(type, payload)
  } catch DecodingError.dataCorrupted(let context) {
    told = context.debugDescription
  } catch {
    told = "\(error)"
  }
  print("\(name) \(payload): \(told)")
}
"#;

/// The payloads the Swift read refuses where serde reads them, as TypeScript and the generated Zod
/// schema refuse them: a unit variant's entry over `null`, which the enum read alone takes as the
/// variant's name alone.
const REFUSED_AS_ZOD_REFUSES: [(&str, &str); 1] = [("Surging", r#"{"Slack":null}"#)];

/// What the run prints of each payload: what serde writes back of it, the rows the Swift read
/// refuses as TypeScript and Zod refuse them apart, then why serde refuses a member under two of
/// its names and each payload of [`Tholing`] it refuses.
const PRINTED: &str = "Surging {\"Berth\":{\"label\":\"n\"}}: {\"Berth\":{\"label\":\"n\"}}\n\
     Surging {\"Pier\":{\"label\":\"n\"}}: {\"Berth\":{\"label\":\"n\"}}\n\
     Surging {\"Pier\":{\"name\":\"n\"}}: {\"Berth\":{\"label\":\"n\"}}\n\
     Surging {\"Berth\":{\"label\":\"n\",\"name\":\"m\"}}: refused\n\
     Surging \"Calm\": \"Calm\"\n\
     Surging \"Still\": \"Calm\"\n\
     Surging \"Slack\": \"Calm\"\n\
     Surging {\"Slack\":null}: refused\n\
     Surging {\"Held\":\"x\"}: {\"Held\":\"x\"}\n\
     Surging {\"Tied\":\"x\"}: {\"Held\":\"x\"}\n\
     Careening {\"kind\":\"Berth\",\"label\":\"n\"}: {\"kind\":\"Berth\",\"label\":\"n\"}\n\
     Careening {\"kind\":\"Pier\",\"label\":\"n\"}: {\"kind\":\"Berth\",\"label\":\"n\"}\n\
     Careening {\"kind\":\"Pier\",\"name\":\"n\"}: {\"kind\":\"Berth\",\"label\":\"n\"}\n\
     Careening {\"kind\":\"Berth\",\"label\":\"n\",\"name\":\"m\"}: refused\n\
     Careening {\"kind\":\"Calm\"}: {\"kind\":\"Calm\"}\n\
     Careening {\"kind\":\"Still\"}: {\"kind\":\"Calm\"}\n\
     Scending {\"kind\":\"Berth\",\"data\":{\"label\":\"n\"}}: \
     {\"data\":{\"label\":\"n\"},\"kind\":\"Berth\"}\n\
     Scending {\"kind\":\"Pier\",\"data\":{\"label\":\"n\"}}: \
     {\"data\":{\"label\":\"n\"},\"kind\":\"Berth\"}\n\
     Scending {\"kind\":\"Pier\",\"data\":{\"name\":\"n\"}}: \
     {\"data\":{\"label\":\"n\"},\"kind\":\"Berth\"}\n\
     Scending {\"kind\":\"Calm\"}: {\"kind\":\"Calm\"}\n\
     Scending {\"kind\":\"Still\"}: {\"kind\":\"Calm\"}\n\
     Scending {\"kind\":\"Held\",\"data\":\"x\"}: {\"data\":\"x\",\"kind\":\"Held\"}\n\
     Scending {\"kind\":\"Tied\",\"data\":\"x\"}: {\"data\":\"x\",\"kind\":\"Held\"}\n\
     Swell \"Flood\": \"Flood\"\n\
     Swell \"Ebb\": \"Ebb\"\n\
     Swell \"Low\": \"Ebb\"\n\
     Swell \"Neap\": \"Ebb\"\n\
     Swell \"High\": refused\n\
     Lulling \" Ebb \": \"Ebb\"\n\
     Lulling \" Low \": \"Ebb\"\n\
     Lulling \"Flood\": \"Flood\"\n\
     Tiding {\"by_swell\":{\"Neap\":1,\"Flood\":2},\"swells\":[\"Low\",\"Flood\"]}: \
     {\"by_swell\":{\"Ebb\":1,\"Flood\":2},\"swells\":[\"Ebb\",\"Flood\"]}\n\
     Veering {\"label\":\"n\"}: {\"label\":\"n\"}\n\
     Veering {\"name\":\"n\"}: {\"label\":\"n\"}\n\
     Veering {\"label\":\"n\",\"name\":\"m\"}: refused\n\
     Veering 3: 3\n\
     Hawsing {\"label\":\"n\"}: {\"label\":\"n\"}\n\
     Hawsing {\"name\":\"n\"}: {\"label\":\"n\"}\n\
     Hawsing {\"title\":\"n\",\"remark\":\"r\"}: {\"label\":\"n\",\"note\":\"r\"}\n\
     Hawsing {\"label\":\"n\",\"name\":\"m\"}: refused\n\
     Hawsing {\"name\":\"n\",\"title\":\"m\"}: refused\n\
     Gybing<String> {\"label\":\"n\"}: {\"label\":\"n\"}\n\
     Gybing<String> {\"name\":\"n\"}: {\"label\":\"n\"}\n\
     Gybing<String> {\"title\":\"n\",\"remark\":\"r\"}: {\"label\":\"n\",\"note\":\"r\"}\n\
     Gybing<String> {\"label\":\"n\",\"name\":\"m\"}: refused\n\
     Gybing<String> {\"name\":\"n\",\"title\":\"m\"}: refused\n\
     Fairleading {\"name\":\"n\",\"north\":2}: {\"keys\":[\"north\"],\"label\":\"n\"}\n\
     Fairleading {\"label\":\"n\",\"north\":2}: {\"keys\":[\"north\"],\"label\":\"n\"}\n\
     Fairleading {\"label\":\"n\",\"name\":\"m\",\"north\":2}: refused\n\
     Fothering {\"name\":\"n\",\"north\":2}: {\"keys\":[\"north\"],\"label\":\"n\"}\n\
     Fothering {\"label\":\"n\",\"north\":2}: {\"keys\":[\"north\"],\"label\":\"n\"}\n\
     Fothering {\"label\":\"n\",\"name\":\"m\",\"north\":2}: refused\n\
     Riding {\"Pier\":{\"name\":\"n\"},\"note\":\"a\"}: \
     {\"Berth\":{\"label\":\"n\"},\"note\":\"a\"}\n\
     Riding {\"note\":\"a\",\"Tied\":\"x\"}: {\"Held\":\"x\",\"note\":\"a\"}\n\
     Riding {\"note\":\"a\",\"Slack\":null}: {\"Calm\":null,\"note\":\"a\"}\n\
     Riding {\"Berth\":{\"label\":\"n\",\"name\":\"m\"},\"note\":\"a\"}: refused\n\
     Swinging {\"kind\":\"Pier\",\"name\":\"n\",\"note\":\"a\"}: \
     {\"kind\":\"Berth\",\"label\":\"n\",\"note\":\"a\"}\n\
     Swinging {\"note\":\"a\",\"kind\":\"Still\"}: {\"kind\":\"Calm\",\"note\":\"a\"}\n\
     Swinging {\"kind\":\"Pier\",\"label\":\"n\",\"name\":\"m\",\"note\":\"a\"}: refused\n\
     Drifting {\"kind\":\"Pier\",\"data\":{\"name\":\"n\"},\"note\":\"a\"}: \
     {\"data\":{\"label\":\"n\"},\"kind\":\"Berth\",\"note\":\"a\"}\n\
     Drifting {\"kind\":\"Still\",\"note\":\"a\"}: {\"kind\":\"Calm\",\"note\":\"a\"}\n\
     Drifting {\"kind\":\"Tied\",\"data\":\"x\",\"note\":\"a\"}: \
     {\"data\":\"x\",\"kind\":\"Held\",\"note\":\"a\"}\n\
     Hawsing {\"label\":\"n\",\"name\":\"m\"}: duplicate field `label`\n\
     Tholing {\"by_tide\":{\"Ebb\":1},\"count\":2,\"mark\":\" m \",\"spare\":{\"label\":\"b\"},\
     \"spud\":{\"label\":\"a\"}}: \
     {\"by_tide\":{\"Ebb\":1},\"count\":2,\"mark\":\"m\",\"spare\":{\"label\":\"b\"},\
     \"spud\":{\"label\":\"a\"}}\n\
     Tholing {\"tides\":{\"Flood\":1},\"tally\":2,\"tag\":\" m \",\"spare_post\":{\"label\":\"b\"},\
     \"post\":{\"label\":\"a\"}}: \
     {\"by_tide\":{\"Flood\":1},\"count\":2,\"mark\":\"m\",\"spare\":{\"label\":\"b\"},\
     \"spud\":{\"label\":\"a\"}}\n\
     Tholing {\"tides\":{},\"tag\":\"m\",\"post\":{\"label\":\"a\"}}: \
     {\"by_tide\":{},\"count\":0,\"mark\":\"m\",\"spud\":{\"label\":\"a\"}}\n\
     Tholing {\"by_tide\":{},\"tag\":\"m\",\"post\":{\"label\":\"\"}}: \
     'spud.label': too short: minimum length is 1, got 0\n\
     Tholing {\"by_tide\":{},\"count\":1,\"tally\":2,\"mark\":\"m\",\"spud\":{\"label\":\"a\"}}: \
     duplicate field `count`\n";

/// What serde writes back of `payload` read as `T`, its keys sorted as the Swift encoder of the
/// run sorts them, or `refused` where serde refuses it.
fn written_back<T>(payload: &str) -> String
where
    T: DeserializeOwned + Serialize,
{
    serde_json::from_str::<T>(payload).map_or_else(
        |_| "refused".to_owned(),
        |read| sorted(serde_json::to_value(&read).unwrap()).to_string(),
    )
}

/// What serde writes back of `payload` read as `T`, as [`written_back`] tells it, or what it says
/// of a payload it refuses, ahead of the position it says it at.
fn said<T>(payload: &str) -> String
where
    T: DeserializeOwned + Serialize,
{
    serde_json::from_str::<T>(payload).map_or_else(
        |refused| {
            let refusal = refused.to_string();
            refusal
                .split_once(" at line")
                .map_or_else(|| refusal.clone(), |(told, _position)| told.to_owned())
        },
        |read| sorted(serde_json::to_value(&read).unwrap()).to_string(),
    )
}

/// `value` with the keys of each object it holds in their sorted order.
fn sorted(value: Value) -> Value {
    match value {
        Value::Object(object) => {
            let mut entries: Vec<(String, Value)> = object.into_iter().collect();
            entries.sort_by(|(left, _), (right, _)| left.cmp(right));
            Value::Object(
                entries
                    .into_iter()
                    .map(|(key, held)| (key, sorted(held)))
                    .collect(),
            )
        }
        Value::Array(items) => Value::Array(items.into_iter().map(sorted).collect()),
        scalar @ (Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_)) => scalar,
    }
}

/// The type each payload is read as, in Swift, and what serde writes back of each.
fn rows() -> Vec<(&'static str, &'static str, String)> {
    let mut rows = Vec::new();
    let mut of = |read_as: &'static str, payloads: &[&'static str], told: fn(&str) -> String| {
        rows.extend(
            payloads
                .iter()
                .map(|payload| (read_as, *payload, told(payload))),
        );
    };
    of("Surging", &SURGING, written_back::<Surging>);
    of("Careening", &CAREENING, written_back::<Careening>);
    of("Scending", &SCENDING, written_back::<Scending>);
    of("Swell", &SWELL, written_back::<Swell>);
    of("Lulling", &LULLING, written_back::<Lulling>);
    of("Tiding", &[TIDING], written_back::<Tiding>);
    of("Veering", &VEERING, written_back::<Veering>);
    of("Hawsing", &HAWSING, written_back::<Hawsing>);
    of("Gybing<String>", &HAWSING, written_back::<Gybing<String>>);
    of("Fairleading", &FAIRLEADING, written_back::<Fairleading>);
    of("Fothering", &FAIRLEADING, written_back::<Fothering>);
    of("Riding", &RIDING, written_back::<Riding>);
    of("Swinging", &SWINGING, written_back::<Swinging>);
    of("Drifting", &DRIFTING, written_back::<Drifting>);
    for (read_as, payload, told) in &mut rows {
        if REFUSED_AS_ZOD_REFUSES.contains(&(*read_as, *payload)) {
            assert_ne!(told, "refused", "serde reads {read_as} from {payload}");
            "refused".clone_into(told);
        }
    }
    rows
}

/// Each payload is read under each name serde reads it under, a unit variant's entry over `null`
/// flattened among them, written back under the name serde writes, and refused where serde
/// refuses it. A member under two of its names is refused with the words serde refuses it with.
#[test]
fn a_name_serde_reads_beside_the_one_it_writes_is_read_in_swift() {
    let rows = rows();
    let mut expected = String::new();
    let mut main = String::new();
    for (read_as, payload, told) in &rows {
        let _ = writeln!(expected, "{read_as} {payload}: {told}");
        let _ = writeln!(main, "row(\"{read_as}\", {read_as}.self, #\"{payload}\"#)");
    }
    let mut why = |read_as: &str, payload: &str, told: String| {
        let _ = writeln!(expected, "{read_as} {payload}: {told}");
        let _ = writeln!(main, "said(\"{read_as}\", {read_as}.self, #\"{payload}\"#)");
    };
    why("Hawsing", HAWSING[3], said::<Hawsing>(HAWSING[3]));
    for payload in THOLING {
        why("Tholing", payload, said::<Tholing>(payload));
    }
    assert_eq!(
        expected, PRINTED,
        "what serde reads of each payload, the rows the Swift read refuses as Zod does apart"
    );
    let source = [
        "import Foundation".to_owned(),
        pennon_swift::swift_definition(),
        surging_swift::swift_definition(),
        careening_swift::swift_definition(),
        scending_swift::swift_definition(),
        swell_swift::swift_definition(),
        lulling_swift::swift_definition(),
        tiding_swift::swift_definition(),
        veering_swift::swift_definition(),
        hawsing_swift::swift_definition(),
        gybing_swift::swift_definition(),
        fairleading_swift::swift_definition(),
        fothering_swift::swift_definition(),
        riding_swift::swift_definition(),
        swinging_swift::swift_definition(),
        drifting_swift::swift_definition(),
        spud_swift::swift_definition(),
        tideway_swift::swift_definition(),
        tholing_swift::swift_definition(),
        HELPERS.to_owned(),
        main,
    ]
    .join("\n\n");
    let Some(run) = judged(
        "swift-aliased",
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
    assert_eq!(
        run.stdout, expected,
        "what the Swift read prints of each payload"
    );
}
