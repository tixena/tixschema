//! A plain enum that names a check, read on its own and held by another type, by the emitted Dart
//! types under the Dart VM, and passed by the analyzer: each payload of
//! [`plain_enum_check_rows`](super::plain_enum_check_rows) prints what the serde read prints.

#![cfg(feature = "dart")]

use core::fmt::Write as _;

use super::plain_enum_check_rows::{
    appeal_dart, bench_dart, expected, hearing_dart, rows, ruling_dart, verdict_dart,
};
use super::run_dart_validate::{dart, module};

const UPHELD: &str = "
dynamic tidied(dynamic payload) => payload is String ? payload.trim() : payload;

List<String> allowed(Ruling value) =>
    value == Ruling.denied ? const ['a ruling is allowed'] : const [];

List<String> affirmed(Verdict value) =>
    value == Verdict.overturned ? const ['a verdict is upheld'] : const [];
";

const HARNESS: &str = "
void row(String name, Object? Function() read, bool detail) {
  try {
    print('$name: read ${jsonEncode(read())}');
  } on FormatException catch (refused) {
    print('$name: refused');
    if (detail) {
      for (final line in refused.message.split('; ')) {
        print('    $line');
      }
    }
  }
}
";

fn definitions() -> Vec<String> {
    vec![
        verdict_dart::dart_definition(),
        ruling_dart::dart_definition(),
        bench_dart::dart_definition(),
        appeal_dart::dart_definition(),
        hearing_dart::dart_definition(),
    ]
}

#[test]
fn a_plain_enum_with_a_check_reads_in_dart_as_serde_reads_it() {
    let rows = rows();
    let mut main = String::from("void main() {\n");
    for row in &rows {
        let _ = writeln!(
            main,
            "  row('{}', () => {}.fromJson(jsonDecode(r'''{}''')).toJson(), {});",
            row.name, row.read_as, row.payload, row.detail
        );
    }
    main.push_str("}\n");
    let source = module(&definitions(), &[UPHELD, HARNESS], &main);
    let Some((ran, printed)) = dart("plain-enum-reads", &[], &source) else {
        return;
    };
    assert!(ran, "{printed}");
    assert_eq!(printed, expected(&rows));
}

#[test]
fn the_dart_text_of_a_plain_enum_with_a_check_has_no_issue() {
    let source = module(
        &definitions(),
        &[UPHELD],
        "void main() => print(jsonEncode(Verdict.fromJson('Upheld').toJson()));\n",
    );
    let Some((clean, said)) = dart("plain-enum-reads-analyze", &["analyze"], &source) else {
        return;
    };
    assert!(clean, "{said}");
    assert!(said.contains("No issues found!"), "{said}");
}
