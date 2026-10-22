//! A map that is the one member of its struct, a member that names a function, flattened after a
//! struct, beside a member of its holder's own, ahead of a struct, after a member that takes none
//! of the keys it reads, and after an externally tagged enum, an enum under a tag key and an
//! untagged one, and an externally tagged enum beside a member of its holder's own, optional too,
//! and an optional map alone in its struct or in a variant, keyed by a plain enum and of models,
//! each read and written back by the emitted Dart types under the Dart VM, against what serde reads
//! and writes back.

#![cfg(feature = "dart")]

use core::fmt::Write as _;

use serde::{Deserialize, Serialize};

use super::dart_flattened_models::{
    BATTENING, BILGING, Battening, Belaying, Bilging, CAULKING, CLAMPING, Caulking, Clamping,
    DUNNAGE, Doubling, Dunnage, FASTENING, FREEBOARD, Fastening, Freeboard, Hawsehole, Hoisting,
    KEDGING, Kedging, LOADING, Lacing, Loading, MOUSING, Mousing, PAYLOAD, Rigging, SPARE,
    STEEVING, Scuppering, Shoring, Spare, Steeving, TRIMMING, Trimming, Twin, battening_dart,
    belaying_dart, bilging_dart, bracing_dart, caulking_dart, chocking_dart, clamping_dart,
    doubling_dart, dunnage_dart, fastening_dart, frapping_dart, freeboard_dart, garboard_dart,
    hatching_dart, hawsehole_dart, hoisting_dart, kedging_dart, keying_dart, lacing_dart,
    loading_dart, marking_dart, mousing_dart, rigging_dart, scuppering_dart, shoring_dart,
    spare_dart, steeving_dart, ticketing_dart, trimming_dart, twin_dart, warping_dart,
};
#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
use super::dart_flattened_models::{LIMBER, Limber, limber_dart, scantling_dart};
use super::run_dart_validate::{dart, module};

/// The function the models name, which answers the payload as it was handed.
const AS_GIVEN: &str = "dynamic asGiven(dynamic payload) => payload;";

/// The function the models name, which answers the keys of the object it is handed.
const WITH_ITS_KEYS: &str = "dynamic withItsKeys(dynamic payload) => \
     {'keys': (payload as Map<String, dynamic>).keys.toList()};";

