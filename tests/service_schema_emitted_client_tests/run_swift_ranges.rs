//! The four std ranges read and written by the emitted Swift types, compiled with warnings as
//! errors: each payload prints what the serde read prints of it, a range of a model or of a brand
//! judged by `validate()` in the words Rust's `validate()` writes.

#![cfg(feature = "swift")]

use super::range_models::{
    atlas_swift, compass_swift, course_swift, detour_swift, excursion_swift, frontier_swift,
    itinerary_swift, milestone_swift, positioned_swift, signposted_swift, spanned_swift,
    stride_swift, waymark_swift, waypost_swift,
};
use super::range_rows::{declares_a_parameter, expected, rows};
use super::run_swift_read::normalized;
use super::runtime::judged;

/// Names the runtime to run, for a machine that has one somewhere other than `PATH`.
const RUNTIME_VAR: &str = "TIXSCHEMA_SWIFT";

/// What a run reads a payload with, and prints the answer with: the value written back, or
/// `refused`, with each message of what `validate()` refused.
const HARNESS: &str = r#"
func row<Read: Codable>(_ name: String, _ type: Read.Type, _ payload: String) {
  do {
    let held = try JSONDecoder().decode(type, from: Data(payload.utf8))
    let encoder = JSONEncoder()
    encoder.outputFormatting = [.sortedKeys]
    print("\(name): read \(String(data: try encoder.encode(held), encoding: .utf8)!)")
  } catch DecodingError.dataCorrupted(let context) {
    print("\(name): refused")
    for line in context.debugDescription.components(separatedBy: "; ") { print("    \(line)") }
  } catch {
    print("\(name): refused")
  }
}
"#;

/// `{"start":8,"end":13}`, `{"start":8}` and `{"end":13}` read into the struct of each range and
/// written back as they were, in every position a range is held; a bound left out or of another
/// type refused; and each broken bound reported under its key.
#[test]
fn each_range_reads_and_writes_in_swift_as_serde_does() {
    let rows = rows();
    let expected = expected(&rows);
    assert!(
        expected.contains("    'span.start': too short: minimum length is 2, got 1"),
        "{expected}"
    );
    let calls: Vec<String> = rows
        .iter()
        .map(|row| {
            let filled = if declares_a_parameter(row.read_as) {
                "<UInt32>"
            } else {
                ""
            };
            format!(
                "row(\"{}\", {}{filled}.self, #\"{}\"#)",
                row.name, row.read_as, row.json
            )
        })
        .collect();
    let source = [
        "import Foundation".to_owned(),
        milestone_swift::swift_definition(),
        waymark_swift::swift_definition(),
        itinerary_swift::swift_definition(),
        spanned_swift::swift_definition(),
        positioned_swift::swift_definition(),
        detour_swift::swift_definition(),
        course_swift::swift_definition(),
        compass_swift::swift_definition(),
        atlas_swift::swift_definition(),
        excursion_swift::swift_definition(),
        waypost_swift::swift_definition(),
        stride_swift::swift_definition(),
        signposted_swift::swift_definition(),
        frontier_swift::swift_definition(),
        #[cfg(feature = "chrono")]
        super::range_models::rostered_swift::swift_definition(),
        HARNESS.to_owned(),
        calls.join("\n"),
    ]
    .join("\n\n");
    let Some(run) = judged(
        "swift-ranges",
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
