//! An enum's variant serde neither writes nor reads is written by no emitted Dart type: each enum
//! reads the payload of every variant serde writes and writes it back, and refuses the payload of
//! one it skips, as serde does, under every tagging, run under the Dart VM.

#![cfg(feature = "dart")]

use super::run_dart_validate::{dart, module};
use super::skipped_variant_models::{
    ROWS, berthage_dart, cartage_dart, dockage_dart, keelage_dart, lighterage_dart, pilotage_dart,
    tonnage_dart, unread_writes, wharfage_dart,
};

/// What reads each payload and prints it written back, or `refused`.
const SAID: &str = "
String said(dynamic Function() read) {
  try {
    return 'read ${jsonEncode(read())}';
  } catch (_) {
    return 'refused';
  }
}
";

/// The emitted definitions of every enum of the models.
fn definitions() -> Vec<String> {
    vec![
        berthage_dart::dart_definition(),
        wharfage_dart::dart_definition(),
        pilotage_dart::dart_definition(),
        lighterage_dart::dart_definition(),
        keelage_dart::dart_definition(),
        cartage_dart::dart_definition(),
        tonnage_dart::dart_definition(),
        dockage_dart::dart_definition(),
    ]
}

/// What prints each row serde and Zod agree on, read as its type and written back.
fn main_of_rows() -> String {
    let lines: Vec<String> = ROWS
        .iter()
        .filter(|row| row.serde == row.zod)
        .map(|row| {
            format!(
                "  print(said(() => {}.fromJson(jsonDecode(r'{}')).toJson()));",
                row.read_as, row.json
            )
        })
        .collect();
    format!("void main() {{\n{}\n}}\n", lines.join("\n"))
}

/// Each row serde reads is read and written back as it was, and each row serde refuses, of a
/// variant it neither writes nor reads, is refused.
#[test]
fn a_variant_serde_neither_writes_nor_reads_is_refused_in_dart() {
    assert!(
        unread_writes()
            .iter()
            .rev()
            .skip(1)
            .all(|said| said.ends_with("cannot be serialized")),
        "serde writes a variant it skips"
    );
    let source = module(&definitions(), &[SAID], &main_of_rows());
    let Some((ran, printed)) = dart("skipped-variants", &[], &source) else {
        return;
    };
    assert!(ran, "{printed}\n{source}");
    let rows: Vec<_> = ROWS.iter().filter(|row| row.serde == row.zod).collect();
    let lines: Vec<&str> = printed.lines().collect();
    assert_eq!(lines.len(), rows.len(), "{printed}");
    for (row, line) in rows.iter().zip(lines) {
        let read = (row.read)(row.json);
        assert_eq!(read, row.serde, "serde's read of {}", row.json);
        if read {
            let written: serde_json::Value =
                serde_json::from_str(line.strip_prefix("read ").unwrap_or("null")).unwrap();
            assert_eq!(
                written,
                serde_json::from_str::<serde_json::Value>(row.json).unwrap(),
                "{} as {}: {line}",
                row.json,
                row.read_as
            );
        } else {
            assert_eq!(line, "refused", "{} as {}", row.json, row.read_as);
        }
    }
}

/// The analyzer passes the enums.
#[test]
fn the_emitted_enums_with_a_skipped_variant_have_no_issue() {
    let source = module(&definitions(), &[SAID], &main_of_rows());
    let Some((clean, said)) = dart("skipped-variants-analyze", &["analyze"], &source) else {
        return;
    };
    assert!(clean, "{said}");
    assert!(said.contains("No issues found!"), "{said}");
}
