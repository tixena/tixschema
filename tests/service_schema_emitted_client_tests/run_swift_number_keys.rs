//! A map keyed by a brand over a number or a flag, or by an alias of a number, read and written
//! back by the emitted Swift types compiled with warnings as errors, against serde, over each
//! payload of [`number_key_rows`](super::number_key_rows). A key serde refuses is refused, and a
//! number or a flag written between quotes as a value is refused.

#![cfg(feature = "swift")]

use core::fmt::Write as _;

use super::number_key_rows::{
    adjacent_pier_swift, anchor_swift, beacon_swift, burgee_swift, cutter_swift, depths_swift,
    dinghy_swift, float_texts_serde_refuses, gauge_swift, hull_swift, ketch_swift, lanyard_swift,
    moorings_swift, pennant_swift, pier_swift, rows, rudder_swift, shoal_swift, signal_swift,
    skiff_swift, slips_swift, sloop_swift, soundings_swift, yawl_swift,
};
use super::run_swift_read::normalized;
use super::runtime::judged;

/// Names the runtime to run, for a machine that has one somewhere other than `PATH`.
const RUNTIME_VAR: &str = "TIXSCHEMA_SWIFT";

/// What prints a row: the value read and written back, or `refused`.
const HARNESS: &str = r#"
func row<Read: Codable>(_ name: String, _ type: Read.Type, _ payload: String) {
  do {
    let held = try JSONDecoder().decode(type, from: Data(payload.utf8))
    let encoder = JSONEncoder()
    encoder.outputFormatting = [.sortedKeys]
    print("\(name): read \(String(data: try encoder.encode(held), encoding: .utf8)!)")
  } catch {
    print("\(name): refused")
  }
}
"#;

/// Each payload is read and written back by the Swift read as serde reads and writes it.
#[test]
fn a_map_keyed_by_a_number_brand_or_alias_is_read_and_written_back_by_swift_as_serde_does() {
    let mut rows = rows();
    rows.extend(float_texts_serde_refuses());
    let mut calls = String::new();
    let mut expected = String::new();
    for (at, row) in rows.iter().enumerate() {
        let _ = writeln!(
            calls,
            "row(\"{at}\", {}.self, #\"{}\"#)",
            row.named, row.json
        );
        let _ = writeln!(expected, "{at}: {}", row.serde);
    }
    assert!(expected.contains("refused"), "{expected}");
    let source = [
        "import Foundation".to_owned(),
        adjacent_pier_swift::swift_definition(),
        anchor_swift::swift_definition(),
        beacon_swift::swift_definition(),
        burgee_swift::swift_definition(),
        cutter_swift::swift_definition(),
        depths_swift::swift_definition(),
        dinghy_swift::swift_definition(),
        gauge_swift::swift_definition(),
        hull_swift::swift_definition(),
        ketch_swift::swift_definition(),
        lanyard_swift::swift_definition(),
        moorings_swift::swift_definition(),
        pennant_swift::swift_definition(),
        pier_swift::swift_definition(),
        rudder_swift::swift_definition(),
        shoal_swift::swift_definition(),
        signal_swift::swift_definition(),
        skiff_swift::swift_definition(),
        slips_swift::swift_definition(),
        sloop_swift::swift_definition(),
        soundings_swift::swift_definition(),
        yawl_swift::swift_definition(),
        HARNESS.to_owned(),
        calls,
    ]
    .join("\n\n");
    let Some(run) = judged(
        "swift-number-keys",
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
