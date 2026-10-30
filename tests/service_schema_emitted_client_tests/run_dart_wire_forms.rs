//! The emitted Dart types, run under the Dart SDK, handed each value of [`WIRE_FORMS`] and written
//! through their `toJson`, which every Dart client writes a message through: each prints the
//! payload it sends, or `refused`, as the table's Dart column states, and reads every payload any
//! client sends. A `DateTime` of no zone is the local time of a run under `TZ=UTC`.

#![cfg(all(feature = "dart", feature = "chrono"))]

use core::fmt::Write as _;
use std::env;
use std::fs;
use std::process::{Command, id};

use super::run_dart_validate::{analyzed, module};
use super::runtime::ran;
use super::wire_form_rows::{Held, constructed, handed, held, printed, sent_by_any};
use super::wire_forms_models::{
    Sends, WIRE_FORMS, WireFormRow, conveyed_day_dart, conveyed_f32_dart, conveyed_f64_dart,
    conveyed_flag_dart, conveyed_hour_dart, conveyed_i8_dart, conveyed_i16_dart, conveyed_i32_dart,
    conveyed_i64_dart, conveyed_instant_dart, conveyed_isize_dart, conveyed_letter_dart,
    conveyed_literal_count_dart, conveyed_literal_flag_dart, conveyed_literal_text_dart,
    conveyed_millis_dart, conveyed_slot_dart, conveyed_slotted_dart, conveyed_stamp_dart,
    conveyed_text_dart, conveyed_u8_dart, conveyed_u16_dart, conveyed_u32_dart,
};

/// Names the runtime to run, for a machine that has one somewhere other than `PATH`.
const RUNTIME_VAR: &str = "TIXSCHEMA_DART";

/// What prints a value handed: the payload `jsonEncode` writes of its `toJson`, or `refused`
/// where either throws; what prints a payload read: `read`, or the refusal; and what prints a
/// payload the read is to refuse: `refused` for a `FormatException`.
const PRINTERS: &str = "
String sent(Object? Function() handed) {
  try {
    return jsonEncode(handed());
  } catch (_) {
    return 'refused';
  }
}

String read(Object? Function() reading) {
  try {
    reading();
    return 'read';
  } catch (refusal) {
    return 'refused: $refusal';
  }
}

String refusing(Object? Function() reading) {
  try {
    reading();
    return 'read';
  } on FormatException {
    return 'refused';
  }
}
";

/// The Dart value of `json`, as the model `held_by` holds it.
pub fn dart_value(held_by: &str, json: &str) -> String {
    let text = json.trim_matches('"');
    match held(held_by) {
        Held::Number(_) if json == "NaN" => "double.nan".to_owned(),
        Held::Flag | Held::Millis | Held::Number(_) => json.to_owned(),
        Held::Text => json.replace('$', "\\$"),
        Held::Instant => format!("DateTime.parse('{text}')"),
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
fn dart_handed(row: &WireFormRow) -> String {
    let value = dart_value(row.held_by, row.handed);
    constructed(
        row.held_by,
        "NAME: ",
        &|key| format!("{{{key}: true}}"),
        &|listed| {
            if row.held_by == "ConveyedU8" {
                format!("Uint8List.fromList([{listed}])")
            } else {
                format!("[{listed}]")
            }
        },
        &value,
        &value,
    )
}

/// Runs `dart` over `source` under `TZ=UTC`, in a directory of its own named for `named`, and
/// answers whether it succeeded with everything it wrote. `None` where no runtime was reachable,
/// which the probe has then said.
pub fn dart_at_utc(named: &str, source: &str) -> Option<(bool, String)> {
    ran(
        "dart",
        RUNTIME_VAR,
        "dart",
        "reachable.dart",
        "void main() {}",
    )?;
    let at = env::temp_dir().join(format!("tixschema-emitted-dart-{named}-{}", id()));
    if at.exists() {
        fs::remove_dir_all(&at).unwrap();
    }
    fs::create_dir_all(&at).unwrap();
    fs::write(at.join("module.dart"), source).unwrap();
    let runtime = env::var(RUNTIME_VAR).unwrap_or_else(|_| "dart".to_owned());
    let reported = Command::new(runtime)
        .arg("module.dart")
        .env("TZ", "UTC")
        .current_dir(&at)
        .output()
        .unwrap();
    fs::remove_dir_all(&at).unwrap();
    Some((
        reported.status.success(),
        format!(
            "{}{}",
            String::from_utf8_lossy(&reported.stdout),
            String::from_utf8_lossy(&reported.stderr)
        ),
    ))
}

/// Every value handed as the table's Dart column states, every payload any client sends read
/// back, and the payload of each value Dart refuses to send refused by its read, a JSON number
/// among them: Dart holds no `NaN` in JSON.
#[test]
fn each_value_is_sent_by_dart_in_the_form_every_target_reads() {
    let rows = handed(|row| row.dart);
    let mut expected = printed(&rows);
    let mut main = String::from("void main() {\n");
    for (row, _) in &rows {
        let _ = writeln!(main, "  print(sent(() => {}.toJson()));", dart_handed(row));
    }
    for (held_by, payload) in sent_by_any(|row| row.dart) {
        let _ = writeln!(expected, "read");
        let _ = writeln!(
            main,
            "  print(read(() => {held_by}.fromJson(jsonDecode(r'{payload}'))));"
        );
    }
    for row in WIRE_FORMS
        .iter()
        .filter(|row| row.dart == Sends::Refused && row.handed != "NaN")
    {
        let _ = writeln!(expected, "refused");
        let _ = writeln!(
            main,
            "  print(refusing(() => {}.fromJson(jsonDecode(r'{}'))));",
            row.held_by,
            row.payload(Sends::Handed).unwrap()
        );
    }
    main.push_str("}\n");
    let definitions = [
        conveyed_flag_dart::dart_definition(),
        conveyed_letter_dart::dart_definition(),
        conveyed_text_dart::dart_definition(),
        conveyed_u8_dart::dart_definition(),
        conveyed_u16_dart::dart_definition(),
        conveyed_u32_dart::dart_definition(),
        conveyed_i8_dart::dart_definition(),
        conveyed_i16_dart::dart_definition(),
        conveyed_i32_dart::dart_definition(),
        conveyed_i64_dart::dart_definition(),
        conveyed_isize_dart::dart_definition(),
        conveyed_f32_dart::dart_definition(),
        conveyed_f64_dart::dart_definition(),
        conveyed_literal_count_dart::dart_definition(),
        conveyed_literal_flag_dart::dart_definition(),
        conveyed_literal_text_dart::dart_definition(),
        conveyed_day_dart::dart_definition(),
        conveyed_hour_dart::dart_definition(),
        conveyed_stamp_dart::dart_definition(),
        conveyed_instant_dart::dart_definition(),
        conveyed_millis_dart::dart_definition(),
        conveyed_slot_dart::dart_definition(),
        conveyed_slotted_dart::dart_definition(),
    ];
    let source = module(&definitions, &[PRINTERS], &main);
    analyzed("wire-forms-analyze", &source);
    let Some((ran, printed_by_dart)) = dart_at_utc("wire-forms", &source) else {
        return;
    };
    assert!(ran, "{printed_by_dart}");
    assert_eq!(
        printed_by_dart, expected,
        "the Dart client does not send each value as the table states"
    );
}
