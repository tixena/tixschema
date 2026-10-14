//! A key under `#[serde(default)]` the payload leaves out, read by the emitted Dart types under
//! the Dart VM: each payload of the shared table prints what the serde read prints of it.

#![cfg(feature = "dart")]

use core::fmt::Write as _;

use super::run_dart_validate::{dart, module};
use super::serde_default_models::{
    Bollard, CASES, Harbour, Marina, PREPROCESSED, PREPROCESSED_REPORT, REPORT, REQUIRED,
    REQUIRED_REPORT, VARIANT_CASES, VARIANT_REPORT, adjacent_berthing_dart, basin_dart,
    berthing_dart, bollard_dart, config_dart, external_berthing_dart, filled_cases, filled_expected,
    filled_report, fleet_dart, gangway_dart, harbour_dart, jetty_dart, link_dart, logbook_dart,
    manifest_dart, marina_dart, pontoon_dart, preprocessed_values, quay_dart, read_values,
    untagged_berthing_dart, unwritten_defaults, wharf_dart,
};
#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
use super::serde_default_models::{report, variant_report};
#[cfg(feature = "indexmap")]
use super::serde_default_models::{indexed_fleet_dart, indexed_logbook_dart};

/// What prints a row as every side prints it: its name and the value read, written back, or each
/// message of the refusal on a line of its own.
const ROW: &str = "
void row(String name, Object? Function() written) {
  try {
    print('$name: read ${jsonEncode(written())}');
  } on FormatException catch (refused) {
    print('$name: refused');
    for (final violation in refused.message.split('; ')) {
      print('    $violation');
    }
  }
}
";

/// What prints a row of a key the type requires: whether the read answered or threw.
const REQUIRES: &str = "
void requires(String name, Object? Function() read) {
  try {
    read();
    print('$name: read');
  } catch (_) {
    print('$name: refused');
  }
}
";

/// The structs whose codec casts a key of a map the analyzer already knows to be a string: run,
/// and left out of what is analyzed.
const MAPS: [&str; 2] = ["IndexedLogbook", "Logbook"];

/// The function `Bollard` names, as a consumer writes it in Dart.
const TRIMMED_DOCK: &str =
    "dynamic trimmedDock(dynamic payload) => payload is String ? payload.trim() : payload;";

/// The emitted types of the table, the row printer, and a `main` that reads each payload with
/// the `fromJson` of the type the table names, then each payload of `Bollard`.
fn source() -> String {
    let bollards = PREPROCESSED
        .iter()
        .map(|&(name, payload)| (name, "Bollard", payload));
    let rows = CASES.iter().copied().chain(bollards).fold(
        String::new(),
        |mut written, (name, read_as, payload)| {
            let _ = writeln!(
                written,
                "  row('{}', () => {read_as}.fromJson(jsonDecode(r'{payload}')).toJson());",
                name.replace('\'', "\\'")
            );
            written
        },
    );
    module(
        &[
            gangway_dart::dart_definition(),
            wharf_dart::dart_definition(),
            jetty_dart::dart_definition(),
            pontoon_dart::dart_definition(),
            bollard_dart::dart_definition(),
        ],
        &[ROW, TRIMMED_DOCK],
        &format!("void main() {{\n{rows}}}\n"),
    )
}

/// Every row of the table: a key left out is read as the default serde reads, a key held as it
/// was, and a default that breaks its bound is refused with its member named. No function is
/// called for a default.
#[test]
fn each_payload_reads_in_dart_as_the_serde_read_reads_it() {
    for ((_, payload), read) in PREPROCESSED.iter().zip(preprocessed_values()) {
        assert_eq!(serde_json::from_str::<Bollard>(payload).unwrap(), read);
    }
    #[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
    assert_eq!(report(), REPORT, "what the serde read answers");
    let (every_default, _) = read_values();
    assert!(REPORT.contains(&format!(
        "every default at once: read {}\n",
        serde_json::to_string(&every_default).unwrap()
    )));
    let Some((ran, printed)) = dart("serde-default", &[], &source()) else {
        return;
    };
    assert!(ran, "{printed}");
    assert_eq!(printed, format!("{REPORT}{PREPROCESSED_REPORT}"));
}

/// The analyzer passes the emitted read of a default.
#[test]
fn the_read_of_a_default_has_no_issue() {
    let Some((clean, said)) = dart("serde-default-analyze", &["analyze"], &source()) else {
        return;
    };
    assert!(clean, "{said}");
    assert!(said.contains("No issues found!"), "{said}");
}

