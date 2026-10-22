//! A `char` and a chrono date, time and date and time read by the emitted Dart types under the
//! Dart SDK: each payload of [`TEXT_FORMS`] is read, or refused with `FormatException`, where the
//! table states.

#![cfg(all(feature = "dart", feature = "chrono"))]

use core::fmt::Write as _;

use super::run_dart_validate::{dart, module};
use super::text_forms_models::{
    TEXT_FORMS, chronometer_dart, day_lines_dart, day_log_dart, instant_lines_dart,
    instant_log_dart, letter_lines_dart, letter_log_dart, stamp_lines_dart, stamp_log_dart,
    time_lines_dart, time_log_dart,
};

/// What prints a row: `read`, or the message of what the read throws.
const TOLD: &str = "
void told(String at, Object? Function() reading) {
  try {
    reading();
    print('$at: read');
  } on FormatException catch (refusal) {
    print('$at: ${refusal.message}');
  } catch (refusal) {
    print('$at: $refusal');
  }
}
";

/// Each payload is read, or refused, as [`TEXT_FORMS`] states: what Zod refuses, and a date and
/// time with its zone that a Dart `DateTime` does not read.
#[test]
fn each_text_of_one_form_reads_in_dart_where_the_table_states() {
    let mut expected = String::new();
    let mut main = String::from("void main() {\n");
    for (at, row) in TEXT_FORMS.iter().enumerate() {
        let _ = writeln!(expected, "{at}: {}", row.dart);
        let _ = writeln!(
            main,
            "  told('{at}', () => {}.fromJson(jsonDecode(r'{}')));",
            row.read_as, row.json
        );
    }
    main.push_str("}\n");
    let definitions = [
        chronometer_dart::dart_definition(),
        letter_log_dart::dart_definition(),
        letter_lines_dart::dart_definition(),
        day_log_dart::dart_definition(),
        day_lines_dart::dart_definition(),
        time_log_dart::dart_definition(),
        time_lines_dart::dart_definition(),
        stamp_log_dart::dart_definition(),
        stamp_lines_dart::dart_definition(),
        instant_log_dart::dart_definition(),
        instant_lines_dart::dart_definition(),
    ];
    let Some((ran, printed)) = dart("text-forms", &[], &module(&definitions, &[TOLD], &main))
    else {
        return;
    };
    assert!(ran, "{printed}");
    assert_eq!(
        printed, expected,
        "the Dart read is not what the table states"
    );
}
