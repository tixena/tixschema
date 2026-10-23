//! A tuple struct that declares one slot, which serde writes as the slot's value, read and written
//! by the emitted Swift types compiled with warnings as errors, over each payload of
//! [`lone_slot_rows`](super::lone_slot_rows).

#![cfg(feature = "swift")]

use super::lone_slot_rows::{expected, rows};
use super::lone_slots_models::{
    adjacent_stow_swift, bale_pair_swift, bale_swift, baled_swift, bales_by_name_swift,
    bales_by_number_swift, bales_swift, clipped_swift, count_swift, girth_swift, girthed_swift,
    labels_by_number_swift, laden_swift, maybe_bale_swift, moniker_swift, rigger_swift, sack_swift,
    sacked_swift, single_swift, sling_swift, slung_swift, stow_swift, stowed_by_name_swift,
    stowed_by_number_swift, stowed_list_swift, stowed_maybe_swift, stowed_pair_swift, strop_swift,
    tagged_stow_swift, tote_swift, twine_swift, twined_swift, unskipped_swift, untagged_stow_swift,
};
#[cfg(feature = "chrono")]
use super::lone_slots_models::{stowed_stamp_swift, ticked_millis_swift, ticked_swift};
use super::run_swift_read::normalized;
use super::runtime::judged;

/// Names the runtime to run, for a machine that has one somewhere other than `PATH`.
const RUNTIME_VAR: &str = "TIXSCHEMA_SWIFT";

/// What a run reads a payload with, and prints the answer with: the value written back, or each
/// message of a refusal.
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
    print("\(name): \(error)")
  }
}
"#;

/// The text of every model compiles with warnings as errors, and every payload reads and writes in
/// Swift as the serde read reads and writes it: the slot's value, with nothing around it. A value
/// that breaks a rule is refused as the Rust `validate()` reports it, the one slot under no step.
#[test]
fn a_struct_of_one_slot_reads_and_writes_in_swift_as_serde_does_wherever_it_is_held() {
    let rows = rows();
    let calls: Vec<String> = rows
        .iter()
        .map(|row| {
            format!(
                "row(\"{}\", {}.self, #\"{}\"#)",
                row.name, row.read_as, row.payload
            )
        })
        .collect();
    let mut parts = vec![
        "import Foundation".to_owned(),
        bale_swift::swift_definition(),
        sack_swift::swift_definition(),
        count_swift::swift_definition(),
        moniker_swift::swift_definition(),
        baled_swift::swift_definition(),
        bales_swift::swift_definition(),
        maybe_bale_swift::swift_definition(),
        bales_by_name_swift::swift_definition(),
        bales_by_number_swift::swift_definition(),
        labels_by_number_swift::swift_definition(),
        bale_pair_swift::swift_definition(),
        unskipped_swift::swift_definition(),
        clipped_swift::swift_definition(),
        single_swift::swift_definition(),
        sacked_swift::swift_definition(),
        stowed_list_swift::swift_definition(),
        stowed_maybe_swift::swift_definition(),
        stowed_by_name_swift::swift_definition(),
        stowed_by_number_swift::swift_definition(),
        stowed_pair_swift::swift_definition(),
        tote_swift::swift_definition(),
        sling_swift::swift_definition(),
        slung_swift::swift_definition(),
        girth_swift::swift_definition(),
        strop_swift::swift_definition(),
        girthed_swift::swift_definition(),
        rigger_swift::swift_definition(),
        twine_swift::swift_definition(),
        twined_swift::swift_definition(),
        laden_swift::swift_definition(),
        stow_swift::swift_definition(),
        adjacent_stow_swift::swift_definition(),
        untagged_stow_swift::swift_definition(),
        tagged_stow_swift::swift_definition(),
    ];
    #[cfg(feature = "chrono")]
    parts.extend([
        ticked_swift::swift_definition(),
        ticked_millis_swift::swift_definition(),
        stowed_stamp_swift::swift_definition(),
    ]);
    parts.extend([HARNESS.to_owned(), calls.join("\n")]);
    let source = parts.join("\n\n");
    let Some(run) = judged(
        "swift-lone-slots",
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
    assert_eq!(normalized(&run.stdout), normalized(&expected(&rows)));
}
