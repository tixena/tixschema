//! A set read and written by the emitted Swift types at one list level, as serde writes it,
//! compiled with warnings as errors.

#![cfg(feature = "swift")]

use core::fmt::Write as _;

use super::runtime::judged;
#[cfg(feature = "indexmap")]
use super::set_level_models::indexed_swift;
use super::set_level_models::{
    BROKEN, BROKEN_MESSAGE, ROUND_TRIPS, assert_rust_writes_back, entry_swift, groups_swift,
    probe_sets_swift, shelf_swift, wrappers_swift,
};

const RUNTIME_VAR: &str = "TIXSCHEMA_SWIFT";

const HARNESS: &str = r#"
func row<Read: Codable>(_ name: String, _ type: Read.Type, _ payload: String) {
  do {
    let held = try JSONDecoder().decode(type, from: Data(payload.utf8))
    let encoder = JSONEncoder()
    encoder.outputFormatting = .sortedKeys
    print("\(name): \(String(data: try encoder.encode(held), encoding: .utf8)!)")
  } catch DecodingError.dataCorrupted(let context) {
    print("\(name): refused")
    for line in context.debugDescription.components(separatedBy: "; ") { print("    \(line)") }
  } catch {
    print("\(name): \(error)")
  }
}
"#;

#[test]
fn a_set_is_one_list_level_in_the_swift_types() {
    assert_rust_writes_back();
    let mut main = String::new();
    for &(name, json) in ROUND_TRIPS {
        let _ = writeln!(main, "row(\"{name}\", {name}.self, #\"{json}\"#)");
    }
    let (broken_name, broken_json) = BROKEN;
    let _ = writeln!(
        main,
        "row(\"broken\", {broken_name}.self, #\"{broken_json}\"#)"
    );
    main.push_str(
        "print(Shelf(entries: [Entry(name: \"a\")], keyed: [:], maybe: nil, nullable: [], \
         slot: ShelfSlotTuple(slot0: [], slot1: 1)).validate().joined(separator: \"; \"))\n",
    );
    let mut parts = vec![
        "import Foundation".to_owned(),
        entry_swift::swift_definition(),
        groups_swift::swift_definition(),
        probe_sets_swift::swift_definition(),
        shelf_swift::swift_definition(),
        wrappers_swift::swift_definition(),
    ];
    #[cfg(feature = "indexmap")]
    parts.push(indexed_swift::swift_definition());
    parts.extend([HARNESS.to_owned(), main]);
    let source = parts.join("\n\n");
    let Some(run) = judged(
        "swift-set-levels",
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
    let mut expected = String::new();
    for &(name, json) in ROUND_TRIPS {
        let _ = writeln!(expected, "{name}: {json}");
    }
    let _ = write!(
        expected,
        "broken: refused\n    {BROKEN_MESSAGE}\n{BROKEN_MESSAGE}\n"
    );
    assert_eq!(run.stdout, expected, "{source}");
}
