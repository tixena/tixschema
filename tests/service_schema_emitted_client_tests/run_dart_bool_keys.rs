//! A map keyed by a `bool` read and written back by the emitted Dart types under the Dart VM,
//! against serde: held by a field, a tuple's slot, a list, an optional and another map's value,
//! and in a variant's slot under no tag attribute and under a tag and a content key, the taggings
//! serde reads a map under. A key that is neither `true` nor `false` is refused, as serde refuses
//! it.

#![cfg(feature = "dart")]

use core::fmt::Write as _;

use super::bool_key_rows::{adjacent_drawer_dart, cubby_dart, cubby_json, drawer_dart, rows};
use super::run_dart_validate::{dart, module};

/// What prints a row: the value read and written back, or `refused`.
const TOLD: &str = "
void told(String at, Object? Function() read) {
  String printed;
  try {
    printed = 'read ${jsonEncode(read())}';
  } on FormatException catch (_) {
    printed = 'refused';
  }
  print('$at: $printed');
}
";

fn definitions() -> Vec<String> {
    vec![
        cubby_dart::dart_definition(),
        drawer_dart::dart_definition(),
        adjacent_drawer_dart::dart_definition(),
    ]
}

/// Each payload is read and written back by the Dart read as serde reads and writes it, and a key
/// that is neither `true` nor `false` is refused in every position.
#[test]
fn a_map_keyed_by_bool_is_read_and_written_back_by_dart_as_serde_does() {
    let rows = rows();
    let mut main = String::from("void main() {\n");
    let mut expected = String::new();
    for (at, row) in rows.iter().enumerate() {
        let _ = writeln!(
            main,
            "  told('{at}', () => {}.fromJson(jsonDecode(r'{}')).toJson());",
            row.named, row.json
        );
        let _ = writeln!(expected, "{at}: {}", row.serde);
    }
    main.push_str("}\n");
    assert!(expected.contains("refused"), "{expected}");
    let source = module(&definitions(), &[TOLD], &main);
    let Some((ran, printed)) = dart("bool-keys", &[], &source) else {
        return;
    };
    assert!(ran, "{printed}");
    assert_eq!(printed, expected, "the Dart read is not serde's");
}

/// The analyzer passes the types.
#[test]
fn the_emitted_types_have_no_issue() {
    let source = module(
        &definitions(),
        &[TOLD],
        &format!(
            "void main() => told('0', () => Cubby.fromJson(jsonDecode(r'{}')).toJson());\n",
            cubby_json(r#"{"false":"a","true":"b"}"#)
        ),
    );
    let Some((clean, said)) = dart("bool-keys-analyze", &["analyze"], &source) else {
        return;
    };
    assert!(clean, "{said}");
    assert!(said.contains("No issues found!"), "{said}");
}

/// The read of a `bool` key names the two keys it accepts and refuses any other.
#[test]
fn the_emitted_read_of_a_bool_key_refuses_any_other_text() {
    let written = cubby_dart::dart_definition();
    assert!(
        written.contains(
            "switch (e.key) { \"true\" => true, \"false\" => false, final refused => throw \
             FormatException('Invalid bool key: $refused') }"
        ),
        "{written}"
    );
    assert!(!written.contains("== \"true\""), "{written}");
}
