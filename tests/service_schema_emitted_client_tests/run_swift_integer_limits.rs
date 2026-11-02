//! Each limit of [`ROWS`] held by the emitted Swift types compiled with warnings as errors:
//! `validate()` of the value read with no judgement, and the read that refuses with what
//! `validate()` answers.

#![cfg(feature = "swift")]

use core::fmt::Write as _;

use super::integer_limit_models::{
    REPORT, ROWS, limit_ceiling_swift, limit_ends_swift, limit_floor_swift, limit_forms_swift,
    limit_mixed_swift,
};
use super::runtime::judged;

/// Names the runtime to run, for a machine that has one somewhere other than `PATH`.
const RUNTIME_VAR: &str = "TIXSCHEMA_SWIFT";

/// What prints a row: `passes` or each message of `validate()`, and a line more where the read
/// does not answer what `validate()` answered.
const REPORTER: &str = r#"
protocol ReadUnjudged: Decodable {
  init(swiftSchemaUnjudged decoder: Decoder) throws
  func validate() -> [String]
}

struct Unjudged<Held: ReadUnjudged>: Decodable {
  let held: Held
  init(from decoder: Decoder) throws { held = try Held(swiftSchemaUnjudged: decoder) }
}

extension LimitCeiling: ReadUnjudged {}
extension LimitFloor: ReadUnjudged {}
extension LimitForms: ReadUnjudged {}
extension LimitMixed: ReadUnjudged {}
extension LimitEnds: ReadUnjudged {}

func report<Read: ReadUnjudged>(_ name: String, _ type: Read.Type, _ payload: String) throws {
  let data = Data(payload.utf8)
  let answered = try JSONDecoder().decode(Unjudged<Read>.self, from: data).held.validate()
  if answered.isEmpty {
    print("\(name): passes")
  } else {
    print("\(name): refused")
    for message in answered { print("    \(message)") }
  }
  do {
    _ = try JSONDecoder().decode(type, from: data)
    if !answered.isEmpty { print("    the read passed it") }
  } catch DecodingError.dataCorrupted(let context) {
    if context.debugDescription != answered.joined(separator: "; ") {
      print("    the read said \(context.debugDescription)")
    }
  }
}
"#;

#[test]
fn each_limit_holds_in_swift_as_it_is_written() {
    let mut main = String::new();
    for (name, read_as, json) in ROWS {
        let _ = writeln!(main, "try report(\"{name}\", {read_as}.self, #\"{json}\"#)");
    }
    let source = [
        "import Foundation".to_owned(),
        limit_ceiling_swift::swift_definition(),
        limit_floor_swift::swift_definition(),
        limit_forms_swift::swift_definition(),
        limit_mixed_swift::swift_definition(),
        limit_ends_swift::swift_definition(),
        REPORTER.to_owned(),
        main,
    ]
    .join("\n\n");
    let Some(run) = judged(
        "swift-integer-limits",
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
    assert_eq!(run.stdout, REPORT);
}
