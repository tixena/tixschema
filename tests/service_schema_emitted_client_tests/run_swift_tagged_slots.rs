//! The lone slot of a newtype variant under a tag alone, read and written back by the emitted
//! Swift types compiled with warnings as errors, against what serde writes: the slot is read from
//! the object without the tag, as serde hands it, and written beside the tag.

#![cfg(all(feature = "swift", feature = "serde"))]

use alloc::collections::BTreeMap;
use core::fmt::Write as _;

use serde_json::Value;

use super::runtime::judged;
use super::tagged_slot_models::{
    UNIT_ENTRY, WRITTEN, ballast_swift, bunkered_swift, demurrage_swift, docketed_swift,
    dray_swift, drayage_swift, freighted_swift, lighter_swift, lightering_swift, palleted_swift,
    serde_told, stevedore_swift, tonnage_swift, waybilled_swift, wharfage_swift,
};

/// Names the runtime to run, for a machine that has one somewhere other than `PATH`.
const RUNTIME_VAR: &str = "TIXSCHEMA_SWIFT";

/// The function `Docketed::Trimmed` names, and what prints a row: the value read, written back
/// with its keys sorted, or the description of what the read throws.
const TOLD: &str = r#"
func lightened(_ payload: Any) -> Any {
  guard var members = payload as? [String: Any] else { return payload }
  members.removeValue(forKey: "spare")
  return members
}

func told<Read: Codable>(_ at: Int, _ type: Read.Type, _ payload: String) {
  do {
    let encoder = JSONEncoder()
    encoder.outputFormatting = [.sortedKeys, .withoutEscapingSlashes]
    let read = try JSONDecoder().decode(type, from: Data(payload.utf8))
    print("\(at): read \(String(data: try encoder.encode(read), encoding: .utf8)!)")
  } catch DecodingError.dataCorrupted(let context) {
    print("\(at): \(context.debugDescription)")
  } catch {
    print("\(at): \(error)")
  }
}
"#;

/// What the Swift client answers of [`UNIT_ENTRY`], for which TypeScript and Zod describe no
/// payload: it refuses the entry over `null`, as Zod refuses it.
const UNIT_ENTRY_TOLD: &str = "unknown tag Loose";

/// `json` with the keys of every object sorted, as the Swift encoder of the run writes them.
fn sorted(json: &str) -> String {
    /// `value` with the keys of every object it holds sorted.
    fn sorting(value: Value) -> Value {
        match value {
            Value::Object(entries) => {
                let keyed: BTreeMap<String, Value> = entries
                    .into_iter()
                    .map(|(key, held)| (key, sorting(held)))
                    .collect();
                Value::Object(keyed.into_iter().collect())
            }
            Value::Array(items) => Value::Array(items.into_iter().map(sorting).collect()),
            held @ (Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_)) => held,
        }
    }
    serde_json::to_string(&sorting(serde_json::from_str(json).unwrap())).unwrap()
}

/// Each payload serde writes is read by the Swift read and written back as serde wrote it.
#[test]
fn a_slot_beside_the_tag_is_read_and_written_back_by_swift_as_serde_writes_it() {
    let mut main = String::new();
    let mut expected = String::new();
    for (at, (read_as, json)) in WRITTEN.iter().enumerate() {
        let _ = writeln!(main, "told({at}, {read_as}.self, #\"{json}\"#)");
        if *json == UNIT_ENTRY {
            let _ = writeln!(expected, "{at}: {UNIT_ENTRY_TOLD}");
        } else {
            let written = serde_told(read_as, json);
            let back = written.strip_prefix("read ").unwrap();
            let _ = writeln!(expected, "{at}: read {}", sorted(back));
        }
    }
    let source = [
        "import Foundation".to_owned(),
        tonnage_swift::swift_definition(),
        ballast_swift::swift_definition(),
        wharfage_swift::swift_definition(),
        drayage_swift::swift_definition(),
        lighter_swift::swift_definition(),
        demurrage_swift::swift_definition(),
        bunkered_swift::swift_definition(),
        palleted_swift::swift_definition(),
        dray_swift::swift_definition(),
        stevedore_swift::swift_definition(),
        freighted_swift::swift_definition(),
        docketed_swift::swift_definition(),
        lightering_swift::swift_definition(),
        waybilled_swift::swift_definition(),
        TOLD.to_owned(),
        main,
    ]
    .join("\n\n");
    let Some(run) = judged(
        "swift-tagged-slots",
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
    assert_eq!(run.stdout, expected, "the Swift read is not serde's");
}
