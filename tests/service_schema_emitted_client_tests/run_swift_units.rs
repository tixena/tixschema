//! `()` read and written back by the emitted Swift types compiled with warnings as errors, against
//! serde: a field, a brand's slot, a list's item, a tuple's position, an `Option` and a variant's
//! one value, each read from `null` and written as `null`, and each value serde refuses in its
//! place refused with `DecodingError.dataCorrupted`, an absent key with the `keyNotFound` Swift's
//! own read throws. A tuple struct of no slot keeps `[]`.

#![cfg(all(feature = "swift", feature = "serde"))]

use alloc::collections::BTreeMap;
use core::fmt::Write as _;

use serde_json::Value;

use super::runtime::judged;
use super::unit_models::{
    REFUSED, WRITTEN, probe_maybe_unit_swift, probe_nothing_swift, probe_slotless_swift,
    probe_unit_adjacent_swift, probe_unit_choice_swift, probe_unit_either_holder_swift,
    probe_unit_either_swift, probe_units_swift, serde_refusal, serde_written,
};

/// Names the runtime to run, for a machine that has one somewhere other than `PATH`.
const RUNTIME_VAR: &str = "TIXSCHEMA_SWIFT";

/// What prints a row: the value read, written back with its keys sorted, the description of a
/// `dataCorrupted` the read throws, or the case of any other error Swift's own read throws, with
/// the key it names.
const TOLD: &str = r#"
func told<Read: Codable>(_ at: Int, _ type: Read.Type, _ payload: String) {
  do {
    let encoder = JSONEncoder()
    encoder.outputFormatting = [.sortedKeys, .withoutEscapingSlashes]
    let read = try JSONDecoder().decode(type, from: Data(payload.utf8))
    print("\(at): read \(String(data: try encoder.encode(read), encoding: .utf8)!)")
  } catch DecodingError.dataCorrupted(let context) {
    print("\(at): dataCorrupted: \(context.debugDescription)")
  } catch DecodingError.keyNotFound(let key, _) {
    print("\(at): keyNotFound: \(key.stringValue)")
  } catch DecodingError.valueNotFound {
    print("\(at): valueNotFound")
  } catch {
    print("\(at): \(error)")
  }
}
"#;

/// What the Swift read answers of each payload of [`REFUSED`], in its order.
const SWIFT_REFUSED: [&str; 14] = [
    "dataCorrupted: invalid type: [], expected unit",
    "dataCorrupted: invalid type: [], expected unit",
    "dataCorrupted: invalid type: [], expected unit",
    "dataCorrupted: invalid type: [], expected unit",
    "dataCorrupted: invalid type: {}, expected unit",
    "dataCorrupted: invalid type: \"x\", expected unit",
    "keyNotFound: unit",
    "keyNotFound: nothing",
    "dataCorrupted: invalid type: [], expected unit",
    "dataCorrupted: invalid type: [], expected unit",
    "dataCorrupted: invalid type: [], expected unit",
    "keyNotFound: body",
    "keyNotFound: either",
    "valueNotFound",
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
fn unit_is_read_from_null_and_written_as_null_in_swift() {
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
        probe_nothing_swift::swift_definition(),
        probe_units_swift::swift_definition(),
        probe_maybe_unit_swift::swift_definition(),
        probe_unit_choice_swift::swift_definition(),
        probe_unit_adjacent_swift::swift_definition(),
        probe_unit_either_swift::swift_definition(),
        probe_unit_either_holder_swift::swift_definition(),
        probe_slotless_swift::swift_definition(),
        TOLD.to_owned(),
        main,
    ]
    .join("\n\n");
    let Some(run) = judged(
        "swift-units",
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
