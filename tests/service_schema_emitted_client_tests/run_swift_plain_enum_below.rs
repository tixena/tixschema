//! A plain enum flattened into a holder declared above it, read and written back by the emitted
//! Swift types compiled with warnings as errors.

#![cfg(feature = "swift")]

use core::fmt::Write as _;

use super::plain_enum_below_models::{
    BRANDED, REFUSED, ROWS, assert_printed_as_serde_writes, bobstay_swift, casement_swift,
    glazing_swift, mullion_swift, pane_swift, reefing_swift, transom_swift, weathering_swift,
};
use super::runtime::judged;

/// Names the runtime to run, for a machine that has one somewhere other than `PATH`.
const RUNTIME_VAR: &str = "TIXSCHEMA_SWIFT";

/// What prints a row read and written back, and whether a payload is refused.
const TOLD: &str = r#"
func told<Read: Codable>(_ name: String, _ type: Read.Type, _ payload: String) {
  let read = try! JSONDecoder().decode(type, from: Data(payload.utf8))
  let encoder = JSONEncoder()
  encoder.outputFormatting = [.sortedKeys]
  print("\(name): \(String(decoding: try! encoder.encode(read), as: UTF8.self))")
}

func refuses<Read: Codable>(_ name: String, _ type: Read.Type, _ payload: String) {
  print("\(name) refused: \((try? JSONDecoder().decode(type, from: Data(payload.utf8))) == nil)")
}
"#;

/// The statements that print every row and every refusal.
fn main_of_the_rows() -> String {
    let mut main = String::new();
    for (name, payload) in ROWS {
        let _ = writeln!(main, "told(\"{name}\", {name}.self, #\"{payload}\"#)");
    }
    for (name, payload) in REFUSED {
        let _ = writeln!(main, "refuses(\"{name}\", {name}.self, #\"{payload}\"#)");
    }
    let (name, payload) = BRANDED;
    let _ = writeln!(main, "refuses(\"{name}\", {name}.self, #\"{payload}\"#)");
    main
}

/// Each model reads its payload and writes back what serde writes, and refuses a payload serde
/// refuses.
#[test]
fn a_plain_enum_flattened_below_its_holder_is_read_and_written_back_in_swift() {
    let source = [
        "import Foundation".to_owned(),
        casement_swift::swift_definition(),
        transom_swift::swift_definition(),
        mullion_swift::swift_definition(),
        weathering_swift::swift_definition(),
        pane_swift::swift_definition(),
        reefing_swift::swift_definition(),
        bobstay_swift::swift_definition(),
        glazing_swift::swift_definition(),
        TOLD.to_owned(),
        main_of_the_rows(),
    ]
    .join("\n\n");
    let Some(run) = judged(
        "swift-plain-enum-below",
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
        "`swift -warnings-as-errors main.swift` failed.\n--- stdout ---\n{}\n--- stderr \
         ---\n{}\n--- source ---\n{source}",
        run.stdout, run.stderr
    );
    let (rows, branded) = run.stdout.trim_end().rsplit_once('\n').unwrap();
    assert_eq!(branded, "Reefing refused: true", "{}", run.stdout);
    assert_printed_as_serde_writes(&format!("{rows}\n"));
}
