//! A set read and written by the emitted Dart types at one list level, as serde writes it, under
//! the analyzer and the Dart VM.

#![cfg(feature = "dart")]

use core::fmt::Write as _;

use super::run_dart_validate::{dart, module};
#[cfg(feature = "indexmap")]
use super::set_level_models::indexed_dart;
use super::set_level_models::{BROKEN, BROKEN_MESSAGE, ROUND_TRIPS, assert_rust_writes_back};
use super::set_level_models::{
    entry_dart, groups_dart, probe_sets_dart, shelf_dart, wrappers_dart,
};

const ROW: &str = "
void row(String name, Object? Function() read) {
  try {
    print('$name: ${jsonEncode(read())}');
  } on FormatException catch (refused) {
    print('$name: refused');
    for (final violation in refused.message.split('; ')) {
      print('    $violation');
    }
  }
}
";

fn definitions() -> Vec<String> {
    let mut written = vec![
        entry_dart::dart_definition(),
        groups_dart::dart_definition(),
        probe_sets_dart::dart_definition(),
        shelf_dart::dart_definition(),
        wrappers_dart::dart_definition(),
    ];
    #[cfg(feature = "indexmap")]
    written.push(indexed_dart::dart_definition());
    written
}

fn source() -> String {
    let mut main = String::from("void main() {\n");
    for &(name, json) in ROUND_TRIPS {
        let _ = writeln!(
            main,
            "  row('{name}', () => {name}.fromJson(jsonDecode(r'{json}')).toJson());"
        );
    }
    let (broken_name, broken_json) = BROKEN;
    let _ = writeln!(
        main,
        "  row('broken', () => {broken_name}.fromJson(jsonDecode(r'{broken_json}')).toJson());"
    );
    main.push_str(
        "  print(Shelf(entries: [Entry(name: 'a')], keyed: {}, maybe: null, nullable: [], \
         slot: ([], 1)).validate());\n}\n",
    );
    module(&definitions(), &[ROW], &main)
}

#[test]
fn a_set_is_one_list_level_in_the_dart_types() {
    assert_rust_writes_back();
    let Some((ran, printed)) = dart("set-levels", &[], &source()) else {
        return;
    };
    assert!(ran, "{printed}");
    let mut expected = String::new();
    for &(name, json) in ROUND_TRIPS {
        let _ = writeln!(expected, "{name}: {json}");
    }
    let _ = write!(
        expected,
        "broken: refused\n    {BROKEN_MESSAGE}\n[{BROKEN_MESSAGE}]\n"
    );
    assert_eq!(printed, expected);
}

#[test]
fn the_emitted_set_levels_have_no_issue() {
    let Some((clean, said)) = dart("set-levels-analyze", &["analyze"], &source()) else {
        return;
    };
    assert!(clean, "{said}");
    assert!(said.contains("No issues found!"), "{said}");
}
