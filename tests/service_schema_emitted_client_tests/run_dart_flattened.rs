//! A map flattened after a struct, beside a member of its holder's own and in a variant under a
//! tag key, a map that is the one flattened member, a map flattened ahead of a struct, two
//! flattened maps, a map flattened after a member that takes none of the keys it reads, and, after
//! a struct, a struct with a flattened map, a member that names a function, a JSON value and a
//! struct that reads the same key, and a map flattened after an externally tagged enum, after an
//! enum under a tag key and after an untagged one, each read and written back by the emitted Dart
//! types under the Dart VM, against what serde reads and writes back.

#![cfg(feature = "dart")]

use alloc::collections::BTreeMap;
use core::fmt::Write as _;

use serde::{Deserialize, Serialize};

use super::dart_flattened_models::{
    Belaying, CLAMPING, Clamping, Coupling, Doubling, FASTENING, Fastening, Hoisting, KEDGING,
    Kedging, LOADING, Lacing, Loading, Loosening, MOUSING, Mousing, PAYLOAD, Pairing, Rigging,
    STOWING, Shoring, Stowing, TETHERING, TRIMMING, Tethering, Trimming, Twin, belaying_dart,
    bracing_dart, chocking_dart, clamping_dart, coupling_dart, doubling_dart, fastening_dart,
    frapping_dart, hatching_dart, hoisting_dart, kedging_dart, keying_dart, lacing_dart,
    loading_dart, loosening_dart, marking_dart, mousing_dart, pairing_dart, rigging_dart,
    shoring_dart, spare_dart, stowing_dart, tethering_dart, ticketing_dart, trimming_dart,
    twin_dart, warping_dart,
};
use super::run_dart_validate::{dart, module};

/// The function the models name, which answers the payload as it was handed.
const AS_GIVEN: &str = "dynamic asGiven(dynamic payload) => payload;";

/// The function [`Twin`] names, which answers the keys of the object it is handed.
const WITH_ITS_KEYS: &str = "dynamic withItsKeys(dynamic payload) => \
     {'keys': (payload as Map<String, dynamic>).keys.toList()};";

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

/// What prints the member each holder read after a struct and what it writes back, and whether a
/// struct after a struct that reads the same key is read.
const AFTER_A_STRUCT_A_MEMBER: &str = "
void main() {
  final pairing = Pairing.fromJson(jsonDecode(r'PAYLOAD'));
  print('pairing: ${jsonEncode(pairing.spare.rest)} ${jsonEncode(pairing.toJson())}');
  final twin = Twin.fromJson(jsonDecode(r'PAYLOAD'));
  print('twin: ${jsonEncode(twin.other.keys)} ${jsonEncode(twin.toJson())}');
  final loosening = Loosening.fromJson(jsonDecode(r'PAYLOAD'));
  print('loosening: ${jsonEncode(loosening.rest)} ${jsonEncode(loosening.toJson())}');
  try {
    Doubling.fromJson(jsonDecode(r'PAYLOAD'));
    print('doubling: read');
  } catch (_) {
    print('doubling: refused');
  }
}
";

/// What prints each holder of a flattened enum read from each of its payloads: the enum, each
/// member after it and the holder written back, or that the read is refused.
const AFTER_AN_ENUM: &str = "
String row(String Function() read) {
  try {
    return read();
  } catch (_) {
    return 'refused';
  }
}

String loaded(String payload) {
  final loading = Loading.fromJson(jsonDecode(payload));
  return '${jsonEncode(loading.hatching.toJson())} ${jsonEncode(loading.rest)} ${jsonEncode(loading.toJson())}';
}

String kedged(String payload) {
  final kedging = Kedging.fromJson(jsonDecode(payload));
  return '${jsonEncode(kedging.kedge.toJson())} ${jsonEncode(kedging.rest)} ${jsonEncode(kedging.toJson())}';
}

void main() {
  for (final payload in LOADING) {
    print('loading: ${row(() => loaded(payload))}');
  }
  for (final payload in KEDGING) {
    print('kedging: ${row(() => kedged(payload))}');
  }
  final trimming = Trimming.fromJson(jsonDecode(r'TRIMMING'));
  print('trimming: ${jsonEncode(trimming.marking.toJson())} ${jsonEncode(trimming.rest)} ${jsonEncode(trimming.toJson())}');
  final mousing = Mousing.fromJson(jsonDecode(r'MOUSING'));
  print('mousing: ${jsonEncode(mousing.rest)} ${jsonEncode(mousing.toJson())}');
  final shoring = Shoring.fromJson(jsonDecode(r'PAYLOAD'));
  print('shoring: ${jsonEncode(shoring.rest)} ${jsonEncode(shoring.toJson())}');
}
";

