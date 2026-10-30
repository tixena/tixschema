//! A check or a function named after a member of its own type, in the emitted Dart types: run by
//! the Dart VM and passed by the analyzer, each row printing what Rust prints.

#![cfg(feature = "dart")]

use core::fmt::Write as _;

use super::member_named_rows::{
    Kind, couple_dart, crate_dart, expected, freight_dart, gauge_dart, hold_dart, ledger_dart,
    parcel_dart, plain_dart, rating_dart, rows, stamp_dart,
};
use super::run_dart_validate::{dart, module};

const FUNCTIONS: &str = "
List<String> rated(Rating value) =>
    value == Rating.rated ? const ['a rating is unrated'] : const [];

dynamic unrated(dynamic payload) => payload is String ? payload.trim() : payload;

List<String> label(Parcel value) => value.label.isEmpty ? const ['a parcel is labelled'] : const [];

List<String> code(String value) => value == 'dup' ? const ['a code is not dup'] : const [];

dynamic note(dynamic payload) =>
    payload is Map<String, dynamic> ? {'note': 'none', ...payload} : payload;

dynamic seal(dynamic payload) => payload is String ? payload.trim() : payload;

List<String> validate(Gauge value) => value.size > 9 ? const ['a gauge is at most 9'] : const [];

List<String> value(Stamp value) => value.value.isEmpty ? const ['a stamp is not empty'] : const [];

List<String> tag(Crate value) => value.tag == 'x' ? const ['a crate is not x'] : const [];

List<String> title(Freight value) =>
    value is FreightHeld && value.title.isEmpty ? const ['a freight is titled'] : const [];

List<String> codes<CodeType>(Hold<CodeType> value) =>
    value.codes.isEmpty ? const ['a hold has a code'] : const [];

List<String> spare(List<String> value) =>
    value.length > 1 ? const ['a hold has one spare'] : const [];

List<String> pair((String, int) value) =>
    value.$2 < 0 ? const ['a pair is not negative'] : const [];

List<String> entries(Map<int, String> value) =>
    value.isEmpty ? const ['a ledger has an entry'] : const [];

List<String> ruled(Plain value) => value.size == 0 ? const ['a plain is sized'] : const [];

void report(String name, List<String> violations) {
  if (violations.isEmpty) {
    print('$name: passes');
    return;
  }
  print('$name: refused');
  for (final violation in violations) {
    print('    $violation');
  }
}
";

fn definitions() -> Vec<String> {
    vec![
        rating_dart::dart_definition(),
        parcel_dart::dart_definition(),
        gauge_dart::dart_definition(),
        stamp_dart::dart_definition(),
        crate_dart::dart_definition(),
        freight_dart::dart_definition(),
        hold_dart::dart_definition(),
        couple_dart::dart_definition(),
        ledger_dart::dart_definition(),
        plain_dart::dart_definition(),
    ]
}

fn source() -> String {
    let mut main = String::from("void main() {\n");
    for row in rows() {
        let payload = format!("jsonDecode(r'''{}''')", row.payload);
        match row.kind {
            Kind::Validate => writeln!(
                main,
                "  report('{}', {}.fromJsonUnjudged({payload}{}).validate());",
                row.name,
                row.read_as,
                if row.read_as.contains('<') {
                    ", (e) => e as String"
                } else {
                    ""
                }
            ),
            Kind::Read => writeln!(
                main,
                "  print('{}: read ${{jsonEncode({}.fromJson({payload}).toJson())}}');",
                row.name, row.read_as
            ),
        }
        .unwrap();
    }
    main.push_str("}\n");
    module(&definitions(), &[FUNCTIONS], &main)
}

#[test]
fn a_check_named_after_a_member_runs_in_dart_as_it_runs_in_rust() {
    let Some((ran, printed)) = dart("member-named", &[], &source()) else {
        return;
    };
    assert!(ran, "{printed}");
    assert_eq!(printed, expected(&rows()));
}

#[test]
fn the_dart_text_of_a_check_named_after_a_member_has_no_issue() {
    let source = source();
    let Some((clean, said)) = dart("member-named-analyze", &["analyze"], &source) else {
        return;
    };
    assert!(clean, "{said}\n--- source ---\n{source}");
    assert!(said.contains("No issues found!"), "{said}");
}

#[test]
fn only_a_name_that_meets_a_member_is_called_through_a_forwarder() {
    let rating = rating_dart::dart_definition();
    assert!(
        rating.starts_with(
            "final _ratingCheckRated = rated;\nfinal _ratingPreprocessUnrated = unrated;\n"
        ),
        "{rating}"
    );
    assert!(rating.contains("..._ratingCheckRated(this)"), "{rating}");
    assert!(
        rating.contains("_ratingPreprocessUnrated(payload)"),
        "{rating}"
    );
    let (crate_text, parcel) = (
        crate_dart::dart_definition(),
        parcel_dart::dart_definition(),
    );
    assert!(
        crate_text.starts_with("final _crateCheckTag = tag;\n"),
        "{crate_text}"
    );
    assert!(
        parcel.contains("final _parcelCheckLabel = label;"),
        "{parcel}"
    );
    assert!(
        parcel.contains("final _parcelCheckCode = code;"),
        "{parcel}"
    );
    assert!(
        parcel.contains("final _parcelPreprocessNote = note;"),
        "{parcel}"
    );
    assert!(
        parcel.contains("final _parcelPreprocessSeal = seal;"),
        "{parcel}"
    );
    let plain = plain_dart::dart_definition();
    assert!(plain.contains("...ruled(this)"), "{plain}");
    assert!(!plain.contains("final _"), "{plain}");
}
