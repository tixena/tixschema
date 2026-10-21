//! A set read by the emitted Swift types, compiled with warnings as errors: a payload that repeats
//! an item of a set is refused by the type's read, in the words `validate()` answers, under the
//! set's place.

#![cfg(feature = "swift")]

use super::runtime::judged;
use super::set_models::{
    PAYLOADS, PRINTED, assert_rust_keeps_one_item, coin_swift, declares_a_parameter,
    every_list_swift, folder_swift, lanyard_swift, nesting_swift, point_swift, pouch_swift,
    shelving_swift, sorting_swift, ticket_swift, wallet_swift,
};

/// Names the runtime to run, for a machine that has one somewhere other than `PATH`.
const RUNTIME_VAR: &str = "TIXSCHEMA_SWIFT";

/// What a run reads a payload with, and prints the answer with: `read`, or `refused` with each
/// message of what `validate()` refused.
const HARNESS: &str = r#"
func row<Read: Codable>(_ name: String, _ type: Read.Type, _ payload: String) {
  do {
    _ = try JSONDecoder().decode(type, from: Data(payload.utf8))
    print("\(name): read")
  } catch DecodingError.dataCorrupted(let context) {
    print("\(name): refused")
    for line in context.debugDescription.components(separatedBy: "; ") { print("    \(line)") }
  } catch {
    print("\(name): \(error)")
  }
}
"#;

/// Each payload that repeats an item of a set is refused under the set's place, where Rust's read
/// keeps one item of it, and `validate()` answers the repeat of a set built in code.
#[test]
fn a_set_that_repeats_an_item_is_refused_by_the_swift_read() {
    assert_rust_keeps_one_item();
    let calls: Vec<String> = PAYLOADS
        .iter()
        .map(|payload| {
            let filled = if declares_a_parameter(payload.read_as) {
                "<String>"
            } else {
                ""
            };
            format!(
                "row(\"{}\", {}{filled}.self, #\"{}\"#)",
                payload.name, payload.read_as, payload.json
            )
        })
        .collect();
    let source = [
        "import Foundation".to_owned(),
        point_swift::swift_definition(),
        ticket_swift::swift_definition(),
        folder_swift::swift_definition(),
        every_list_swift::swift_definition(),
        nesting_swift::swift_definition(),
        lanyard_swift::swift_definition(),
        pouch_swift::swift_definition(),
        shelving_swift::swift_definition(),
        sorting_swift::swift_definition(),
        coin_swift::swift_definition(),
        wallet_swift::swift_definition(),
        HARNESS.to_owned(),
        calls.join("\n"),
        "print(Ticket(tags: [\"a\", \"a\"]).validate())".to_owned(),
    ]
    .join("\n\n");
    let Some(run) = judged(
        "swift-sets",
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
        format!("{PRINTED}[\"\\'tags\\': repeats an item: a set holds each item once\"]\n")
    );
}