/// The payload of [`LOADING`] that holds a variant's entry under its alias: Dart reads no variant
/// under an alias, and refuses what serde reads.
const UNDER_AN_ALIAS: usize = 3;

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

/// `payloads` as a Dart list of raw strings.
fn listed(payloads: &[&str]) -> String {
    let raw: Vec<String> = payloads
        .iter()
        .map(|payload| format!("r'{payload}'"))
        .collect();
    format!("[{}]", raw.join(", "))
}

/// `main` with each payload written in.
fn with_payloads(main: &str) -> String {
    main.replace("PAYLOAD", PAYLOAD)
        .replace("CLAMPING", CLAMPING)
        .replace("TETHERING", TETHERING)
        .replace("FASTENING", FASTENING)
        .replace("STOWING", STOWING)
        .replace("LOADING", &listed(&LOADING))
        .replace("KEDGING", &listed(&KEDGING))
        .replace("TRIMMING", TRIMMING)
        .replace("MOUSING", MOUSING)
}

/// What serde reads of `payload` as `T`, through `parts`, or that it refuses it.
fn read_of<T>(payload: &str, parts: fn(&T) -> String) -> String
where
    T: for<'de> Deserialize<'de>,
{
    serde_json::from_str::<T>(payload).map_or_else(|_| "refused".to_owned(), |read| parts(&read))
}

