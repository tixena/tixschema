//! A `literal` read by the emitted Dart types under the Dart VM, against serde: each payload of
//! the literal table is read and refused as serde reads and refuses it, each violation in the
//! sentence Rust's `validate()` reports, and the analyzer passes every type.

#![cfg(all(feature = "dart", feature = "serde"))]

use core::fmt::Write as _;

use super::literal_models::{
    Answered, LITERAL_READS, adjacent_assay_dart, assay_dart, bullion_dart, hallmark_dart,
    ingot_dart, literal_report, names_each_violation, serde_read, tagged_assay_dart,
    touchmark_dart,
};
use super::run_dart_validate::{dart, module};

/// What prints a row: its name and whether it is read, then each violation the read refused it
/// with, where `named` asks for them.
const TOLD: &str = "
void told(String name, bool named, Object? Function() read) {
  try {
    read();
    print('$name: read');
  } on FormatException catch (refusal) {
    print('$name: refused');
    if (named) {
      for (final violation in refusal.message.split('; ')) {
        print('    $violation');
      }
    }
  }
}
";

/// The emitted types every row reads.
fn definitions() -> Vec<String> {
    vec![
        adjacent_assay_dart::dart_definition(),
        assay_dart::dart_definition(),
        bullion_dart::dart_definition(),
        hallmark_dart::dart_definition(),
        ingot_dart::dart_definition(),
        tagged_assay_dart::dart_definition(),
        touchmark_dart::dart_definition(),
    ]
}

/// Each payload is read and refused by the Dart read as serde reads and refuses it.
#[test]
fn a_literal_refuses_in_dart_what_serde_refuses() {
    let mut main = String::from("void main() {\n");
    for row in &LITERAL_READS {
        assert_eq!(
            serde_read(row.held_by, row.payload).is_ok(),
            matches!(row.answered, Answered::Read),
            "serde no longer reads {} as the table says",
            row.name
        );
        let _ = writeln!(
            main,
            "  told('{}', {}, () => {}.fromJson(jsonDecode(r'''{}''')));",
            row.name,
            names_each_violation(row),
            row.held_by,
            row.payload
        );
    }
    main.push_str("}\n");
    let source = module(&definitions(), &[TOLD], &main);
    let Some((ran, printed)) = dart("literals", &[], &source) else {
        return;
    };
    assert!(ran, "{printed}");
    assert_eq!(
        printed,
        literal_report(names_each_violation),
        "the Dart read is not serde's"
    );
}

/// The analyzer passes the types the rows read.
#[test]
fn the_emitted_literal_types_have_no_issue() {
    let source = module(
        &definitions(),
        &[TOLD],
        "void main() => told('bullion', true, () => Bullion.fromJson(jsonDecode('[\"bar\",400]')));",
    );
    let Some((clean, said)) = dart("literals-analyze", &["analyze"], &source) else {
        return;
    };
    assert!(clean, "{said}");
    assert!(said.contains("No issues found!"), "{said}");
}
