//! A map that is the one member of its struct and of a variant under a tag key, and a member that
//! names a function, flattened after a struct, beside a member of its holder's own, ahead of a
//! struct, after a member that takes none of the keys it reads, and after an externally tagged
//! enum, an enum under a tag key and an untagged one, each read and written back by the emitted
//! Dart types under the Dart VM, against what serde reads and writes back.

#![cfg(feature = "dart")]

use core::fmt::Write as _;

use serde::{Deserialize, Serialize};

use super::dart_flattened_models::{
    Belaying, CLAMPING, Clamping, Doubling, FASTENING, Fastening, Hoisting, KEDGING, Kedging,
    LOADING, Lacing, Loading, MOUSING, Mousing, PAYLOAD, Rigging, SPARE, Shoring, Spare, TETHERING,
    TRIMMING, Tethering, Trimming, Twin, belaying_dart, bracing_dart, chocking_dart, clamping_dart,
    doubling_dart, fastening_dart, frapping_dart, hatching_dart, hoisting_dart, kedging_dart,
    keying_dart, lacing_dart, loading_dart, marking_dart, mousing_dart, rigging_dart, shoring_dart,
    spare_dart, tethering_dart, ticketing_dart, trimming_dart, twin_dart, warping_dart,
};
use super::run_dart_validate::{dart, module};

/// The function the models name, which answers the payload as it was handed.
const AS_GIVEN: &str = "dynamic asGiven(dynamic payload) => payload;";

/// The function the models name, which answers the keys of the object it is handed.
const WITH_ITS_KEYS: &str = "dynamic withItsKeys(dynamic payload) => \
     {'keys': (payload as Map<String, dynamic>).keys.toList()};";

/// What prints the map each holder read, and what it writes back.
const ALONE: &str = "
void main() {
  final spare = Spare.fromJson(jsonDecode(r'SPARE'));
  print('spare: ${jsonEncode(spare.rest)} ${jsonEncode(spare.toJson())}');
  final tethering = Tethering.fromJson(jsonDecode(r'TETHERING')) as TetheringHeld;
  print('tethering: ${jsonEncode(tethering.rest)} ${jsonEncode(tethering.toJson())}');
}
";

/// What prints the keys each holder's function was handed, and what the holder writes back.
const HANDED: &str = "
void main() {
  final clamping = Clamping.fromJson(jsonDecode(r'CLAMPING'));
  print('clamping: ${jsonEncode(clamping.other.keys)} ${jsonEncode(clamping.toJson())}');
  final fastening = Fastening.fromJson(jsonDecode(r'FASTENING'));
  print('fastening: ${jsonEncode(fastening.other.keys)} ${jsonEncode(fastening.toJson())}');
  final hoisting = Hoisting.fromJson(jsonDecode(r'PAYLOAD'));
  print('hoisting: ${jsonEncode(hoisting.extra.keys)} ${jsonEncode(hoisting.toJson())}');
  final lacing = Lacing.fromJson(jsonDecode(r'PAYLOAD'));
  print('lacing: ${jsonEncode(lacing.rest.keys)} ${jsonEncode(lacing.toJson())}');
  final rigging = Rigging.fromJson(jsonDecode(r'PAYLOAD'));
  print('rigging: ${jsonEncode(rigging.rest.keys)} ${jsonEncode(rigging.toJson())}');
  final belaying = Belaying.fromJson(jsonDecode(r'PAYLOAD'));
  print('belaying: ${jsonEncode(belaying.rest.keys)} ${jsonEncode(belaying.toJson())}');
  final twin = Twin.fromJson(jsonDecode(r'PAYLOAD'));
  print('twin: ${jsonEncode(twin.other.keys)} ${jsonEncode(twin.toJson())}');
  try {
    Doubling.fromJson(jsonDecode(r'PAYLOAD'));
    print('doubling: read');
  } catch (_) {
    print('doubling: refused');
  }
}
";

/// What prints each holder of a flattened enum read from each of its payloads: the enum, the keys
/// the function after it was handed and the holder written back, or that the read is refused.
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
  return '${jsonEncode(loading.hatching.toJson())} ${jsonEncode(loading.rest.keys)} ${jsonEncode(loading.toJson())}';
}

String kedged(String payload) {
  final kedging = Kedging.fromJson(jsonDecode(payload));
  return '${jsonEncode(kedging.kedge.toJson())} ${jsonEncode(kedging.rest.keys)} ${jsonEncode(kedging.toJson())}';
}

