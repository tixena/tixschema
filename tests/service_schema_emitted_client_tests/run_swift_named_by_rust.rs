//! The emitted Swift types compiled with warnings as errors, named as Rust names them: a field
//! written `Self`, a model declared below the model that holds it, and a variant whose own
//! `rename_all` names its members under every tagging. Each payload prints what the serde read
//! prints of it.

#![cfg(feature = "swift")]

use super::named_by_rust_models::{
    bough_swift, carriage_adjacent_swift, carriage_external_swift, carriage_swift,
    carriage_untagged_swift, fork_swift, hauler_swift, hitch_swift, hitched_trailer_swift,
    rope_swift, spur_swift, towed_trailer_swift,
};
use super::named_by_rust_rows::{
    BELOW_ROWS, CARRIAGE_ROWS, Row, SELF_ROWS, declares_a_parameter, expected, normalized,
};
use super::runtime::judged;

/// Names the runtime to run, for a machine that has one somewhere other than `PATH`.
const RUNTIME_VAR: &str = "TIXSCHEMA_SWIFT";

/// What a run reads a payload with, and prints the answer with: the value written back, or
/// `refused`, with each message of what the read refused where the row compares them.
const HARNESS: &str = r#"
func row<Read: Codable>(_ name: String, _ type: Read.Type, _ payload: String, _ judged: Bool) {
  do {
    let held = try JSONDecoder().decode(type, from: Data(payload.utf8))
    let encoder = JSONEncoder()
    encoder.outputFormatting = [.sortedKeys]
    print("\(name): read \(String(data: try encoder.encode(held), encoding: .utf8)!)")
  } catch DecodingError.dataCorrupted(let context) {
    print("\(name): refused")
    if judged {
      for line in context.debugDescription.components(separatedBy: "; ") { print("    \(line)") }
    }
  } catch {
    print("\(name): refused")
  }
}
"#;

/// Runs `rows` and compares what Swift prints with what the serde read prints.
fn compared(named: &str, rows: &[Row], definitions: Vec<String>) {
    let expected = expected(rows);
    let calls: Vec<String> = rows
        .iter()
        .map(|row| {
            let filled = if declares_a_parameter(row.read_as) {
                "<Int32>"
            } else {
                ""
            };
            format!(
                "row(\"{}\", {}{filled}.self, #\"{}\"#, {})",
                row.name, row.read_as, row.json, row.judged
            )
        })
        .collect();
    let mut source = vec!["import Foundation".to_owned()];
    source.extend(definitions);
    source.extend([HARNESS.to_owned(), calls.join("\n")]);
    let program = source.join("\n\n");
    let Some(run) = judged(
        named,
        RUNTIME_VAR,
        "swift",
        &["-warnings-as-errors"],
        "main.swift",
        &program,
    ) else {
        return;
    };
    assert!(
        run.succeeded,
        "`swift -warnings-as-errors main.swift` failed.\n--- stdout ---\n{}\n--- stderr ---\n{}\n--- source ---\n{program}",
        run.stdout, run.stderr
    );
    assert_eq!(normalized(&run.stdout), expected);
}

/// A field written `Self` names the enclosing type, in an `Option`, a `Box`, a map's value, a
/// variant's slot and a generic type, and `validate()` and the read name a child's place.
#[test]
fn a_field_written_self_names_its_type_in_swift() {
    compared(
        "swift-self",
        &SELF_ROWS,
        vec![
            bough_swift::swift_definition(),
            spur_swift::swift_definition(),
            fork_swift::swift_definition(),
            rope_swift::swift_definition(),
        ],
    );
}

/// A model declared below the model that holds it is refused with the text of the Rust read.
#[test]
fn a_model_declared_below_its_holder_is_refused_as_rust_refuses_it_in_swift() {
    compared(
        "swift-below",
        &BELOW_ROWS,
        vec![
            hitch_swift::swift_definition(),
            hitched_trailer_swift::swift_definition(),
            hauler_swift::swift_definition(),
            towed_trailer_swift::swift_definition(),
        ],
    );
}

/// A variant's members cross under its own `rename_all` in Swift, as serde names them.
#[test]
fn a_variants_own_rename_all_names_its_members_in_swift() {
    compared(
        "swift-carriage",
        &CARRIAGE_ROWS,
        vec![
            carriage_swift::swift_definition(),
            carriage_external_swift::swift_definition(),
            carriage_adjacent_swift::swift_definition(),
            carriage_untagged_swift::swift_definition(),
        ],
    );
}
