//! A `usize` read as a `UInt64` and written back as a JSON number by the emitted Swift types
//! compiled with warnings as errors, against serde, in each place a member holds one: the values
//! above `Int64.max` included, each payload serde refuses refused, a default read as serde reads
//! it, and `validate()` answering what the Rust `validate()` answers.

#![cfg(all(feature = "swift", feature = "serde"))]

use alloc::collections::BTreeMap;
use core::fmt::Write as _;

use serde_json::Value;

use super::runtime::judged;
use super::usize_models::{
    DEFAULTED, OUT_OF_BOUNDS, REFUSED, WRITTEN, offsets_swift, rust_violations, serde_defaulted,
    serde_refusal, serde_written,
};
use super::usize_positions_models::{holding_swift, placed_swift, slot_index_swift};

/// Names the runtime to run, for a machine that has one somewhere other than `PATH`.
const RUNTIME_VAR: &str = "TIXSCHEMA_SWIFT";

/// What prints a row: the value read, written back with its keys sorted, the description of a
/// `dataCorrupted` the read throws, or any other error; the default a read takes for an absent
/// key; and what `validate()` answers of a value read with no judgement.
const TOLD: &str = r#"
func told<Read: Codable>(_ at: Int, _ type: Read.Type, _ payload: String) {
  do {
    let encoder = JSONEncoder()
    encoder.outputFormatting = [.sortedKeys, .withoutEscapingSlashes]
    let read = try JSONDecoder().decode(type, from: Data(payload.utf8))
    print("\(at): read \(String(data: try encoder.encode(read), encoding: .utf8)!)")
  } catch DecodingError.dataCorrupted(let context) {
    print("\(at): dataCorrupted: \(context.debugDescription)")
  } catch {
    print("\(at): \(error)")
  }
}

func defaulted(_ at: Int, _ payload: String) throws {
  print("\(at): defaulted \(try JSONDecoder().decode(Placed.self, from: Data(payload.utf8)).defaulted)")
}

struct Unjudged: Decodable {
  let held: Offsets
  init(from decoder: Decoder) throws { held = try Offsets(swiftSchemaUnjudged: decoder) }
}

func validated(_ at: Int, _ bounded: UInt64) throws {
  let payload = "{\"at\":0,\"bounded\":\(bounded),\"by_index\":{},\"span\":{\"start\":0,\"end\":0}}"
  let read = try JSONDecoder().decode(Unjudged.self, from: Data(payload.utf8)).held
  print("\(at): validate: \(read.validate().joined(separator: "; "))")
}
"#;

/// The Swift read of each payload of [`REFUSED`], in its order: each refusal in serde's words.
const SWIFT_REFUSED: [&str; 6] = [
    "dataCorrupted: invalid value: integer `-1`, expected usize",
    "dataCorrupted: invalid type: floating point `1.8446744073709552e+19`, expected usize",
    "dataCorrupted: invalid type: floating point `1.5`, expected usize",
    "dataCorrupted: 'bounded': too small: minimum is 1, got 0",
    "dataCorrupted: 'bounded': too large: maximum is 4096, got 4097",
    "dataCorrupted: 'pinned': not the literal 18446744073709551615, got 9223372036854775808",
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

/// Each payload serde writes is read and written back as serde writes it back, each payload serde
/// refuses is refused, an absent key is read as its default, and `validate()` answers the Rust
/// sentence, under `-warnings-as-errors`.
#[test]
fn a_usize_is_read_and_written_as_a_uint64_in_swift() {
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
    let _ = writeln!(main, "try defaulted({at}, #\"{DEFAULTED}\"#)");
    let _ = writeln!(expected, "{at}: defaulted {}", serde_defaulted());
    at += 1;
    for bounded in OUT_OF_BOUNDS {
        let _ = writeln!(main, "try validated({at}, {bounded})");
        let _ = writeln!(expected, "{at}: validate: {}", rust_violations(bounded));
        at += 1;
    }
    let source = [
        "import Foundation".to_owned(),
        offsets_swift::swift_definition(),
        slot_index_swift::swift_definition(),
        holding_swift::swift_definition(),
        placed_swift::swift_definition(),
        TOLD.to_owned(),
        main,
    ]
    .join("\n\n");
    let Some(run) = judged(
        "swift-usize",
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
