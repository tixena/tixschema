//! A `DateTime<Tz>` read and written back by the emitted Swift types compiled with warnings as
//! errors against serde: a date of whole seconds, of milliseconds, in a list, as a map's key and
//! value, in a tuple's slot and in a variant's slot, and an optional date under `as_number`.

#![cfg(all(feature = "swift", feature = "chrono"))]

use core::fmt::Write as _;

use super::dates_models::{
    SAME_AS_SERDE, landing_swift, nudge_swift, positions_swift, serde_back, stamped_swift,
};
use super::run_swift_read::normalized;
use super::runtime::judged;

/// Names the runtime to run, for a machine that has one somewhere other than `PATH`.
const RUNTIME_VAR: &str = "TIXSCHEMA_SWIFT";

/// What a run reads a payload with, and prints the answer with: the value written back, or
/// `refused`.
const HARNESS: &str = r#"
func row<Read: Codable>(_ name: String, _ type: Read.Type, _ payload: String) {
  do {
    let held = try JSONDecoder().decode(type, from: Data(payload.utf8))
    let encoder = JSONEncoder()
    encoder.outputFormatting = [.sortedKeys, .withoutEscapingSlashes]
    print("\(name): read \(String(data: try encoder.encode(held), encoding: .utf8)!)")
  } catch {
    print("\(name): refused")
  }
}
"#;

/// Each payload is read and written back by the Swift types as serde reads and writes it.
#[test]
fn a_date_in_every_position_is_read_and_written_back_by_swift_as_serde_writes_it() {
    let mut calls = String::new();
    let mut expected = String::new();
    for (name, model, payload) in SAME_AS_SERDE {
        let _ = writeln!(calls, "row(\"{name}\", {model}.self, #\"{payload}\"#)");
        let _ = writeln!(expected, "{name}: read {}", serde_back(model, payload));
    }
    let source = [
        "import Foundation".to_owned(),
        landing_swift::swift_definition(),
        nudge_swift::swift_definition(),
        positions_swift::swift_definition(),
        stamped_swift::swift_definition(),
        HARNESS.to_owned(),
        calls,
    ]
    .join("\n\n");
    let Some(run) = judged(
        "swift-dates",
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
    assert_eq!(normalized(&run.stdout), normalized(&expected));
}
