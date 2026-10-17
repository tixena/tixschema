//! The std `Result` read and written by the emitted Dart types under the Dart VM: each payload
//! prints what the serde read prints of it, a `Result` of a model or of a brand judged by
//! `validate()` in the words Rust's `validate()` writes.

#![cfg(feature = "dart")]

use core::fmt::Write as _;

use super::result_models::{
    attempted_dart, ballot_dart, bureau_dart, errand_dart, errata_dart, jacket_dart, mailbag_dart,
    masthead_dart, missive_dart, notebook_dart, postbox_dart, reply_dart, ruling_dart,
    settlement_dart, summary_dart, turn_dart, unparsed_dart,
};
use super::result_rows::{declares_a_parameter, expected, rows};
use super::run_dart_validate::{dart, module};

/// What prints a row: the value read, written back, or `refused`, with each message of what
/// `validate()` refused.
const ROW: &str = "
void row(String name, Object? Function() written) {
  try {
    print('$name: read ${jsonEncode(written())}');
  } on FormatException catch (refused) {
    print('$name: refused');
    for (final violation in refused.message.split('; ')) {
      print('    $violation');
    }
  } catch (_) {
    print('$name: refused');
  }
}
";

/// The emitted types of the `Result`s, the row printer, and a `main` that reads each payload with
/// the `fromJson` of its type and writes it back with `toJson`.
fn source() -> String {
    let mut definitions = analyzed_definitions();
    definitions.push(postbox_dart::dart_definition());
    definitions.push(jacket_dart::dart_definition());
    definitions.push(notebook_dart::dart_definition());
    let calls = rows().iter().fold(String::new(), |mut written, row| {
        let (converted, write) = if declares_a_parameter(row.read_as) {
            (", (e) => e as int", "(e) => e")
        } else {
            ("", "")
        };
        let _ = writeln!(
            written,
            "  row('{}', () => {}.fromJson(jsonDecode(r'{}'){converted}).toJson({write}));",
            row.name, row.read_as, row.json
        );
        written
    });
    module(
        &definitions,
        &[ROW],
        &format!("void main() {{\n{calls}}}\n"),
    )
}

/// The emitted types the analyzer passes whole. `Postbox`, `Jacket` and `Notebook` are run and
/// left out: the codec of each map casts a key the analyzer already knows to be a string.
fn analyzed_definitions() -> Vec<String> {
    vec![
        masthead_dart::dart_definition(),
        unparsed_dart::dart_definition(),
        ballot_dart::dart_definition(),
        mailbag_dart::dart_definition(),
        reply_dart::dart_definition(),
        missive_dart::dart_definition(),
        ruling_dart::dart_definition(),
        settlement_dart::dart_definition(),
        errand_dart::dart_definition(),
        attempted_dart::dart_definition(),
        turn_dart::dart_definition(),
        summary_dart::dart_definition(),
        bureau_dart::dart_definition(),
        errata_dart::dart_definition(),
    ]
}

/// `{"Ok": T}` and `{"Err": E}` read into the sealed class of each `Result` and written back as
/// they were, in every position a `Result` is held, `null` under an arm of `()`; an object of both
/// keys, of neither and of another key refused; and each broken bound in an arm reported under the
/// arm's key.
#[test]
fn each_result_reads_and_writes_in_dart_as_serde_does() {
    let rows = rows();
    let expected = expected(&rows);
    for written in [
        r#"the reply: read {"outcome":{"Ok":7},"stored":{"Ok":null}}"#,
        "both keys: refused",
        "neither key: refused",
        "another key: refused",
        "    'frontmatter.Ok.slug': too long: maximum length is 8, got 12",
        "    'frontmatter.Ok.title': too short: minimum length is 1, got 0",
    ] {
        assert!(expected.contains(written), "{written} in: {expected}");
    }
    let Some((ran, printed)) = dart("results", &[], &source()) else {
        return;
    };
    assert!(ran, "{printed}");
    assert_eq!(printed, expected);
}

/// The analyzer passes the emitted `Result`s.
#[test]
fn the_emitted_results_have_no_issue() {
    let analyzed = module(
        &analyzed_definitions(),
        &[ROW],
        "void main() => row('a list', () => jsonDecode('[]'));\n",
    );
    let Some((clean, said)) = dart("results-analyze", &["analyze"], &analyzed) else {
        return;
    };
    assert!(clean, "{said}");
    assert!(said.contains("No issues found!"), "{said}");
}
