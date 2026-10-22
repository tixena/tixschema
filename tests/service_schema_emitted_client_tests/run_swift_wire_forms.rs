//! The emitted Swift types, compiled with warnings as errors, handed each value of [`WIRE_FORMS`]
//! and written through their own `encode(to:)`, which every Swift client writes a message
//! through: each prints the payload it sends, or `refused`, as the table's Swift column states,
//! and reads every payload any client sends. A `Date` is handed as the instant its text names.

#![cfg(all(feature = "swift", feature = "chrono"))]

use chrono::DateTime;

use super::runtime::judged;
use super::wire_form_rows::{Held, constructed, handed, held, printed, sent_by_any};
use super::wire_forms_models::{
    WireFormRow, conveyed_day_swift, conveyed_f32_swift, conveyed_f64_swift, conveyed_flag_swift,
    conveyed_hour_swift, conveyed_i8_swift, conveyed_i16_swift, conveyed_i32_swift,
    conveyed_i64_swift, conveyed_instant_swift, conveyed_isize_swift, conveyed_letter_swift,
    conveyed_literal_count_swift, conveyed_literal_flag_swift, conveyed_literal_text_swift,
    conveyed_millis_swift, conveyed_slot_swift, conveyed_slotted_swift, conveyed_stamp_swift,
    conveyed_text_swift, conveyed_u8_swift, conveyed_u16_swift, conveyed_u32_swift,
};

/// Names the runtime to run, for a machine that has one somewhere other than `PATH`.
const RUNTIME_VAR: &str = "TIXSCHEMA_SWIFT";

/// What prints a value handed: the payload its `encode(to:)` writes, or `refused` where it throws;
/// and what prints a payload read: `read`, or the refusal.
const PRINTERS: &str = r#"
func sent<Sent: Encodable>(_ handed: () -> Sent) -> String {
  let encoder = JSONEncoder()
  encoder.outputFormatting = [.sortedKeys, .withoutEscapingSlashes]
  do {
    return String(decoding: try encoder.encode(handed()), as: UTF8.self)
  } catch {
    return "refused"
  }
}

func read<Read: Decodable>(_ type: Read.Type, _ payload: String) -> String {
  do {
    _ = try JSONDecoder().decode(type, from: Data(payload.utf8))
    return "read"
  } catch {
    return "refused \(payload): \(error)"
  }
}
"#;

/// The Swift value of `json`, as the model `held_by` holds it.
pub fn swift_value(held_by: &str, json: &str) -> String {
    let text = json.trim_matches('"');
    match held(held_by) {
        Held::Number(_) if json == "NaN" => "Double.nan".to_owned(),
        Held::Flag | Held::Millis | Held::Number(_) | Held::Text => json.to_owned(),
        Held::Instant => {
            let instant = DateTime::parse_from_rfc3339(text).unwrap();
            format!(
                "Date(timeIntervalSince1970: {}.{:03})",
                instant.timestamp(),
                instant.timestamp_subsec_millis()
            )
        }
        Held::Slot => {
            let mut letters = text.chars();
            let first = letters.next().map(|letter| letter.to_ascii_lowercase());
            format!(
                "ConveyedSlot.{}{}",
                first.unwrap_or_default(),
                letters.as_str()
            )
        }
    }
}

/// The model of `row` holding its value as it was handed.
fn swift_handed(row: &WireFormRow) -> String {
    let value = swift_value(row.held_by, row.handed);
    constructed(
        row.held_by,
        "NAME: ",
        &|key| format!("[{key}: true]"),
        &|listed| format!("[{listed}]"),
        &value,
        &value,
    )
}

/// Every value handed as the table's Swift column states, and every payload any client sends read
/// back.
#[test]
fn each_value_is_sent_by_swift_in_the_form_every_target_reads() {
    let rows = handed(|row| row.swift);
    let mut expected = printed(&rows);
    let mut main: Vec<String> = rows
        .iter()
        .map(|(row, _)| format!("print(sent {{ {} }})", swift_handed(row)))
        .collect();
    for (held_by, payload) in sent_by_any(|row| row.swift) {
        expected.push_str("read\n");
        main.push(format!("print(read({held_by}.self, #\"{payload}\"#))"));
    }
    let source = [
        "import Foundation".to_owned(),
        conveyed_flag_swift::swift_definition(),
        conveyed_letter_swift::swift_definition(),
        conveyed_text_swift::swift_definition(),
        conveyed_u8_swift::swift_definition(),
        conveyed_u16_swift::swift_definition(),
        conveyed_u32_swift::swift_definition(),
        conveyed_i8_swift::swift_definition(),
        conveyed_i16_swift::swift_definition(),
        conveyed_i32_swift::swift_definition(),
        conveyed_i64_swift::swift_definition(),
        conveyed_isize_swift::swift_definition(),
        conveyed_f32_swift::swift_definition(),
        conveyed_f64_swift::swift_definition(),
        conveyed_literal_count_swift::swift_definition(),
        conveyed_literal_flag_swift::swift_definition(),
        conveyed_literal_text_swift::swift_definition(),
        conveyed_day_swift::swift_definition(),
        conveyed_hour_swift::swift_definition(),
        conveyed_stamp_swift::swift_definition(),
        conveyed_instant_swift::swift_definition(),
        conveyed_millis_swift::swift_definition(),
        conveyed_slot_swift::swift_definition(),
        conveyed_slotted_swift::swift_definition(),
        PRINTERS.to_owned(),
        main.join("\n"),
    ]
    .join("\n\n");
    let Some(run) = judged(
        "swift-wire-forms",
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
        run.stdout, expected,
        "the Swift client does not send each value as the table states"
    );
}
