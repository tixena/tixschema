//! A map flattened after a struct, beside a member of its holder's own and in a variant under a
//! tag key, a map that is the one flattened member, a map flattened ahead of a struct, two
//! flattened maps, and a map flattened after a member that takes none of the keys it reads, each
//! read and written back by the emitted Dart types under the Dart VM, against what serde reads and
//! writes back.

#![cfg(feature = "dart")]

use alloc::collections::BTreeMap;

use serde::Serialize;

use super::dart_flattened_models::{
    Belaying, CLAMPING, Clamping, Coupling, FASTENING, Fastening, Hoisting, Lacing, PAYLOAD,
    Rigging, STOWING, Stowing, TETHERING, Tethering, belaying_dart, bracing_dart, clamping_dart,
    coupling_dart, fastening_dart, hoisting_dart, lacing_dart, marking_dart, rigging_dart,
    stowing_dart, tethering_dart, ticketing_dart,
};
use super::run_dart_validate::{dart, module};

/// The function the models name, which answers the payload as it was handed.
const AS_GIVEN: &str = "dynamic asGiven(dynamic payload) => payload;";

/// What prints the map each holder read after a struct, and what it writes back.
const AFTER_A_STRUCT: &str = "
void main() {
  final coupling = Coupling.fromJson(jsonDecode(r'PAYLOAD'));
  print('coupling: ${jsonEncode(coupling.rest)} ${jsonEncode(coupling.toJson())}');
  final clamping = Clamping.fromJson(jsonDecode(r'CLAMPING'));
  print('clamping: ${jsonEncode(clamping.rest)} ${jsonEncode(clamping.toJson())}');
  final tethering = Tethering.fromJson(jsonDecode(r'TETHERING')) as TetheringHeld;
  print('tethering: ${jsonEncode(tethering.rest)} ${jsonEncode(tethering.toJson())}');
  final fastening = Fastening.fromJson(jsonDecode(r'FASTENING'));
  print('fastening: ${jsonEncode(fastening.rest)} ${jsonEncode(fastening.toJson())}');
}
";

/// What prints each member a map ahead of a struct and two maps read, what each holder writes
/// back, and what a value whose two maps hold one key apart is written as and read back as.
const AHEAD: &str = "
void main() {
  final hoisting = Hoisting.fromJson(jsonDecode(r'PAYLOAD'));
  print('hoisting: ${jsonEncode(hoisting.extra)} ${jsonEncode(hoisting.marking.toJson())} ${jsonEncode(hoisting.toJson())}');
  final stowing = Stowing.fromJson(jsonDecode(r'STOWING'));
  print('stowing: ${jsonEncode(stowing.first)} ${jsonEncode(stowing.second)} ${jsonEncode(stowing.toJson())}');
  final apart = Stowing(first: {'north': 1}, second: {'north': 2, 'south': 3});
  final written = jsonEncode(apart.toJson());
  final back = Stowing.fromJson(jsonDecode(written));
  print('apart: $written ${jsonEncode(back.first)} ${jsonEncode(back.second)}');
}
";

/// What prints the map each holder read after a member that takes no key, and what it writes
/// back.
const AFTER_NO_KEY: &str = "
void main() {
  final lacing = Lacing.fromJson(jsonDecode(r'PAYLOAD'));
  print('lacing: ${jsonEncode(lacing.rest)} ${jsonEncode(lacing.toJson())}');
  final rigging = Rigging.fromJson(jsonDecode(r'PAYLOAD'));
  print('rigging: ${jsonEncode(rigging.rest)} ${jsonEncode(rigging.toJson())}');
  final belaying = Belaying.fromJson(jsonDecode(r'PAYLOAD'));
  print('belaying: ${jsonEncode(belaying.rest)} ${jsonEncode(belaying.toJson())}');
}
";

/// The text of `value` as serde writes it.
fn written<T>(value: &T) -> String
where
    T: Serialize,
{
    serde_json::to_string(value).unwrap()
}

/// The object `text` reads as, each key once with its last value: serde writes a key two members
/// hold twice, and a Dart `Map` holds it once.
fn each_key_once(text: &str) -> String {
    serde_json::from_str::<serde_json::Value>(text)
        .unwrap()
        .to_string()
}

/// `main` with each payload written in.
fn with_payloads(main: &str) -> String {
    main.replace("PAYLOAD", PAYLOAD)
        .replace("CLAMPING", CLAMPING)
        .replace("TETHERING", TETHERING)
        .replace("FASTENING", FASTENING)
        .replace("STOWING", STOWING)
}

/// What the Dart VM prints of `main` over the emitted definitions, or `None` where it is missing.
fn printed(named: &str, definitions: &[String], main: &str) -> Option<String> {
    let source = module(definitions, &[AS_GIVEN], &with_payloads(main));
    let (ran, printed) = dart(named, &[], &source)?;
    assert!(ran, "{printed}");
    Some(printed)
}

