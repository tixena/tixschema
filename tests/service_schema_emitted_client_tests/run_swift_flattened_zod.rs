//! A flattened choice, the choice it holds and an internally tagged struct variant, read by the
//! emitted Swift types compiled with warnings as errors: each payload of the shared rows is read
//! as serde reads it, and each value of the checked rows is held to the messages Rust's
//! `validate()` answers.

#![cfg(feature = "swift")]

use core::fmt::Write as _;

use super::flattened_zod_models::{
    CHECKED_REPORT, ROWS, berthage_swift, block_swift, booking_swift, checked_cases,
    checked_report, deck_swift, gap_swift, hatch_swift, mole_swift, moorage_swift, paragraph_swift,
    pause_swift, quote_swift, serde_report, slip_swift, span_swift, tenancy_swift, tethered_swift,
    wharf_swift, wharfage_swift,
};
use super::run_swift_read::normalized;
use super::runtime::judged;

/// Names the runtime to run, for a machine that has one somewhere other than `PATH`.
const RUNTIME_VAR: &str = "TIXSCHEMA_SWIFT";

/// The functions the models name, as a consumer writes them in Swift.
const FUNCTIONS: &str = r#"
func asGiven(_ payload: Any) -> Any {
  payload
}

func aSpan(_ value: Span) -> [String] {
  switch value {
  case .hours(let held) where held.hours == 0: return ["'hours': a span is not empty"]
  case .days(let held) where held.days == "9": return ["'days': a span is under nine days"]
  default: return []
  }
}

func notBlank(_ value: String) -> [String] {
  value.isEmpty ? ["a day label is not blank"] : []
}
"#;

/// What prints a row: the value read and written back with its keys sorted, or `refused`, and what
/// prints a checked row.
const HARNESS: &str = r#"
func row<Read: Codable>(_ name: String, _ type: Read.Type, _ payload: String) {
  do {
    let held = try JSONDecoder().decode(type, from: Data(payload.utf8))
    let encoder = JSONEncoder()
    encoder.outputFormatting = [.sortedKeys]
    print("\(name): read \(String(data: try encoder.encode(held), encoding: .utf8)!)")
  } catch {
    print("\(name): refused")
  }
}

func checked<Read: Codable>(_ name: String, _ type: Read.Type, _ payload: String) {
  do {
    _ = try JSONDecoder().decode(type, from: Data(payload.utf8))
    print("\(name): passes")
  } catch DecodingError.dataCorrupted(let context) {
    print("\(name): refused")
    for violation in context.debugDescription.components(separatedBy: "; ") { print("    \(violation)") }
  } catch {
    print("\(name): refused")
  }
}
"#;

fn source(calls: &str) -> String {
    [
        "import Foundation".to_owned(),
        moorage_swift::swift_definition(),
        berthage_swift::swift_definition(),
        tethered_swift::swift_definition(),
        mole_swift::swift_definition(),
        wharf_swift::swift_definition(),
        slip_swift::swift_definition(),
        wharfage_swift::swift_definition(),
        hatch_swift::swift_definition(),
        deck_swift::swift_definition(),
        gap_swift::swift_definition(),
        pause_swift::swift_definition(),
        span_swift::swift_definition(),
        booking_swift::swift_definition(),
        tenancy_swift::swift_definition(),
        block_swift::swift_definition(),
        paragraph_swift::swift_definition(),
        quote_swift::swift_definition(),
        FUNCTIONS.to_owned(),
        HARNESS.to_owned(),
        calls.to_owned(),
    ]
    .join("\n\n")
}

fn run(named: &str, calls: &str) -> Option<String> {
    let source = source(calls);
    let run = judged(
        named,
        RUNTIME_VAR,
        "swift",
        &["-warnings-as-errors"],
        "main.swift",
        &source,
    )?;
    assert!(
        run.succeeded,
        "`swift -warnings-as-errors main.swift` failed.\n--- stdout ---\n{}\n--- stderr ---\n{}\n--- source ---\n{source}",
        run.stdout, run.stderr
    );
    Some(normalized(&run.stdout))
}

/// Each payload of the rows is read by Swift as serde reads it.
#[test]
fn a_flattened_choice_is_read_by_swift_as_serde_reads_it() {
    let mut calls = String::new();
    for row in ROWS {
        let _ = writeln!(
            calls,
            "row(\"{}\", {}.self, #\"{}\"#)",
            row.name, row.schema, row.payload
        );
    }
    let Some(printed) = run("swift-flattened-zod-read", &calls) else {
        return;
    };
    assert_eq!(printed, normalized(&serde_report()));
}

/// Each value of the checked rows is refused by Swift with the messages `validate()` answers.
#[test]
fn a_check_of_a_flattened_choice_answers_in_swift_what_validate_answers() {
    let cases = checked_cases();
    assert_eq!(checked_report(&cases), CHECKED_REPORT);
    let mut calls = String::new();
    for case in &cases {
        let _ = writeln!(
            calls,
            "checked(\"{}\", {}.self, #\"{}\"#)",
            case.name, case.schema, case.payload
        );
    }
    let Some(printed) = run("swift-flattened-zod-checks", &calls) else {
        return;
    };
    assert_eq!(printed, normalized(CHECKED_REPORT));
}
