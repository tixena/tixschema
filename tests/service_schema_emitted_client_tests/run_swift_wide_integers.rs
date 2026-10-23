//! An `i64` read and written back as a JSON number by the emitted Swift types compiled with
//! warnings as errors, against serde, in each place a member holds one, and each payload serde
//! refuses refused.

#![cfg(all(feature = "swift", feature = "serde"))]

use alloc::collections::BTreeMap;
use core::fmt::Write as _;

use serde_json::Value;

use super::runtime::judged;
use super::wide_integer_models::{
    REFUSED, WRITTEN, serde_refusal, serde_written, sixty_beside_swift, sixty_event_swift,
    sixty_id_swift, sixty_level_swift, sixty_locked_swift, sixty_reached_swift,
    sixty_revision_swift, sixty_tally_swift,
};

/// Names the runtime to run, for a machine that has one somewhere other than `PATH`.
const RUNTIME_VAR: &str = "TIXSCHEMA_SWIFT";

/// What prints a row: the value read, written back with its keys sorted, the description of a
/// `dataCorrupted` the read throws, the type and the path of a `typeMismatch`, whose description
/// Foundation words differently from one version to the next, or any other error.
const TOLD: &str = r#"
func told<Read: Codable>(_ at: Int, _ type: Read.Type, _ payload: String) {
  do {
    let encoder = JSONEncoder()
    encoder.outputFormatting = [.sortedKeys, .withoutEscapingSlashes]
    let read = try JSONDecoder().decode(type, from: Data(payload.utf8))
    print("\(at): read \(String(data: try encoder.encode(read), encoding: .utf8)!)")
  } catch DecodingError.dataCorrupted(let context) {
    print("\(at): dataCorrupted: \(context.debugDescription)")
  } catch DecodingError.typeMismatch(let expected, let context) {
    let path = context.codingPath.map { $0.stringValue }.joined(separator: ".")
    print("\(at): typeMismatch: \(expected) at '\(path)'")
  } catch {
    print("\(at): \(error)")
  }
}
"#;

/// What the Swift read answers of each payload of [`REFUSED`], in its order: `JSONDecoder` answers
/// a number beyond an `Int64` and a fraction as data that is not valid JSON.
const SWIFT_REFUSED: [&str; 8] = [
    "dataCorrupted: The given data was not valid JSON.",
    "typeMismatch: Int64 at ''",
    "dataCorrupted: The given data was not valid JSON.",
    "dataCorrupted: The given data was not valid JSON.",
    "dataCorrupted: 'level': too large: maximum is 5, got 6",
    "dataCorrupted: The given data was not valid JSON.",
    "dataCorrupted: 'fixed': not the literal 214, got 215",
    "typeMismatch: Int64 at 'keyed.4'",
];

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

/// Each payload serde writes is read and written back as serde writes it back, and each payload
/// serde refuses is refused, under `-warnings-as-errors`.
#[test]
fn an_i64_is_read_and_written_as_a_number_in_swift() {
    let mut main = String::new();
    let mut expected = String::new();
    let mut at = 0_usize;
    for (read_as, json) in WRITTEN {
        let _ = writeln!(main, "told({at}, {read_as}.self, #\"{json}\"#)");
        let _ = writeln!(
            expected,
            "{at}: read {}",
            sorted(&serde_written(read_as, json))
        );
        at += 1;
    }
    for ((read_as, json, said), refused) in REFUSED.into_iter().zip(SWIFT_REFUSED) {
        assert_eq!(
            serde_refusal(read_as, json).as_deref(),
            Some(said),
            "serde no longer refuses {json}"
        );
        let _ = writeln!(main, "told({at}, {read_as}.self, #\"{json}\"#)");
        let _ = writeln!(expected, "{at}: {refused}");
        at += 1;
    }
    let source = [
        "import Foundation".to_owned(),
        sixty_tally_swift::swift_definition(),
        sixty_id_swift::swift_definition(),
        sixty_event_swift::swift_definition(),
        sixty_level_swift::swift_definition(),
        sixty_revision_swift::swift_definition(),
        sixty_reached_swift::swift_definition(),
        sixty_locked_swift::swift_definition(),
        sixty_beside_swift::swift_definition(),
        TOLD.to_owned(),
        main,
    ]
    .join("\n\n");
    let Some(run) = judged(
        "swift-wide-integers",
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
