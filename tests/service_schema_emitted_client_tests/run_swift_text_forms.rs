//! A `char` and a chrono date, time and date and time read by the emitted Swift types, compiled
//! with warnings as errors: each payload of [`TEXT_FORMS`] is read, or refused with
//! `DecodingError.dataCorrupted`, where the table states.

#![cfg(all(feature = "swift", feature = "chrono"))]

use core::fmt::Write as _;

use super::runtime::judged;
use super::text_forms_models::{
    TEXT_FORMS, chronometer_swift, day_lines_swift, day_log_swift, instant_lines_swift,
    instant_log_swift, letter_lines_swift, letter_log_swift, stamp_lines_swift, stamp_log_swift,
    time_lines_swift, time_log_swift,
};

/// Names the runtime to run, for a machine that has one somewhere other than `PATH`.
const RUNTIME_VAR: &str = "TIXSCHEMA_SWIFT";

/// What prints a row: `read`, or the description of what the read throws.
const TOLD: &str = r#"
func told<Read: Decodable>(_ at: Int, _ type: Read.Type, _ payload: String) {
  do {
    _ = try JSONDecoder().decode(type, from: Data(payload.utf8))
    print("\(at): read")
  } catch DecodingError.dataCorrupted(let context) {
    print("\(at): \(context.debugDescription)")
  } catch {
    print("\(at): \(error)")
  }
}
"#;

/// Each payload is read, or refused, as [`TEXT_FORMS`] states: what Zod refuses, and a date and
/// time with its zone that a Swift `Date` does not read.
#[test]
fn each_text_of_one_form_reads_in_swift_where_the_table_states() {
    let mut expected = String::new();
    let mut calls = String::new();
    for (at, row) in TEXT_FORMS.iter().enumerate() {
        let _ = writeln!(expected, "{at}: {}", row.swift);
        let _ = writeln!(
            calls,
            "told({at}, {}.self, #\"{}\"#)",
            row.read_as, row.json
        );
    }
    let source = [
        "import Foundation".to_owned(),
        chronometer_swift::swift_definition(),
        letter_log_swift::swift_definition(),
        letter_lines_swift::swift_definition(),
        day_log_swift::swift_definition(),
        day_lines_swift::swift_definition(),
        time_log_swift::swift_definition(),
        time_lines_swift::swift_definition(),
        stamp_log_swift::swift_definition(),
        stamp_lines_swift::swift_definition(),
        instant_log_swift::swift_definition(),
        instant_lines_swift::swift_definition(),
        TOLD.to_owned(),
        calls,
    ]
    .join("\n\n");
    let Some(run) = judged(
        "swift-text-forms",
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
        "the Swift read is not what the table states"
    );
}
