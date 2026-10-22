//! The four std ranges read and written by the emitted Dart types under the Dart VM: each payload
//! prints what the serde read prints of it, a range of a model or of a brand judged by
//! `validate()` in the words Rust's `validate()` writes.

#![cfg(feature = "dart")]

use core::fmt::Write as _;

use super::range_models::{
    atlas_dart, compass_dart, course_dart, detour_dart, excursion_dart, frontier_dart,
    itinerary_dart, milestone_dart, positioned_dart, signposted_dart, spanned_dart, stride_dart,
    waymark_dart, waypost_dart,
};
use super::range_rows::{declares_a_parameter, expected, judged, rows};
use super::run_dart_validate::{dart, module};

/// What prints a row: the value read, written back, or `refused`, with each message of what
/// `validate()` refused where the type judges a model or a brand it holds.
const ROW: &str = "
void row(String name, bool judged, Object? Function() written) {
  try {
    print('$name: read ${jsonEncode(written())}');
  } on FormatException catch (refused) {
    print('$name: refused');
    if (!judged) return;
    for (final violation in refused.message.split('; ')) {
      print('    $violation');
    }
  } catch (_) {
    print('$name: refused');
  }
}
";

/// The emitted types of the ranges, the row printer, and a `main` that reads each payload with
/// the `fromJson` of its type and writes it back with `toJson`.
fn source() -> String {
    let mut definitions = analyzed_definitions();
    definitions.push(positioned_dart::dart_definition());
    definitions.push(atlas_dart::dart_definition());
    definitions.push(excursion_dart::dart_definition());
    let calls = rows().iter().fold(String::new(), |mut written, row| {
        let (converted, write) = if declares_a_parameter(row.read_as) {
            (", (e) => e as int", "(e) => e")
        } else {
            ("", "")
        };
        let _ = writeln!(
            written,
            "  row('{}', {}, () => {}.fromJson(jsonDecode(r'{}'){converted}).toJson({write}));",
            row.name,
            judged(row.read_as),
            row.read_as,
            row.json
        );
        written
    });
    module(
        &definitions,
        &[ROW],
        &format!("void main() {{\n{calls}}}\n"),
    )
}

/// The emitted types the analyzer passes whole. `Positioned`, `Atlas` and `Excursion` are run and
/// left out: the codec of each map casts a key the analyzer already knows to be a string.
fn analyzed_definitions() -> Vec<String> {
    vec![
        milestone_dart::dart_definition(),
        waymark_dart::dart_definition(),
        itinerary_dart::dart_definition(),
        spanned_dart::dart_definition(),
        stride_dart::dart_definition(),
        signposted_dart::dart_definition(),
        frontier_dart::dart_definition(),
        detour_dart::dart_definition(),
        course_dart::dart_definition(),
        compass_dart::dart_definition(),
        waypost_dart::dart_definition(),
        #[cfg(feature = "chrono")]
        super::range_models::rostered_dart::dart_definition(),
    ]
}

/// `{"start":8,"end":13}`, `{"start":8}` and `{"end":13}` read into the record of each range and
/// written back as they were, in every position a range is held; a bound left out or of another
/// type refused; and each broken bound reported under its key.
#[test]
fn each_range_reads_and_writes_in_dart_as_serde_does() {
    let rows = rows();
    let expected = expected(&rows);
    for written in [
        r#"{"closed":{"start":1,"end":2},"from":{"start":8},"span":{"start":8,"end":13},"to":{"end":13}}"#,
        "    'span.start': too short: minimum length is 2, got 1",
        "    'span.end': too short: minimum length is 2, got 1",
    ] {
        assert!(expected.contains(written), "{written} in: {expected}");
    }
    let Some((ran, printed)) = dart("ranges", &[], &source()) else {
        return;
    };
    assert!(ran, "{printed}");
    assert_eq!(printed, expected);
}

/// The analyzer passes the emitted ranges.
#[test]
fn the_emitted_ranges_have_no_issue() {
    let analyzed = module(
        &analyzed_definitions(),
        &[ROW],
        "void main() => row('a list', false, () => jsonDecode('[]'));\n",
    );
    let Some((clean, said)) = dart("ranges-analyze", &["analyze"], &analyzed) else {
        return;
    };
    assert!(clean, "{said}");
    assert!(said.contains("No issues found!"), "{said}");
}