/// A map flattened after a struct holds the keys the struct did not take, as serde hands it them,
/// and each holder is written back as serde writes it back. A map that is the one flattened member
/// reads and writes as it did.
#[test]
fn a_map_after_a_flattened_struct_reads_in_dart_the_keys_serde_hands_it() {
    let coupling: Coupling = serde_json::from_str(PAYLOAD).unwrap();
    let clamping: Clamping = serde_json::from_str(CLAMPING).unwrap();
    let tethering: Tethering = serde_json::from_str(TETHERING).unwrap();
    let Tethering::Held { marking, rest } = &tethering;
    assert_eq!(marking.label, "n");
    let fastening: Fastening = serde_json::from_str(FASTENING).unwrap();
    let expected = format!(
        "coupling: {} {}\nclamping: {} {}\ntethering: {} {}\nfastening: {} {}\n",
        written(&coupling.rest),
        written(&coupling),
        written(&clamping.rest),
        written(&clamping),
        written(rest),
        written(&tethering),
        written(&fastening.rest),
        written(&fastening),
    );
    assert_eq!(
        expected,
        "coupling: {\"north\":2} {\"label\":\"n\",\"north\":2}\n\
         clamping: {\"north\":2} {\"label\":\"n\",\"note\":\"a\",\"north\":2}\n\
         tethering: {\"north\":2} {\"kind\":\"Held\",\"label\":\"n\",\"north\":2}\n\
         fastening: {\"north\":2} {\"note\":\"a\",\"north\":2}\n"
    );
    let Some(printed) = printed(
        "flattened-after-a-struct",
        &[
            marking_dart::dart_definition(),
            coupling_dart::dart_definition(),
            clamping_dart::dart_definition(),
            tethering_dart::dart_definition(),
            fastening_dart::dart_definition(),
        ],
        AFTER_A_STRUCT,
    ) else {
        return;
    };
    assert_eq!(printed, expected);
}

/// A map flattened ahead of a struct is handed every key, the struct's among them, and each of two
/// maps the same keys, as serde hands them. serde writes a key two members hold twice: Dart writes
/// it once, with the last value, which is what serde reads back of the text it wrote.
#[test]
fn a_map_ahead_of_a_struct_and_two_maps_read_and_write_in_dart_as_serde_does() {
    let hoisting: Hoisting = serde_json::from_str(PAYLOAD).unwrap();
    let hoisting_written = written(&hoisting);
    assert_eq!(hoisting_written, r#"{"label":"n","north":2,"label":"n"}"#);
    let stowing: Stowing = serde_json::from_str(STOWING).unwrap();
    let apart = Stowing {
        first: BTreeMap::from([("north".to_owned(), 1)]),
        second: BTreeMap::from([("north".to_owned(), 2), ("south".to_owned(), 3)]),
    };
    let apart_written = written(&apart);
    assert_eq!(apart_written, r#"{"north":1,"north":2,"south":3}"#);
    let back: Stowing = serde_json::from_str(&apart_written).unwrap();
    let expected = format!(
        "hoisting: {} {} {}\nstowing: {} {} {}\napart: {} {} {}\n",
        written(&hoisting.extra),
        written(&hoisting.marking),
        each_key_once(&hoisting_written),
        written(&stowing.first),
        written(&stowing.second),
        each_key_once(&written(&stowing)),
        each_key_once(&apart_written),
        written(&back.first),
        written(&back.second),
    );
    assert_eq!(
        expected,
        "hoisting: {\"label\":\"n\",\"north\":2} {\"label\":\"n\"} {\"label\":\"n\",\"north\":2}\n\
         stowing: {\"north\":2,\"south\":3} {\"north\":2,\"south\":3} {\"north\":2,\"south\":3}\n\
         apart: {\"north\":2,\"south\":3} {\"north\":2,\"south\":3} {\"north\":2,\"south\":3}\n"
    );
    let Some(printed) = printed(
        "flattened-ahead",
        &[
            marking_dart::dart_definition(),
            hoisting_dart::dart_definition(),
            stowing_dart::dart_definition(),
        ],
        AHEAD,
    ) else {
        return;
    };
    assert_eq!(printed, expected);
}

/// A struct that names a function on itself, a member that names one, and a struct with a
/// flattened member of its own take none of the keys they read, so a map flattened after them is
/// handed every key, as serde hands it them.
#[test]
fn a_map_after_a_member_that_takes_no_key_reads_in_dart_every_key() {
    let lacing: Lacing = serde_json::from_str(PAYLOAD).unwrap();
    let rigging: Rigging = serde_json::from_str(PAYLOAD).unwrap();
    let belaying: Belaying = serde_json::from_str(PAYLOAD).unwrap();
    let expected = format!(
        "lacing: {} {}\nrigging: {} {}\nbelaying: {} {}\n",
        written(&lacing.rest),
        each_key_once(&written(&lacing)),
        written(&rigging.rest),
        each_key_once(&written(&rigging)),
        written(&belaying.rest),
        each_key_once(&written(&belaying)),
    );
    assert_eq!(
        expected,
        "lacing: {\"label\":\"n\",\"north\":2} {\"label\":\"n\",\"north\":2}\n\
         rigging: {\"label\":\"n\",\"north\":2} {\"label\":\"n\",\"north\":2}\n\
         belaying: {\"label\":\"n\",\"north\":2} {\"label\":\"n\",\"north\":2}\n"
    );
    let Some(printed) = printed(
        "flattened-after-no-key",
        &[
            marking_dart::dart_definition(),
            ticketing_dart::dart_definition(),
            bracing_dart::dart_definition(),
            lacing_dart::dart_definition(),
            rigging_dart::dart_definition(),
            belaying_dart::dart_definition(),
        ],
        AFTER_NO_KEY,
    ) else {
        return;
    };
    assert_eq!(printed, expected);
}