/// What the Dart VM prints of `main` over the emitted definitions, or `None` where it is missing.
fn printed(named: &str, definitions: &[String], main: &str) -> Option<String> {
    let source = module(
        definitions,
        &[AS_GIVEN, WITH_ITS_KEYS],
        &with_payloads(main),
    );
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

/// A struct with a flattened map, a member that names a function and a JSON value, each flattened
/// after a struct, are handed the keys the struct did not take, and each holder is written back as
/// serde writes it back. A struct after a struct that reads the same key is refused, as serde
/// refuses it.
#[test]
fn a_member_after_a_flattened_struct_reads_in_dart_the_keys_serde_hands_it() {
    let pairing: Pairing = serde_json::from_str(PAYLOAD).unwrap();
    let twin: Twin = serde_json::from_str(PAYLOAD).unwrap();
    let loosening: Loosening = serde_json::from_str(PAYLOAD).unwrap();
    let doubling = if serde_json::from_str::<Doubling>(PAYLOAD).is_ok() {
        "read"
    } else {
        "refused"
    };
    let expected = format!(
        "pairing: {} {}\ntwin: {} {}\nloosening: {} {}\ndoubling: {doubling}\n",
        written(&pairing.spare.rest),
        written(&pairing),
        written(&twin.other.keys),
        written(&twin),
        written(&loosening.rest),
        written(&loosening),
    );
    assert_eq!(
        expected,
        "pairing: {\"north\":2} {\"label\":\"n\",\"north\":2}\n\
         twin: [\"north\"] {\"label\":\"n\",\"keys\":[\"north\"]}\n\
         loosening: {\"north\":2} {\"label\":\"n\",\"north\":2}\n\
         doubling: refused\n"
    );
    let Some(printed) = printed(
        "flattened-after-a-struct-a-member",
        &[
            marking_dart::dart_definition(),
            spare_dart::dart_definition(),
            pairing_dart::dart_definition(),
            keying_dart::dart_definition(),
            twin_dart::dart_definition(),
            loosening_dart::dart_definition(),
            doubling_dart::dart_definition(),
        ],
        AFTER_A_STRUCT_A_MEMBER,
    ) else {
        return;
    };
    assert_eq!(printed, expected);
}

/// A map flattened after an externally tagged enum is handed every entry but the first under a
/// variant's name, and a struct after the enum every entry but that one, as serde hands them,
/// whichever variant's entry comes first. A map after an enum under a tag key alone, or after an
/// untagged one, is handed every key. Each holder is written back as serde writes it back.
#[test]
fn a_map_after_a_flattened_enum_reads_in_dart_the_entries_serde_hands_it() {
    let loaded: Vec<String> = LOADING
        .iter()
        .map(|payload| {
            read_of(payload, |loading: &Loading| {
                format!(
                    "{} {} {}",
                    written(&loading.hatching),
                    written(&loading.rest),
                    written(loading)
                )
            })
        })
        .collect();
    let kedged: Vec<String> = KEDGING
        .iter()
        .map(|payload| {
            read_of(payload, |kedging: &Kedging| {
                format!(
                    "{} {} {}",
                    written(&kedging.kedge),
                    written(&kedging.rest),
                    written(kedging)
                )
            })
        })
        .collect();
    let trimming: Trimming = serde_json::from_str(TRIMMING).unwrap();
    let mousing: Mousing = serde_json::from_str(MOUSING).unwrap();
    let shoring: Shoring = serde_json::from_str(PAYLOAD).unwrap();
    let rows = |named: &str, read: &[String]| {
        read.iter().fold(String::new(), |mut acc, row| {
            writeln!(acc, "{named}: {row}").unwrap();
            acc
        })
    };
    let after_loading = format!(
        "{}trimming: {} {} {}\nmousing: {} {}\nshoring: {} {}\n",
        rows("kedging", &kedged),
        written(&trimming.marking),
        written(&trimming.rest),
        written(&trimming),
        written(&mousing.rest),
        each_key_once(&written(&mousing)),
        written(&shoring.rest),
        each_key_once(&written(&shoring)),
    );
    let expected = format!("{}{after_loading}", rows("loading", &loaded));
    assert_eq!(
        expected,
        "loading: {\"Bay\":{\"label\":\"n\"}} {\"north\":2} {\"Bay\":{\"label\":\"n\"},\"north\":2}\n\
         loading: {\"Bay\":{\"label\":\"n\"}} {\"Dock\":{\"label\":\"s\"},\"north\":2} \
         {\"Bay\":{\"label\":\"n\"},\"Dock\":{\"label\":\"s\"},\"north\":2}\n\
         loading: {\"Dock\":{\"label\":\"s\"}} {\"Bay\":{\"label\":\"n\"},\"north\":2} \
         {\"Dock\":{\"label\":\"s\"},\"Bay\":{\"label\":\"n\"},\"north\":2}\n\
         loading: {\"Dock\":{\"label\":\"s\"}} {\"north\":2} {\"Dock\":{\"label\":\"s\"},\"north\":2}\n\
         loading: {\"Bay\":{\"label\":\"n\"}} {\"Quay\":{\"label\":\"s\"},\"north\":2} \
         {\"Bay\":{\"label\":\"n\"},\"Quay\":{\"label\":\"s\"},\"north\":2}\n\
         kedging: {\"pier\":{\"label\":\"n\"}} {\"north\":2} {\"pier\":{\"label\":\"n\"},\"north\":2}\n\
         kedging: {\"open_bay\":{\"label\":\"n\"}} {\"north\":2} \
         {\"open_bay\":{\"label\":\"n\"},\"north\":2}\n\
         kedging: refused\n\
         trimming: {\"label\":\"m\"} {\"north\":2} {\"Bay\":{\"label\":\"n\"},\"label\":\"m\",\"north\":2}\n\
         mousing: {\"kind\":\"Rope\",\"label\":\"n\",\"north\":2} \
         {\"kind\":\"Rope\",\"label\":\"n\",\"north\":2}\n\
         shoring: {\"label\":\"n\",\"north\":2} {\"label\":\"n\",\"north\":2}\n"
    );
    let mut in_dart = loaded;
    in_dart[UNDER_AN_ALIAS] = "refused".to_owned();
    let expected_in_dart = format!("{}{after_loading}", rows("loading", &in_dart));
    let Some(printed) = printed(
        "flattened-after-an-enum",
        &[
            marking_dart::dart_definition(),
            hatching_dart::dart_definition(),
            loading_dart::dart_definition(),
            warping_dart::dart_definition(),
            kedging_dart::dart_definition(),
            trimming_dart::dart_definition(),
            frapping_dart::dart_definition(),
            mousing_dart::dart_definition(),
            chocking_dart::dart_definition(),
            shoring_dart::dart_definition(),
        ],
        AFTER_AN_ENUM,
    ) else {
        return;
    };
    assert_eq!(printed, expected_in_dart);
}
