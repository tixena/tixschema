//! A member serde reads and never writes, read and written back by the emitted Swift types
//! compiled with warnings as errors, in a struct of its own and beside a flattened member, against
//! what serde reads and writes back.

#![cfg(feature = "swift")]

use super::runtime::judged;
use super::unwritten_models::{PAYLOAD, WRITTEN, braiding_swift, furling_swift, reefing_swift};

/// Names the runtime to run, for a machine that has one somewhere other than `PATH`.
const RUNTIME_VAR: &str = "TIXSCHEMA_SWIFT";

/// What prints each struct read from the payload: the member read, and the value written back.
const MAIN: &str = r##"
func written<Read: Encodable>(_ value: Read) -> String {
  let encoder = JSONEncoder()
  encoder.outputFormatting = [.sortedKeys]
  return String(data: try! encoder.encode(value), encoding: .utf8)!
}

let furling = try! JSONDecoder().decode(Furling.self, from: Data(#"PAYLOAD"#.utf8))
print("in a struct of its own: read \(furling.spare.map { "\($0)" } ?? "nil"), written \(written(furling))")
let reefing = try! JSONDecoder().decode(Reefing.self, from: Data(#"PAYLOAD"#.utf8))
print("beside a flattened member: read \(reefing.spare.map { "\($0)" } ?? "nil"), written \(written(reefing))")
"##;

/// Each struct reads the member and writes it back as serde does: never.
#[test]
fn a_member_serde_never_writes_is_read_and_not_written_in_swift() {
    let source = [
        "import Foundation".to_owned(),
        braiding_swift::swift_definition(),
        furling_swift::swift_definition(),
        reefing_swift::swift_definition(),
        MAIN.replace("PAYLOAD", PAYLOAD),
    ]
    .join("\n\n");
    let Some(run) = judged(
        "swift-unwritten",
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
        run.stdout,
        format!(
            "in a struct of its own: read 3, written {WRITTEN}\n\
             beside a flattened member: read 3, written {WRITTEN}\n"
        )
    );
}
