//! A `literal` read by the emitted Swift types compiled with warnings as errors, against serde:
//! each payload of the literal table is read and refused as serde reads and refuses it, each
//! violation in the sentence Rust's `validate()` reports.

#![cfg(all(feature = "swift", feature = "serde"))]

use core::fmt::Write as _;

use super::literal_models::{
    Answered, LITERAL_READS, adjacent_assay_swift, assay_swift, bullion_swift, hallmark_swift,
    ingot_swift, literal_report, names_each_violation, serde_read, tagged_assay_swift,
    touchmark_swift,
};
use super::runtime::judged;

/// Names the runtime to run, for a machine that has one somewhere other than `PATH`.
const RUNTIME_VAR: &str = "TIXSCHEMA_SWIFT";

/// What prints a row: its name and whether it is read, then each violation the read refused it
/// with, where `named` asks for them. An untagged enum refuses a payload no variant reads with
/// `typeMismatch`, and any other error is printed whole.
const TOLD: &str = r#"
func told<Read: Decodable>(_ name: String, _ named: Bool, _ type: Read.Type, _ payload: String) {
  do {
    _ = try JSONDecoder().decode(type, from: Data(payload.utf8))
    print("\(name): read")
  } catch DecodingError.dataCorrupted(let context) {
    print("\(name): refused")
    if named {
      for violation in context.debugDescription.components(separatedBy: "; ") {
        print("    \(violation)")
      }
    }
  } catch DecodingError.typeMismatch {
    print("\(name): refused")
  } catch {
    print("\(name): \(error)")
  }
}
"#;

/// Each payload is read and refused by the Swift read as serde reads and refuses it, under
/// `-warnings-as-errors`.
#[test]
fn a_literal_refuses_in_swift_what_serde_refuses() {
    let mut main = String::new();
    for row in &LITERAL_READS {
        assert_eq!(
            serde_read(row.held_by, row.payload).is_ok(),
            matches!(row.answered, Answered::Read),
            "serde no longer reads {} as the table says",
            row.name
        );
        let _ = writeln!(
            main,
            "told(\"{}\", {}, {}.self, #\"{}\"#)",
            row.name,
            names_each_violation(row),
            row.held_by,
            row.payload
        );
    }
    let source = [
        "import Foundation".to_owned(),
        adjacent_assay_swift::swift_definition(),
        assay_swift::swift_definition(),
        bullion_swift::swift_definition(),
        hallmark_swift::swift_definition(),
        ingot_swift::swift_definition(),
        tagged_assay_swift::swift_definition(),
        touchmark_swift::swift_definition(),
        TOLD.to_owned(),
        main,
    ]
    .join("\n\n");
    let Some(run) = judged(
        "swift-literals",
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
        literal_report(names_each_violation),
        "the Swift read is not serde's"
    );
}