void main() {
  for (final payload in LOADING) {
    print('loading: ${row(() => loaded(payload))}');
  }
  for (final payload in KEDGING) {
    print('kedging: ${row(() => kedged(payload))}');
  }
  final trimming = Trimming.fromJson(jsonDecode(r'TRIMMING'));
  print('trimming: ${jsonEncode(trimming.marking.toJson())} ${jsonEncode(trimming.rest.keys)} ${jsonEncode(trimming.toJson())}');
  final mousing = Mousing.fromJson(jsonDecode(r'MOUSING'));
  print('mousing: ${jsonEncode(mousing.rest.keys)} ${jsonEncode(mousing.toJson())}');
  final shoring = Shoring.fromJson(jsonDecode(r'PAYLOAD'));
  print('shoring: ${jsonEncode(shoring.rest.keys)} ${jsonEncode(shoring.toJson())}');
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
        .replace("SPARE", SPARE)
        .replace("CLAMPING", CLAMPING)
        .replace("TETHERING", TETHERING)
        .replace("FASTENING", FASTENING)
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

/// A map that is the one member of its struct holds every key, and one in a variant under a tag
/// key every key but the tag, as serde hands them, and each holder is written back as serde writes
/// it back.
#[test]
fn a_map_alone_reads_in_dart_the_keys_serde_hands_it() {
    let spare: Spare = serde_json::from_str(SPARE).unwrap();
    let tethering: Tethering = serde_json::from_str(TETHERING).unwrap();
    let Tethering::Held { rest } = &tethering;
    let expected = format!(
        "spare: {} {}\ntethering: {} {}\n",
        written(&spare.rest),
        written(&spare),
        written(rest),
        written(&tethering),
    );
    assert_eq!(
        expected,
        "spare: {\"north\":2,\"south\":3} {\"north\":2,\"south\":3}\n\
         tethering: {\"north\":2} {\"kind\":\"Held\",\"north\":2}\n"
    );
    let Some(printed) = printed(
        "flattened-alone",
        &[
            spare_dart::dart_definition(),
            tethering_dart::dart_definition(),
        ],
        ALONE,
    ) else {
        return;
    };
    assert_eq!(printed, expected);
}

/// A member that names a function is handed the keys no member ahead of it took, as serde hands it
/// them: after a struct, beside a member of its holder's own, ahead of a struct, and after a
/// member that takes no key. Each holder is written back as serde writes it back. A struct after a
/// struct that reads the same key is refused, as serde refuses it.
#[test]
fn a_function_reads_in_dart_the_keys_serde_hands_it() {
    let clamping: Clamping = serde_json::from_str(CLAMPING).unwrap();
    let fastening: Fastening = serde_json::from_str(FASTENING).unwrap();
    let hoisting: Hoisting = serde_json::from_str(PAYLOAD).unwrap();
    let lacing: Lacing = serde_json::from_str(PAYLOAD).unwrap();
    let rigging: Rigging = serde_json::from_str(PAYLOAD).unwrap();
    let belaying: Belaying = serde_json::from_str(PAYLOAD).unwrap();
    let twin: Twin = serde_json::from_str(PAYLOAD).unwrap();
    let doubling = if serde_json::from_str::<Doubling>(PAYLOAD).is_ok() {
        "read"
    } else {
        "refused"
    };
    let expected = format!(
        "clamping: {} {}\nfastening: {} {}\nhoisting: {} {}\nlacing: {} {}\nrigging: {} {}\n\
         belaying: {} {}\ntwin: {} {}\ndoubling: {doubling}\n",
        written(&clamping.other.keys),
        written(&clamping),
        written(&fastening.other.keys),
        written(&fastening),
        written(&hoisting.extra.keys),
        written(&hoisting),
        written(&lacing.rest.keys),
        written(&lacing),
        written(&rigging.rest.keys),
        written(&rigging),
        written(&belaying.rest.keys),
        written(&belaying),
        written(&twin.other.keys),
        written(&twin),
    );
    assert_eq!(
        expected,
        "clamping: [\"north\"] {\"label\":\"n\",\"note\":\"a\",\"keys\":[\"north\"]}\n\
         fastening: [\"north\"] {\"note\":\"a\",\"keys\":[\"north\"]}\n\
         hoisting: [\"label\",\"north\"] {\"keys\":[\"label\",\"north\"],\"label\":\"n\"}\n\
         lacing: [\"label\",\"north\"] {\"label\":\"n\",\"keys\":[\"label\",\"north\"]}\n\
         rigging: [\"label\",\"north\"] {\"label\":\"n\",\"keys\":[\"label\",\"north\"]}\n\
         belaying: [\"label\",\"north\"] {\"label\":\"n\",\"keys\":[\"label\",\"north\"]}\n\
         twin: [\"north\"] {\"label\":\"n\",\"keys\":[\"north\"]}\n\
         doubling: refused\n"
    );
    let Some(printed) = printed(
        "flattened-handed",
        &[
            marking_dart::dart_definition(),
            keying_dart::dart_definition(),
            clamping_dart::dart_definition(),
            fastening_dart::dart_definition(),
            hoisting_dart::dart_definition(),
            ticketing_dart::dart_definition(),
            bracing_dart::dart_definition(),
            lacing_dart::dart_definition(),
            rigging_dart::dart_definition(),
            belaying_dart::dart_definition(),
            twin_dart::dart_definition(),
            doubling_dart::dart_definition(),
        ],
        HANDED,
    ) else {
        return;
    };
    assert_eq!(printed, expected);
}

/// A member that names a function flattened after an externally tagged enum is handed every entry
/// but the first under a variant's name, and a struct after the enum every entry but that one, as
/// serde hands them, whichever variant's entry comes first. A function after an enum under a tag
/// key alone, or after an untagged one, is handed every key. Each holder is written back as serde
/// writes it back.
#[test]
fn a_function_after_a_flattened_enum_reads_in_dart_the_entries_serde_hands_it() {
    let loaded: Vec<String> = LOADING
        .iter()
        .map(|payload| {
            read_of(payload, |loading: &Loading| {
                format!(
                    "{} {} {}",
                    written(&loading.hatching),
                    written(&loading.rest.keys),
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
                    written(&kedging.rest.keys),
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
        written(&trimming.rest.keys),
        written(&trimming),
        written(&mousing.rest.keys),
        written(&mousing),
        written(&shoring.rest.keys),
        written(&shoring),
    );
    let expected = format!("{}{after_loading}", rows("loading", &loaded));
    assert_eq!(
        expected,
        "loading: {\"Bay\":{\"label\":\"n\"}} [\"north\"] {\"Bay\":{\"label\":\"n\"},\"keys\":[\"north\"]}\n\
         loading: {\"Bay\":{\"label\":\"n\"}} [\"Dock\",\"north\"] \
         {\"Bay\":{\"label\":\"n\"},\"keys\":[\"Dock\",\"north\"]}\n\
         loading: {\"Dock\":{\"label\":\"s\"}} [\"Bay\",\"north\"] \
         {\"Dock\":{\"label\":\"s\"},\"keys\":[\"Bay\",\"north\"]}\n\
         loading: {\"Dock\":{\"label\":\"s\"}} [\"north\"] {\"Dock\":{\"label\":\"s\"},\"keys\":[\"north\"]}\n\
         loading: {\"Bay\":{\"label\":\"n\"}} [\"Quay\",\"north\"] \
         {\"Bay\":{\"label\":\"n\"},\"keys\":[\"Quay\",\"north\"]}\n\
         kedging: {\"pier\":{\"label\":\"n\"}} [\"north\"] {\"pier\":{\"label\":\"n\"},\"keys\":[\"north\"]}\n\
         kedging: {\"open_bay\":{\"label\":\"n\"}} [\"north\"] \
         {\"open_bay\":{\"label\":\"n\"},\"keys\":[\"north\"]}\n\
         kedging: refused\n\
         trimming: {\"label\":\"m\"} [\"north\"] {\"Bay\":{\"label\":\"n\"},\"label\":\"m\",\"keys\":[\"north\"]}\n\
         mousing: [\"kind\",\"label\",\"north\"] \
         {\"kind\":\"Rope\",\"label\":\"n\",\"keys\":[\"kind\",\"label\",\"north\"]}\n\
         shoring: [\"label\",\"north\"] {\"label\":\"n\",\"keys\":[\"label\",\"north\"]}\n"
    );
    let mut in_dart = loaded;
    in_dart[UNDER_AN_ALIAS] = "refused".to_owned();
    let expected_in_dart = format!("{}{after_loading}", rows("loading", &in_dart));
    let Some(printed) = printed(
        "flattened-after-an-enum",
        &[
            marking_dart::dart_definition(),
            keying_dart::dart_definition(),
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
