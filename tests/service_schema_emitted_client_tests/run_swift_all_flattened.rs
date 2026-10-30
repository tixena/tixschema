//! A struct whose every member is flattened, and an enum whose variant holds only a flattened
//! member, read and written back by the emitted Swift types compiled with warnings as errors.

#![cfg(feature = "swift")]

use core::fmt::Write as _;

use super::all_flattened_models::{
    REFUSED, ROWS, assert_printed_as_serde_writes, berth_swift, gimbal_swift, hoist_swift,
    keelson_swift, lantern_swift, mast_swift, rigged_swift, rigging_swift, rigs_swift,
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
    main
}

/// Each model reads its payload and writes back what serde writes, and refuses a payload serde
/// refuses.
#[test]
fn a_model_of_flattened_members_alone_is_read_and_written_back_in_swift() {
    let source = [
        "import Foundation".to_owned(),
        lantern_swift::swift_definition(),
        mast_swift::swift_definition(),
        berth_swift::swift_definition(),
        keelson_swift::swift_definition(),
        gimbal_swift::swift_definition(),
        rigging_swift::swift_definition(),
        rigged_swift::swift_definition(),
        rigs_swift::swift_definition(),
        hoist_swift::swift_definition(),
        TOLD.to_owned(),
        main_of_the_rows(),
    ]
    .join("\n\n");
    let Some(run) = judged(
        "swift-all-flattened",
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
    assert_printed_as_serde_writes(&run.stdout);
}
