//! A key under `#[serde(default)]` the payload leaves out, read by the emitted Dart types under
//! the Dart VM: each payload of the shared table prints what the serde read prints of it.

#![cfg(feature = "dart")]

use core::fmt::Write as _;

use super::run_dart_validate::{dart, module};
#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
use super::serde_default_models::report;
use super::serde_default_models::{
    Basin, Bollard, CASES, Marina, PREPROCESSED, PREPROCESSED_REPORT, REPORT, basin_dart,
    bollard_dart, gangway_dart, jetty_dart, marina_dart, pontoon_dart, preprocessed_values,
    read_values, unwritten_defaults, wharf_dart,
};

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

/// A default with no JSON keeps its key required: `Marina` derives no `Serialize`, and the
/// `moored` of `Basin` names its parameter. The `dock` of `Basin` names none, and is read as its
/// default.
#[test]
fn a_default_with_no_json_keeps_its_key_required() {
    let (read_marina, read_basin) = unwritten_defaults();
    assert_eq!(
        serde_json::from_str::<Marina>(r#"{"label":"m"}"#).unwrap(),
        read_marina
    );
    assert_eq!(
        serde_json::from_str::<Basin<String>>(r#"{"moored":[]}"#).unwrap(),
        read_basin
    );
    let marina = marina_dart::dart_definition();
    assert!(
        marina.contains("Marina(dock: json['dock'] as String,label: json['label'] as String,)"),
        "{marina}"
    );
    let basin = basin_dart::dart_definition();
    assert!(
        basin.contains(
            "Basin(dock: json.containsKey('dock') ? (json['dock'] as String) : \
             (jsonDecode('\"\"') as String),moored: (json['moored'] as List<dynamic>)"
        ),
        "{basin}"
    );
}
