//! A set read by the emitted Dart types under the Dart VM: a payload that repeats an item of a set
//! is refused by the type's read, in the words `validate()` answers, under the set's place.

#![cfg(feature = "dart")]

use core::fmt::Write as _;

use super::run_dart_validate::{dart, module};
use super::set_models::{
    PAYLOADS, PRINTED, assert_rust_keeps_one_item, coin_dart, declares_a_parameter,
    every_list_dart, folder_dart, lanyard_dart, nesting_dart, point_dart, pouch_dart,
    shelving_dart, sorting_dart, ticket_dart, wallet_dart,
};

/// What prints a row: `read`, or `refused` with each message of what `validate()` refused.
const ROW: &str = "
void row(String name, Object? Function() read) {
  try {
    read();
    print('$name: read');
  } on FormatException catch (refused) {
    print('$name: refused');
    for (final violation in refused.message.split('; ')) {
      print('    $violation');
    }
  }
}
";

/// The emitted types of the sets.
fn definitions() -> Vec<String> {
    vec![
        nesting_dart::dart_definition(),
        pouch_dart::dart_definition(),
        point_dart::dart_definition(),
        ticket_dart::dart_definition(),
        folder_dart::dart_definition(),
        every_list_dart::dart_definition(),
        lanyard_dart::dart_definition(),
        shelving_dart::dart_definition(),
        sorting_dart::dart_definition(),
        coin_dart::dart_definition(),
        wallet_dart::dart_definition(),
    ]
}

/// The calls of a `main` that reads each payload with the `fromJson` of its type, then lists what
/// `validate()` answers of a set built with a repeat.
fn calls() -> String {
    let mut written_calls = PAYLOADS.iter().fold(String::new(), |mut written, payload| {
        let converted = if declares_a_parameter(payload.read_as) {
            ", (e) => e as String"
        } else {
            ""
        };
        let _ = writeln!(
            written,
            "  row('{}', () => {}.fromJson(jsonDecode(r'{}'){converted}));",
            payload.name, payload.read_as, payload.json
        );
        written
    });
    written_calls.push_str("  print(Ticket(tags: ['a', 'a']).validate());\n");
    written_calls
}

/// The emitted types, the row printer, and the `main` of [`calls`].
fn source() -> String {
    module(
        &definitions(),
        &[ROW],
        &format!("void main() {{\n{}}}\n", calls()),
    )
}

/// Each payload that repeats an item of a set is refused under the set's place, where Rust's read
/// keeps one item of it, and `validate()` answers the repeat of a set built in code.
#[test]
fn a_set_that_repeats_an_item_is_refused_by_the_dart_read() {
    assert_rust_keeps_one_item();
    let Some((ran, printed)) = dart("sets", &[], &source()) else {
        return;
    };
    assert!(ran, "{printed}");
    assert_eq!(
        printed,
        format!("{PRINTED}['tags': repeats an item: a set holds each item once]\n")
    );
}

/// The analyzer passes the emitted sets.
#[test]
fn the_emitted_sets_have_no_issue() {
    let analyzed = module(
        &definitions(),
        &[ROW],
        "void main() => row('a list', () => jsonDecode('[]'));\n",
    );
    let Some((clean, said)) = dart("sets-analyze", &["analyze"], &analyzed) else {
        return;
    };
    assert!(clean, "{said}");
    assert!(said.contains("No issues found!"), "{said}");
}
