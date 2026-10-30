//! The emitted Dart types run under the Dart VM and passed by the analyzer, named as Rust names
//! them: a field written `Self`, a model declared below the model that holds it, and a variant
//! whose own `rename_all` names its members under every tagging. Each payload prints what the
//! serde read prints of it.

#![cfg(feature = "dart")]

use core::fmt::Write as _;

use super::named_by_rust_models::{
    bough_dart, carriage_adjacent_dart, carriage_dart, carriage_external_dart,
    carriage_untagged_dart, fork_dart, hauler_dart, hitch_dart, hitched_trailer_dart, rope_dart,
    spur_dart, tip_dart, towed_trailer_dart,
};
use super::named_by_rust_rows::{
    BELOW_ROWS, CARRIAGE_ROWS, Row, SELF_ROWS, TIP_ROWS, declares_a_parameter, expected, normalized,
};
use super::run_dart_validate::{dart, module};

/// What prints a row: the value read and written back, or `refused`, with each message of what
/// the read refused where the row compares them.
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

/// The emitted types of the models a field written `Self` is declared on.
fn self_definitions() -> Vec<String> {
    vec![
        bough_dart::dart_definition(),
        spur_dart::dart_definition(),
        fork_dart::dart_definition(),
        rope_dart::dart_definition(),
    ]
}

/// The emitted types of a holder and the models declared below it.
fn below_definitions() -> Vec<String> {
    vec![
        hitch_dart::dart_definition(),
        hitched_trailer_dart::dart_definition(),
        hauler_dart::dart_definition(),
        towed_trailer_dart::dart_definition(),
    ]
}

/// The emitted types of the enums whose variant writes its own `rename_all`.
fn carriage_definitions() -> Vec<String> {
    vec![
        carriage_dart::dart_definition(),
        carriage_external_dart::dart_definition(),
        carriage_adjacent_dart::dart_definition(),
        carriage_untagged_dart::dart_definition(),
    ]
}

/// The emitted types, the row printer, and a `main` that reads each payload of `rows` with the
/// `fromJson` of its type and writes it back with `toJson`.
fn source(rows: &[Row], definitions: &[String]) -> String {
    let calls = rows.iter().fold(String::new(), |mut written, row| {
        let (converted, write) = if declares_a_parameter(row.read_as) {
            (", (e) => e as int", "(e) => e")
        } else {
            ("", "")
        };
        let _ = writeln!(
            written,
            "  row('{}', {}, () => {}.fromJson(jsonDecode(r'{}'){converted}).toJson({write}));",
            row.name, row.judged, row.read_as, row.json
        );
        written
    });
    module(definitions, &[ROW], &format!("void main() {{\n{calls}}}\n"))
}

/// Runs `rows` and compares what the VM prints with what the serde read prints.
fn compared(named: &str, rows: &[Row], definitions: &[String]) {
    let expected = expected(rows);
    let Some((ran, printed)) = dart(named, &[], &source(rows, definitions)) else {
        return;
    };
    assert!(ran, "{printed}");
    assert_eq!(normalized(&printed), expected);
}

/// A field written `Self` names the enclosing type, in an `Option`, a `Box`, a map's value, a
/// variant's slot and a generic type, and `validate()` and the read name a child's place.
#[test]
fn a_field_written_self_names_its_type_in_dart() {
    compared("self", &SELF_ROWS, &self_definitions());
}

/// A field written `Self` in an `Option` over a `Box` names the enclosing type.
#[test]
fn an_optional_box_of_self_names_its_type_in_dart() {
    compared("tip", &TIP_ROWS, &[tip_dart::dart_definition()]);
}

/// A model declared below the model that holds it is refused with the text of the Rust read.
#[test]
fn a_model_declared_below_its_holder_is_refused_as_rust_refuses_it_in_dart() {
    compared("below", &BELOW_ROWS, &below_definitions());
}

/// A variant's members cross under its own `rename_all` in Dart, as serde names them.
#[test]
fn a_variants_own_rename_all_names_its_members_in_dart() {
    compared("carriage", &CARRIAGE_ROWS, &carriage_definitions());
}

/// The analyzer passes the emitted types.
#[test]
fn the_emitted_types_named_by_rust_have_no_issue() {
    for (named, definitions) in [
        ("named-by-rust-self-analyze", self_definitions()),
        (
            "named-by-rust-tip-analyze",
            vec![tip_dart::dart_definition()],
        ),
        ("named-by-rust-below-analyze", below_definitions()),
        ("named-by-rust-carriage-analyze", carriage_definitions()),
    ] {
        let analyzed = module(
            &definitions,
            &[ROW],
            "void main() => row('a list', false, () => jsonDecode('[]'));\n",
        );
        let Some((clean, said)) = dart(named, &["analyze"], &analyzed) else {
            return;
        };
        assert!(clean, "{said}");
        assert!(said.contains("No issues found!"), "{said}");
    }
}
