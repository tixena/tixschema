//! A plain enum that names a check, read on its own and held by another type, by the emitted
//! Swift types compiled with warnings as errors: each payload of
//! [`plain_enum_check_rows`](super::plain_enum_check_rows) prints what the serde read prints.

#![cfg(feature = "swift")]

use super::plain_enum_check_rows::{
    appeal_swift, bench_swift, expected, hearing_swift, rows, ruling_swift, verdict_swift,
};
use super::run_swift_read::normalized;
use super::runtime::judged;

const RUNTIME_VAR: &str = "TIXSCHEMA_SWIFT";

const UPHELD: &str = r#"
func tidied(_ payload: Any) -> Any {
  if let text = payload as? String { return text.trimmingCharacters(in: .whitespaces) }
  return payload
}

func allowed(_ value: Ruling) -> [String] {
  if case .denied = value { return ["a ruling is allowed"] }
  return []
}

func affirmed(_ value: Verdict) -> [String] {
  if case .overturned = value { return ["a verdict is upheld"] }
  return []
}
"#;

const HARNESS: &str = r#"
func row<Read: Codable>(_ name: String, _ type: Read.Type, _ payload: String, _ detail: Bool) {
  do {
    let held = try JSONDecoder().decode(type, from: Data(payload.utf8))
    let encoder = JSONEncoder()
    encoder.outputFormatting = [.sortedKeys]
    print("\(name): read \(String(data: try encoder.encode(held), encoding: .utf8)!)")
  } catch DecodingError.dataCorrupted(let context) {
    print("\(name): refused")
    if detail {
      for line in context.debugDescription.components(separatedBy: "; ") { print("    \(line)") }
    }
  } catch {
    print("\(name): refused")
  }
}
"#;

#[test]
fn a_plain_enum_with_a_check_reads_in_swift_as_serde_reads_it() {
    let rows = rows();
    let calls: Vec<String> = rows
        .iter()
        .map(|row| {
            format!(
                "row(\"{}\", {}.self, #\"{}\"#, {})",
                row.name, row.read_as, row.payload, row.detail
            )
        })
        .collect();
    let source = [
        "import Foundation".to_owned(),
        verdict_swift::swift_definition(),
        bench_swift::swift_definition(),
        appeal_swift::swift_definition(),
        hearing_swift::swift_definition(),
        ruling_swift::swift_definition(),
        UPHELD.to_owned(),
        HARNESS.to_owned(),
        calls.join("\n"),
    ]
    .join("\n\n");
    let Some(run) = judged(
        "swift-plain-enum-reads",
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
    assert_eq!(normalized(&run.stdout), normalized(&expected(&rows)));
}
