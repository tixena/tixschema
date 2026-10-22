//! An enum's variant serde neither writes nor reads is written by no emitted Swift type: each enum
//! reads the payload of every variant serde writes and writes it back, and refuses the payload of
//! one it skips, as serde does, under every tagging, compiled with warnings as errors.

#![cfg(feature = "swift")]

use super::runtime::judged;
use super::skipped_variant_models::{
    ROWS, berthage_swift, cartage_swift, dockage_swift, keelage_swift, lighterage_swift,
    pilotage_swift, tonnage_swift, unread_writes, wharfage_swift,
};

/// Names the runtime to run, for a machine that has one somewhere other than `PATH`.
const RUNTIME_VAR: &str = "TIXSCHEMA_SWIFT";

/// What reads a payload and answers it written back, or `refused`.
const SAID: &str = r#"
func said<Read: Codable>(_ type: Read.Type, _ json: String) -> String {
  guard let read = try? JSONDecoder().decode(Read.self, from: Data(json.utf8)) else {
    return "refused"
  }
  let encoder = JSONEncoder()
  encoder.outputFormatting = [.sortedKeys]
  return "read " + String(data: try! encoder.encode(read), encoding: .utf8)!
}
"#;

/// What prints each row serde and Zod agree on, read as its type and written back.
fn main_of_rows() -> String {
    ROWS.iter()
        .filter(|row| row.serde == row.zod)
        .map(|row| format!("print(said({}.self, #\"{}\"#))", row.read_as, row.json))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Each row serde reads is read and written back as it was, and each row serde refuses, of a
/// variant it neither writes nor reads, is refused.
#[test]
fn a_variant_serde_neither_writes_nor_reads_is_refused_in_swift() {
    assert!(
        unread_writes()
            .iter()
            .rev()
            .skip(1)
            .all(|said| said.ends_with("cannot be serialized")),
        "serde writes a variant it skips"
    );
    let source = [
        "import Foundation".to_owned(),
        berthage_swift::swift_definition(),
        wharfage_swift::swift_definition(),
        pilotage_swift::swift_definition(),
        lighterage_swift::swift_definition(),
        keelage_swift::swift_definition(),
        cartage_swift::swift_definition(),
        tonnage_swift::swift_definition(),
        dockage_swift::swift_definition(),
        SAID.to_owned(),
        main_of_rows(),
    ]
    .join("\n\n");
    let Some(run) = judged(
        "swift-skipped-variants",
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
    let rows: Vec<_> = ROWS.iter().filter(|row| row.serde == row.zod).collect();
    let lines: Vec<&str> = run.stdout.lines().collect();
    assert_eq!(lines.len(), rows.len(), "{}", run.stdout);
    for (row, line) in rows.iter().zip(lines) {
        let read = (row.read)(row.json);
        assert_eq!(read, row.serde, "serde's read of {}", row.json);
        if read {
            let written: serde_json::Value =
                serde_json::from_str(line.strip_prefix("read ").unwrap_or("null")).unwrap();
            assert_eq!(
                written,
                serde_json::from_str::<serde_json::Value>(row.json).unwrap(),
                "{} as {}: {line}",
                row.json,
                row.read_as
            );
        } else {
            assert_eq!(line, "refused", "{} as {}", row.json, row.read_as);
        }
    }
}
