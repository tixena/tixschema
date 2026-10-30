//! A flattened choice, the choice it holds and an internally tagged struct variant, read by the
//! emitted Dart types under the Dart VM: each payload of the shared rows is read as serde reads
//! it, and each value of the checked rows is held to the messages Rust's `validate()` answers.

#![cfg(feature = "dart")]

use core::fmt::Write as _;

use super::flattened_zod_models::{
    CHECKED_REPORT, ROWS, berthage_dart, block_dart, booking_dart, cargo_dart, checked_cases,
    checked_report, deck_dart, gap_dart, hatch_dart, hold_dart, mole_dart, moorage_dart,
    paragraph_dart, pause_dart, quote_dart, serde_report, slip_dart, span_dart, tally_dart,
    tenancy_dart, tethered_dart, wharf_dart, wharfage_dart,
};
use super::run_dart_validate::{analyzed, dart, module};

/// The functions the models name, as a consumer writes them in Dart.
const FUNCTIONS: &str = r#"
dynamic asGiven(dynamic payload) => payload;

List<String> aSpan(Span value) => switch (value) {
      SpanHours(:final hours) when hours == 0 => ["'hours': a span is not empty"],
      SpanDays(:final days) when days == '9' => ["'days': a span is under nine days"],
      _ => const [],
    };

List<String> notBlank(String value) => value.isEmpty ? const ['a day label is not blank'] : const [];
"#;

/// What prints a row: the value read and written back with its keys sorted, or `refused`, and
/// what prints a checked row.
const HARNESS: &str = "
dynamic sorted(dynamic value) {
  if (value is Map) {
    final keys = value.keys.map((key) => key as String).toList()..sort();
    return {for (final key in keys) key: sorted(value[key])};
  }
  if (value is List) return value.map(sorted).toList();
  return value;
}

void row(String name, Object? Function() read) {
  try {
    print('$name: read ${jsonEncode(sorted(read()))}');
  } catch (_) {
    print('$name: refused');
  }
}

void checked(String name, Object? Function() read) {
  try {
    read();
    print('$name: passes');
  } on FormatException catch (refused) {
    print('$name: refused');
    for (final violation in refused.message.split('; ')) {
      print('    $violation');
    }
  }
}
";

fn definitions() -> Vec<String> {
    vec![
        moorage_dart::dart_definition(),
        berthage_dart::dart_definition(),
        tethered_dart::dart_definition(),
        mole_dart::dart_definition(),
        wharf_dart::dart_definition(),
        slip_dart::dart_definition(),
        wharfage_dart::dart_definition(),
        hatch_dart::dart_definition(),
        deck_dart::dart_definition(),
        cargo_dart::dart_definition(),
        hold_dart::dart_definition(),
        tally_dart::dart_definition(),
        gap_dart::dart_definition(),
        pause_dart::dart_definition(),
        span_dart::dart_definition(),
        booking_dart::dart_definition(),
        tenancy_dart::dart_definition(),
        block_dart::dart_definition(),
        paragraph_dart::dart_definition(),
        quote_dart::dart_definition(),
    ]
}

fn source(main: &str) -> String {
    module(&definitions(), &[FUNCTIONS, HARNESS], main)
}

/// Each payload of the rows is read by Dart as serde reads it.
#[test]
fn a_flattened_choice_is_read_by_dart_as_serde_reads_it() {
    let mut main = String::from("void main() {\n");
    for row in ROWS {
        let _ = writeln!(
            main,
            "  row('{}', () => {}.fromJson(jsonDecode(r'{}')).toJson());",
            row.name, row.schema, row.payload
        );
    }
    main.push_str("}\n");
    analyzed("flattened-zod-read-analyze", &source(&main));
    let Some((ran, printed)) = dart("flattened-zod-read", &[], &source(&main)) else {
        return;
    };
    assert!(ran, "{printed}");
    assert_eq!(printed, serde_report(), "the Dart read is not serde's");
}

/// Each value of the checked rows is refused by Dart with the messages `validate()` answers.
#[test]
fn a_check_of_a_flattened_choice_answers_in_dart_what_validate_answers() {
    let cases = checked_cases();
    assert_eq!(checked_report(&cases), CHECKED_REPORT);
    let mut main = String::from("void main() {\n");
    for case in &cases {
        let _ = writeln!(
            main,
            "  checked('{}', () => {}.fromJson(jsonDecode(r'{}')));",
            case.name, case.schema, case.payload
        );
    }
    main.push_str("}\n");
    analyzed("flattened-zod-checks-analyze", &source(&main));
    let Some((ran, printed)) = dart("flattened-zod-checks", &[], &source(&main)) else {
        return;
    };
    assert!(ran, "{printed}");
    assert_eq!(printed, CHECKED_REPORT, "the Dart validate is not Rust's");
}