/// What prints the map the holder read, and what it writes back.
const ALONE: &str = "
void main() {
  final spare = Spare.fromJson(jsonDecode(r'SPARE'));
  print('spare: ${jsonEncode(spare.rest)} ${jsonEncode(spare.toJson())}');
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

/// What prints the enum and the member [`Battening`] read from each of its payloads, and what it
/// writes back, or that the read is refused.
const BESIDE_A_MEMBER: &str = "
String battened(String payload) {
  try {
    final battening = Battening.fromJson(jsonDecode(payload));
    return '${jsonEncode(battening.hatching.toJson())} ${battening.note} ${jsonEncode(battening.toJson())}';
  } catch (_) {
    return 'refused';
  }
}

void main() {
  for (final payload in BATTENING) {
    print('battening: ${battened(payload)}');
  }
}
";

/// What prints the enum and the member [`Caulking`] read from each of its payloads, and what it
/// writes back, or that the read is refused.
const OPTIONAL_BESIDE_A_MEMBER: &str = "
String caulked(String payload) {
  try {
    final caulking = Caulking.fromJson(jsonDecode(payload));
    return '${jsonEncode(caulking.hatching?.toJson())} ${caulking.note} ${jsonEncode(caulking.toJson())}';
  } catch (_) {
    return 'refused';
  }
}

void main() {
  for (final payload in CAULKING) {
    print('caulking: ${caulked(payload)}');
  }
}
";

/// What prints the map each holder of an optional map read from each of its payloads, and what the
/// holder writes back, or that the read is refused.
const OPTIONAL_MAPS: &str = "
String row(String Function() read) {
  try {
    return read();
  } catch (_) {
    return 'refused';
  }
}

String dunnage(String payload) {
  final read = Dunnage.fromJson(jsonDecode(payload));
  return '${jsonEncode(read.counts)} ${jsonEncode(read.toJson())}';
}

String steeving(String payload) {
  final read = Steeving.fromJson(jsonDecode(payload));
  return '${jsonEncode((read as SteevingOpen).counts)} ${jsonEncode(read.toJson())}';
}

String freeboard(String payload) {
  final read = Freeboard.fromJson(jsonDecode(payload));
  return '${jsonEncode((read as FreeboardOpen).counts)} ${jsonEncode(read.toJson())}';
}

String scuppering(String payload) {
  final read = Scuppering.fromJson(jsonDecode(payload));
  return '${jsonEncode((read as ScupperingOpen).counts)} ${jsonEncode(read.toJson())}';
}

String bilging(String payload) {
  final read = Bilging.fromJson(jsonDecode(payload));
  final counts = read.counts?.map((key, count) => MapEntry(key.toJson(), count));
  return '${jsonEncode(counts)} ${jsonEncode(read.toJson())}';
}

String hawsehole(String payload) {
  final read = Hawsehole.fromJson(jsonDecode(payload));
  return '${jsonEncode(read.counts)} ${jsonEncode(read.toJson())}';
}

void main() {
  for (final payload in DUNNAGE) {
    print('dunnage: ${row(() => dunnage(payload))}');
  }
  for (final payload in STEEVING) {
    print('steeving: ${row(() => steeving(payload))}');
  }
  for (final payload in FREEBOARD) {
    print('freeboard: ${row(() => freeboard(payload))}');
  }
  for (final payload in DUNNAGE) {
    print('scuppering: ${row(() => scuppering(payload))}');
  }
  for (final payload in BILGING) {
    print('bilging: ${row(() => bilging(payload))}');
  }
  for (final payload in DUNNAGE) {
    print('hawsehole: ${row(() => hawsehole(payload))}');
  }
}
";

/// What prints the map [`Limber`] read from each of its payloads, and what it writes back, or the
/// message of the read's refusal.
#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
const OPTIONAL_MAP_OF_MODELS: &str = "
void main() {
  for (final payload in LIMBER) {
    try {
      final read = Limber.fromJson(jsonDecode(payload));
      print('limber: ${jsonEncode(read.notches)} ${jsonEncode(read.toJson())}');
    } on FormatException catch (refused) {
      print('limber: ${refused.message}');
    }
  }
}
";

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
        .replace("FASTENING", FASTENING)
        .replace("LOADING", &listed(&LOADING))
        .replace("BATTENING", &listed(&BATTENING))
        .replace("CAULKING", &listed(&CAULKING))
        .replace("KEDGING", &listed(&KEDGING))
        .replace("TRIMMING", TRIMMING)
        .replace("MOUSING", MOUSING)
        .replace("DUNNAGE", &listed(&DUNNAGE))
        .replace("STEEVING", &listed(&STEEVING))
        .replace("FREEBOARD", &listed(&FREEBOARD))
        .replace("BILGING", &listed(&BILGING))
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

/// A map that is the one member of its struct holds every key, as serde hands them, and the
/// holder is written back as serde writes it back.
#[test]
fn a_map_alone_reads_in_dart_the_keys_serde_hands_it() {
    let spare: Spare = serde_json::from_str(SPARE).unwrap();
    let expected = format!("spare: {} {}\n", written(&spare.rest), written(&spare));
    assert_eq!(
        expected,
        "spare: {\"north\":2,\"south\":3} {\"north\":2,\"south\":3}\n"
    );
    let Some(printed) = printed("flattened-alone", &[spare_dart::dart_definition()], ALONE) else {
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

/// An externally tagged enum flattened ahead of a member that names a function is read from the
/// first entry under a variant's name or an alias of one, wherever it is in the object, and the
/// function is handed every entry but that one, as serde reads them. A struct after the enum is
/// handed every entry but that one. A function after an enum under a tag key alone, or after an
/// untagged one, is handed every key. Each holder is written back as serde writes it back.
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
         loading: {\"Bay\":{\"label\":\"n\"}} [\"north\"] {\"Bay\":{\"label\":\"n\"},\"keys\":[\"north\"]}\n\
         loading: {\"Dock\":{\"label\":\"s\"}} [\"north\",\"south\",\"Bay\"] \
         {\"Dock\":{\"label\":\"s\"},\"keys\":[\"north\",\"south\",\"Bay\"]}\n\
         kedging: {\"pier\":{\"label\":\"n\"}} [\"north\"] {\"pier\":{\"label\":\"n\"},\"keys\":[\"north\"]}\n\
         kedging: {\"open_bay\":{\"label\":\"n\"}} [\"north\"] \
         {\"open_bay\":{\"label\":\"n\"},\"keys\":[\"north\"]}\n\
         kedging: refused\n\
         trimming: {\"label\":\"m\"} [\"north\"] {\"Bay\":{\"label\":\"n\"},\"label\":\"m\",\"keys\":[\"north\"]}\n\
         mousing: [\"kind\",\"label\",\"north\"] \
         {\"kind\":\"Rope\",\"label\":\"n\",\"keys\":[\"kind\",\"label\",\"north\"]}\n\
         shoring: [\"label\",\"north\"] {\"label\":\"n\",\"keys\":[\"label\",\"north\"]}\n"
    );
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
    assert_eq!(printed, expected);
}

/// An externally tagged enum flattened beside a member of its holder's own is read from the first
/// entry under a variant's name, after keys no member reads and after the member's own key, and
/// refused where no entry is under one, as serde reads it. The holder is written back as serde
/// writes it back.
#[test]
fn a_flattened_enum_beside_a_member_reads_in_dart_the_entry_serde_reads() {
    let battened = BATTENING.iter().fold(String::new(), |mut acc, payload| {
        let read = read_of(payload, |battening: &Battening| {
            format!(
                "{} {} {}",
                written(&battening.hatching),
                battening.note,
                written(battening)
            )
        });
        writeln!(acc, "battening: {read}").unwrap();
        acc
    });
    assert_eq!(
        battened,
        "battening: {\"Bay\":{\"label\":\"n\"}} a {\"Bay\":{\"label\":\"n\"},\"note\":\"a\"}\n\
         battening: {\"Bay\":{\"label\":\"n\"}} a {\"Bay\":{\"label\":\"n\"},\"note\":\"a\"}\n\
         battening: {\"Dock\":{\"label\":\"s\"}} a {\"Dock\":{\"label\":\"s\"},\"note\":\"a\"}\n\
         battening: refused\n"
    );
    let Some(printed) = printed(
        "flattened-beside-a-member",
        &[
            hatching_dart::dart_definition(),
            battening_dart::dart_definition(),
        ],
        BESIDE_A_MEMBER,
    ) else {
        return;
    };
    assert_eq!(printed, battened);
}

/// An optional externally tagged enum flattened beside a member of its holder's own is read as no
/// value where no entry is under a variant's name, and from the first entry under one, as serde
/// reads it. An entry that does not read is refused, as the Zod schema refuses it, where serde
/// reads no value and drops the entry. The holder is written back as serde writes it back.
#[test]
fn an_optional_flattened_enum_beside_a_member_reads_in_dart_no_value_where_no_entry_is() {
    let caulked = CAULKING.map(|payload| {
        read_of(payload, |caulking: &Caulking| {
            format!(
                "{} {} {}",
                written(&caulking.hatching),
                caulking.note,
                written(caulking)
            )
        })
    });
    assert_eq!(
        caulked,
        [
            r#"null a {"note":"a"}"#,
            r#"null a {"note":"a"}"#,
            r#"{"Bay":{"label":"n"}} a {"Bay":{"label":"n"},"note":"a"}"#,
            r#"null a {"note":"a"}"#,
        ],
        "serde reads no value where no entry reads"
    );
    let mut expected = caulked[..3].iter().fold(String::new(), |mut acc, row| {
        writeln!(acc, "caulking: {row}").unwrap();
        acc
    });
    expected.push_str("caulking: refused\n");
    let Some(printed) = printed(
        "flattened-optional-beside-a-member",
        &[
            hatching_dart::dart_definition(),
            caulking_dart::dart_definition(),
        ],
        OPTIONAL_BESIDE_A_MEMBER,
    ) else {
        return;
    };
    assert_eq!(printed, expected, "the Dart reads beside serde's");
}

/// `rows` of the holder `named`, one line each.
fn lines(named: &str, rows: &[String]) -> String {
    rows.iter().fold(String::new(), |mut acc, row| {
        writeln!(acc, "{named}: {row}").unwrap();
        acc
    })
}

/// An optional map alone in its struct, or the one member of a variant under its name, under a
/// content key or untagged, is read as `null` wherever an entry does not read, a count of text,
/// `null` and a key that names no variant of the plain enum it is keyed by, and the payload is
/// read, as serde reads it. A map that holds no value is written as no entry, as serde writes it.
/// One that names a function is read from what it answers, and an entry there that does not read
/// refuses the payload, as serde refuses it.
#[test]
fn an_optional_map_reads_in_dart_no_value_where_an_entry_does_not_read() {
    let dunnage = DUNNAGE.map(|payload| {
        read_of(payload, |read: &Dunnage| {
            format!("{} {}", written(&read.counts), written(read))
        })
    });
    let steeving = STEEVING.map(|payload| {
        read_of(payload, |read: &Steeving| {
            let Steeving::Open { counts } = read;
            format!("{} {}", written(counts), written(read))
        })
    });
    let freeboard = FREEBOARD.map(|payload| {
        read_of(payload, |read: &Freeboard| {
            let Freeboard::Open { counts } = read;
            format!("{} {}", written(counts), written(read))
        })
    });
    let scuppering = DUNNAGE.map(|payload| {
        read_of(payload, |read: &Scuppering| {
            let Scuppering::Open { counts } = read;
            format!("{} {}", written(counts), written(read))
        })
    });
    let bilging = BILGING.map(|payload| {
        read_of(payload, |read: &Bilging| {
            format!("{} {}", written(&read.counts), written(read))
        })
    });
    let hawsehole = DUNNAGE.map(|payload| {
        read_of(payload, |read: &Hawsehole| {
            format!("{} {}", written(&read.counts), written(read))
        })
    });
    let expected = format!(
        "{}{}{}{}{}{}",
        lines("dunnage", &dunnage),
        lines("steeving", &steeving),
        lines("freeboard", &freeboard),
        lines("scuppering", &scuppering),
        lines("bilging", &bilging),
        lines("hawsehole", &hawsehole),
    );
    assert_eq!(
        expected,
        "dunnage: {} {}\n\
         dunnage: {\"north\":2} {\"north\":2}\n\
         dunnage: null {}\n\
         dunnage: null {}\n\
         dunnage: null {}\n\
         steeving: {} {\"Open\":{}}\n\
         steeving: {\"north\":2} {\"Open\":{\"north\":2}}\n\
         steeving: null {\"Open\":{}}\n\
         freeboard: {} {\"kind\":\"Open\",\"held\":{}}\n\
         freeboard: {\"north\":2} {\"kind\":\"Open\",\"held\":{\"north\":2}}\n\
         freeboard: null {\"kind\":\"Open\",\"held\":{}}\n\
         scuppering: {} {}\n\
         scuppering: {\"north\":2} {\"north\":2}\n\
         scuppering: null {}\n\
         scuppering: null {}\n\
         scuppering: null {}\n\
         bilging: {\"North\":2} {\"North\":2}\n\
         bilging: null {}\n\
         bilging: null {}\n\
         hawsehole: {} {}\n\
         hawsehole: {\"north\":2} {\"north\":2}\n\
         hawsehole: refused\n\
         hawsehole: refused\n\
         hawsehole: refused\n",
        "serde no longer reads each payload as it did"
    );
    let Some(printed) = printed(
        "flattened-optional-maps",
        &[
            dunnage_dart::dart_definition(),
            steeving_dart::dart_definition(),
            freeboard_dart::dart_definition(),
            scuppering_dart::dart_definition(),
            garboard_dart::dart_definition(),
            bilging_dart::dart_definition(),
            hawsehole_dart::dart_definition(),
        ],
        OPTIONAL_MAPS,
    ) else {
        return;
    };
    assert_eq!(printed, expected, "the Dart reads beside serde's");
}

/// An optional map of models is read as `null` where a model does not read, as serde reads it, and
/// a model that breaks its rule is read with no judgement and refused with its holder, in serde's
/// words.
#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
#[test]
fn an_optional_map_of_models_reads_in_dart_no_value_where_a_model_does_not_read() {
    let expected = LIMBER.iter().fold(String::new(), |mut acc, payload| {
        let told = serde_json::from_str::<Limber>(payload).map_or_else(
            |refused| refused.to_string(),
            |read| format!("{} {}", written(&read.notches), written(&read)),
        );
        writeln!(acc, "limber: {told}").unwrap();
        acc
    });
    assert_eq!(
        expected,
        "limber: {\"north\":{\"mark\":\"x\"}} {\"north\":{\"mark\":\"x\"}}\n\
         limber: 'north.mark': too short: minimum length is 1, got 0\n\
         limber: null {}\n",
        "serde no longer reads each payload as it did"
    );
    let source = module(
        &[
            scantling_dart::dart_definition(),
            limber_dart::dart_definition(),
        ],
        &[],
        &OPTIONAL_MAP_OF_MODELS.replace("LIMBER", &listed(&LIMBER)),
    );
    let Some((ran, printed)) = dart("flattened-optional-map-of-models", &[], &source) else {
        return;
    };
    assert!(ran, "{printed}");
    assert_eq!(printed, expected, "the Dart reads beside serde's");
}
