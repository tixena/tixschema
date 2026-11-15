//! A `DateTime<Tz>` read and written back by the emitted Dart types under the Dart VM, against
//! serde: a date of whole seconds is written with no fraction, as chrono writes it, a fraction of
//! milliseconds or microseconds is kept, and a date finer than a microsecond is read at the
//! microsecond Dart's `DateTime` holds. A map keyed by dates that read as one key is refused.

#![cfg(all(feature = "dart", feature = "chrono"))]

use core::fmt::Write as _;

use super::dates_models::{
    Nudge, SAME_AS_SERDE, landing_dart, nudge_dart, positions_dart, serde_back, stamped_dart,
};
use super::run_dart_validate::{analyzed, dart, module};

/// What prints a row: the value read and written back, or `refused`.
const TOLD: &str = "
void told(String at, Object? Function() read) {
  String printed;
  try {
    printed = 'read ${jsonEncode(read())}';
  } on FormatException catch (_) {
    printed = 'refused';
  }
  print('$at: $printed');
}
";

/// What prints a row: the message of the refusal, or `read`.
const MESSAGE: &str = "
void message(String at, Object? Function() read) {
  String printed;
  try {
    read();
    printed = 'read';
  } on FormatException catch (refusal) {
    printed = refusal.message;
  }
  print('$at: $printed');
}
";

/// A payload, what serde writes back for it, and what Dart writes back for it.
const ROWS: [(&str, &str, &str); 7] = [
    (
        "whole seconds",
        r#"{"at":"2025-09-16T05:20:00Z","by_at":{"2025-09-16T05:20:00Z":1}}"#,
        r#"{"at":"2025-09-16T05:20:00Z","by_at":{"2025-09-16T05:20:00Z":1}}"#,
    ),
    (
        "milliseconds",
        r#"{"at":"2025-09-16T05:20:00.500Z","by_at":{"2025-09-16T05:20:00.500Z":1}}"#,
        r#"{"at":"2025-09-16T05:20:00.500Z","by_at":{"2025-09-16T05:20:00.500Z":1}}"#,
    ),
    (
        "microseconds",
        r#"{"at":"2024-01-02T03:04:05.678901Z","by_at":{"2024-01-02T03:04:05.678901Z":1}}"#,
        r#"{"at":"2024-01-02T03:04:05.678901Z","by_at":{"2024-01-02T03:04:05.678901Z":1}}"#,
    ),
    (
        "nanoseconds",
        r#"{"at":"2024-01-02T03:04:05.678901234Z","by_at":{}}"#,
        r#"{"at":"2024-01-02T03:04:05.678901Z","by_at":{}}"#,
    ),
    (
        "keys that differ at the microsecond",
        r#"{"at":"2024-01-02T03:04:05Z","by_at":{"2024-01-02T03:04:05.678901Z":1,"2024-01-02T03:04:05.678902Z":2}}"#,
        r#"{"at":"2024-01-02T03:04:05Z","by_at":{"2024-01-02T03:04:05.678901Z":1,"2024-01-02T03:04:05.678902Z":2}}"#,
    ),
    (
        "keys that differ below the microsecond",
        r#"{"at":"2024-01-02T03:04:05Z","by_at":{"2024-01-02T03:04:05.678901234Z":1,"2024-01-02T03:04:05.678901567Z":2}}"#,
        "refused",
    ),
    (
        "an offset",
        r#"{"at":"2025-09-16T07:20:00+02:00","by_at":{}}"#,
        r#"{"at":"2025-09-16T05:20:00Z","by_at":{}}"#,
    ),
];

/// What serde writes back for `json`.
fn serde_told(json: &str) -> String {
    format!("read {}", serde_back("Landing", json))
}

/// Each payload is read by serde, and written back by Dart in the text of the table.
#[test]
fn a_date_is_read_and_written_back_by_dart_in_the_text_chrono_writes() {
    let mut main = String::from("void main() {\n");
    let mut expected = String::new();
    for (name, json, dart_writes) in ROWS {
        let _ = writeln!(
            main,
            "  told('{name}', () => Landing.fromJson(jsonDecode(r'{json}')).toJson());"
        );
        if dart_writes == "refused" {
            let _ = writeln!(expected, "{name}: refused");
        } else {
            let _ = writeln!(expected, "{name}: read {dart_writes}");
        }
    }
    main.push_str("}\n");
    for (name, json, dart_writes) in &ROWS[..3] {
        assert_eq!(
            serde_told(json),
            format!("read {dart_writes}"),
            "serde writes the {name} as Dart does"
        );
    }
    let source = module(&[landing_dart::dart_definition()], &[TOLD], &main);
    analyzed("dates-analyze", &source);
    let Some((ran, printed)) = dart("dates", &[], &source) else {
        return;
    };
    assert!(ran, "{printed}");
    assert_eq!(printed, expected, "the Dart write is not chrono's");
}

/// A date of whole seconds and of milliseconds, in a list, as a map's value, in a tuple's slot, in
/// a variant's slot and under `as_number`, is written back by Dart as serde writes it, and what
/// Dart writes is read by serde to the same instant.
#[test]
fn a_date_in_every_position_is_read_and_written_back_by_dart_as_serde_writes_it() {
    let mut main = String::from("void main() {\n");
    let mut expected = String::new();
    for (name, model, payload) in SAME_AS_SERDE {
        let _ = writeln!(
            main,
            "  told('{name}', () => {model}.fromJson(jsonDecode(r'{payload}')).toJson());"
        );
        let _ = writeln!(expected, "{name}: read {}", serde_back(model, payload));
    }
    main.push_str("}\n");
    let source = module(
        &[
            landing_dart::dart_definition(),
            nudge_dart::dart_definition(),
            positions_dart::dart_definition(),
            stamped_dart::dart_definition(),
        ],
        &[TOLD],
        &main,
    );
    analyzed("dates-positions-analyze", &source);
    let Some((ran, printed)) = dart("dates-positions", &[], &source) else {
        return;
    };
    assert!(ran, "{printed}");
    assert_eq!(printed, expected, "the Dart write is not serde's");
}

/// A date under `as_number` that holds a float, or an integer a `u64` holds above an `i64`, is
/// refused by Dart in the words serde refuses it in.
#[test]
fn a_date_under_as_number_is_refused_by_dart_in_the_words_serde_refuses_it_in() {
    let rows = [
        ("a fraction", r#"{"due":1.5}"#, r#"{"due":1.5}"#),
        ("a whole float", r#"{"due":1e3}"#, r#"{"due":1000.0}"#),
        (
            "an integer above an i64",
            r#"{"due":9223372036854775808}"#,
            r#"{"due":BigInt.parse('9223372036854775808')}"#,
        ),
    ];
    let mut main = String::from("void main() {\n");
    let mut expected = String::new();
    for (name, json, held) in rows {
        let _ = writeln!(
            main,
            "  message('{name}', () => Nudge.fromJson(<String, dynamic>{held}));"
        );
        let refused = serde_json::from_str::<Nudge>(json).unwrap_err().to_string();
        let (words, _) = refused.rsplit_once(" at line ").unwrap();
        let _ = writeln!(expected, "{name}: {words}");
    }
    main.push_str("}\n");
    let source = module(&[nudge_dart::dart_definition()], &[MESSAGE], &main);
    let Some((ran, printed)) = dart("dates-as-number-refused", &[], &source) else {
        return;
    };
    assert!(ran, "{printed}");
    assert_eq!(printed, expected, "the Dart refusal is not serde's");
}
