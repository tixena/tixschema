//! Each limit of [`ROWS`] held by the emitted Dart types under the Dart VM: `validate()` of the
//! value read with no judgement, and the read that refuses with what `validate()` answers. The
//! analyzer passes every type.

#![cfg(feature = "dart")]

use core::fmt::Write as _;

use super::integer_limit_models::{
    REPORT, ROWS, limit_ceiling_dart, limit_ends_dart, limit_floor_dart, limit_forms_dart,
    limit_mixed_dart, limit_wide_dart,
};
use super::run_dart_validate::{analyzed, dart, module};

/// What prints a row: `passes` or each message of `validate()`, and a line more where the read
/// does not answer what `validate()` answered.
const REPORTER: &str = "
void report(String name, List<String> answered, Object? Function() read) {
  if (answered.isEmpty) {
    print('$name: passes');
  } else {
    print('$name: refused');
    for (final message in answered) {
      print('    $message');
    }
  }
  try {
    read();
    if (answered.isNotEmpty) print('    the read passed it');
  } on FormatException catch (refused) {
    if (refused.message != answered.join('; ')) print('    the read said ${refused.message}');
  }
}
";

fn definitions() -> Vec<String> {
    vec![
        limit_ceiling_dart::dart_definition(),
        limit_floor_dart::dart_definition(),
        limit_forms_dart::dart_definition(),
        limit_mixed_dart::dart_definition(),
        limit_ends_dart::dart_definition(),
        limit_wide_dart::dart_definition(),
    ]
}

#[test]
fn each_limit_holds_in_dart_as_it_is_written() {
    let mut main = String::from("void main() {\n");
    for (name, read_as, json) in ROWS {
        let _ = writeln!(
            main,
            "  report('{name}', {read_as}.fromJsonUnjudged(jsonDecode(r'{json}')).validate(), () \
             => {read_as}.fromJson(jsonDecode(r'{json}')));"
        );
    }
    main.push_str("}\n");
    let source = module(&definitions(), &[REPORTER], &main);
    let Some((ran, printed)) = dart("integer-limits", &[], &source) else {
        return;
    };
    assert!(ran, "{printed}");
    assert_eq!(printed, REPORT);
}

#[test]
fn the_emitted_integer_limits_have_no_issue() {
    let source = module(
        &definitions(),
        &[REPORTER],
        "void main() => report('none', const [], () => jsonDecode('[]'));\n",
    );
    analyzed("integer-limits-analyze", &source);
}
