//! The std `Result` read and written by the emitted Swift types, compiled with warnings as
//! errors: each payload prints what the serde read prints of it, a `Result` of a model or of a
//! brand judged by `validate()` in the words Rust's `validate()` writes.

#![cfg(feature = "swift")]

use super::result_models::{
    attempted_swift, ballot_swift, bureau_swift, errand_swift, errata_swift, jacket_swift,
    mailbag_swift, masthead_swift, missive_swift, notebook_swift, postbox_swift, reply_swift,
    ruling_swift, settlement_swift, summary_swift, turn_swift, unparsed_swift,
};
use super::result_rows::{declares_a_parameter, expected, rows};
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

/// `{"Ok": T}` and `{"Err": E}` read into the enum of each `Result` and written back as they were,
/// in every position a `Result` is held, `null` under an arm of `()`; an object of both keys, of
/// neither and of another key refused; and each broken bound in an arm reported under the arm's
/// key.
#[test]
fn each_result_reads_and_writes_in_swift_as_serde_does() {
    let rows = rows();
    let expected = expected(&rows);
    assert!(
        expected.contains("    'frontmatter.Ok.title': too short: minimum length is 1, got 0"),
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
        masthead_swift::swift_definition(),
        unparsed_swift::swift_definition(),
        ballot_swift::swift_definition(),
        mailbag_swift::swift_definition(),
        reply_swift::swift_definition(),
        missive_swift::swift_definition(),
        postbox_swift::swift_definition(),
        ruling_swift::swift_definition(),
        settlement_swift::swift_definition(),
        errand_swift::swift_definition(),
        attempted_swift::swift_definition(),
        turn_swift::swift_definition(),
        summary_swift::swift_definition(),
        bureau_swift::swift_definition(),
        jacket_swift::swift_definition(),
        notebook_swift::swift_definition(),
        errata_swift::swift_definition(),
        HARNESS.to_owned(),
        calls.join("\n"),
    ]
    .join("\n\n");
    let Some(run) = judged(
        "swift-results",
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