/// A default with no JSON keeps its key required: `Marina` derives no `Serialize`, and each
/// member of `Harbour` names its parameter outside a list read as empty.
#[test]
fn a_default_with_no_json_keeps_its_key_required() {
    let (read_marina, _) = unwritten_defaults();
    assert_eq!(
        serde_json::from_str::<Marina>(r#"{"label":"m"}"#).unwrap(),
        read_marina
    );
    let marina = marina_dart::dart_definition();
    assert!(
        marina.contains("Marina(dock: json['dock'] as String,label: json['label'] as String,)"),
        "{marina}"
    );
    let harbour = harbour_dart::dart_definition();
    assert!(
        harbour.contains(
            "Harbour(berths: (json['berths'] as List<dynamic>).map((e) => \
             mooredFromJson(e)).toList(),flagship: mooredFromJson(json['flagship']),slips: \
             (json['slips'] as List<dynamic>).map((e) => mooredFromJson(e)).toList(),)"
        ),
        "{harbour}"
    );
}

/// A list of the parameter is read from the JSON of its default, `[]`, through the read it has,
/// with the converter of the parameter the read is handed.
#[test]
fn an_empty_collection_of_a_parameter_is_read_from_its_json() {
    let basin = basin_dart::dart_definition();
    assert!(
        basin.contains(
            "Basin(dock: json.containsKey('dock') ? (json['dock'] as String) : \
             (jsonDecode('\"\"') as String),moored: json.containsKey('moored') ? ((json['moored'] \
             as List<dynamic>).map((e) => mooredFromJson(e)).toList()) : ((jsonDecode('[]') as \
             List<dynamic>).map((e) => mooredFromJson(e)).toList()),)"
        ),
        "{basin}"
    );
}

/// The emitted types of the structs that declare a parameter, the row printers, and a `main`
/// that reads each payload at `String`, then each payload of `Harbour` that leaves out a key.
/// `analyzed` leaves out the types of [`MAPS`] and their rows.
fn filled_source(analyzed: bool) -> String {
    let kept = |read_as: &str| !(analyzed && MAPS.contains(&read_as));
    let filled = filled_cases()
        .into_iter()
        .filter(|&(_, read_as, _)| kept(read_as))
        .fold(String::new(), |mut written, (name, read_as, payload)| {
            let _ = writeln!(
                written,
                "  row('{name}', () => {read_as}<String>.fromJson(jsonDecode(r'{payload}'), (e) => \
                 e as String).toJson((e) => e));"
            );
            written
        });
    let required = REQUIRED
        .iter()
        .fold(String::new(), |mut written, (name, payload)| {
            let _ = writeln!(
                written,
                "  requires('{name}', () => Harbour<String>.fromJson(jsonDecode(r'{payload}'), (e) \
                 => e as String));"
            );
            written
        });
    let definitions: Vec<String> = [
        ("Link", link_dart::dart_definition()),
        ("Basin", basin_dart::dart_definition()),
        ("Config", config_dart::dart_definition()),
        ("Fleet", fleet_dart::dart_definition()),
        ("Logbook", logbook_dart::dart_definition()),
        ("Harbour", harbour_dart::dart_definition()),
        ("Manifest", manifest_dart::dart_definition()),
        #[cfg(feature = "indexmap")]
        ("IndexedFleet", indexed_fleet_dart::dart_definition()),
        #[cfg(feature = "indexmap")]
        ("IndexedLogbook", indexed_logbook_dart::dart_definition()),
    ]
    .into_iter()
    .filter(|&(declared, _)| kept(declared))
    .map(|(_, written)| written)
    .collect();
    module(
        &definitions,
        &[ROW, REQUIRES],
        &format!("void main() {{\n{filled}{required}}}\n"),
    )
}

/// Each payload of a struct that declares a parameter reads at `String` as serde reads it, and a
/// payload that leaves out a key `Harbour` requires is refused.
#[test]
fn each_payload_of_a_struct_that_declares_a_parameter_reads_in_dart_as_serde_reads_it() {
    assert_eq!(
        filled_report(),
        filled_expected(),
        "what the serde read answers"
    );
    let (_, harbours) = unwritten_defaults();
    for ((_, payload), harbour) in REQUIRED.iter().zip(harbours) {
        assert_eq!(
            serde_json::from_str::<Harbour<String>>(payload).unwrap(),
            harbour
        );
    }
    let Some((ran, printed)) = dart("serde-default-filled", &[], &filled_source(false)) else {
        return;
    };
    assert!(ran, "{printed}");
    assert_eq!(printed, format!("{}{REQUIRED_REPORT}", filled_expected()));
}

/// The analyzer passes the emitted read of an empty collection of a parameter.
#[test]
fn the_read_of_an_empty_collection_of_a_parameter_has_no_issue() {
    let Some((clean, said)) = dart(
        "serde-default-filled-analyze",
        &["analyze"],
        &filled_source(true),
    ) else {
        return;
    };
    assert!(clean, "{said}");
    assert!(said.contains("No issues found!"), "{said}");
}

/// The emitted enums whose variants hold a member read as its default, under each tagging, the
/// struct that holds the untagged one, the row printer, and a `main` that reads each payload with
/// the `fromJson` of the type the table names.
fn variant_source() -> String {
    let rows = VARIANT_CASES.iter().fold(
        String::new(),
        |mut written, &(name, read_as, payload)| {
            let _ = writeln!(
                written,
                "  row('{name}', () => {read_as}.fromJson(jsonDecode(r'{payload}')).toJson());"
            );
            written
        },
    );
    module(
        &[
            berthing_dart::dart_definition(),
            external_berthing_dart::dart_definition(),
            adjacent_berthing_dart::dart_definition(),
            untagged_berthing_dart::dart_definition(),
            quay_dart::dart_definition(),
        ],
        &[ROW],
        &format!("void main() {{\n{rows}}}\n"),
    )
}

/// Every row of the variant table, under each tagging: a member of a variant left out is read as
/// the default serde reads, a member held as it was, and a default that breaks its bound is
/// refused with its member named.
#[test]
fn each_variant_payload_reads_in_dart_as_the_serde_read_reads_it() {
    #[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
    assert_eq!(variant_report(), VARIANT_REPORT, "what the serde read answers");
    let Some((ran, printed)) = dart("serde-default-variants", &[], &variant_source()) else {
        return;
    };
    assert!(ran, "{printed}");
    assert_eq!(printed, VARIANT_REPORT);
}

/// The analyzer passes the emitted read of a default of a member of a variant.
#[test]
fn the_read_of_a_default_of_a_variant_has_no_issue() {
    let Some((clean, said)) = dart(
        "serde-default-variants-analyze",
        &["analyze"],
        &variant_source(),
    ) else {
        return;
    };
    assert!(clean, "{said}");
    assert!(said.contains("No issues found!"), "{said}");
}
