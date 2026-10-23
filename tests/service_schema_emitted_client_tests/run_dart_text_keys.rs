//! A map keyed by a `String` read and written back by the emitted Dart types under the Dart VM,
//! against serde: the map held itself, under an `Option`, in a list and as a map's value, beside a
//! map keyed by a `char`. The read takes each key as the `String` it already is, and the analyzer
//! passes the type.

#![cfg(feature = "dart")]

use alloc::collections::BTreeMap;
use core::fmt::Write as _;

use serde::{Deserialize, Serialize};
use tixschema::model_schema;

use super::run_dart_validate::{dart, module};

/// The payloads read: every map held, the optional map left out and `null`, and a `char` key of
/// two characters, which serde refuses.
const PAYLOADS: [&str; 4] = [
    r#"{"byLetter":{"a":1,"b":2},"countList":[{"x":1},{}],"counts":{"x":1,"y":2},"maybeCounts":{"z":3},"nestedCounts":{"outer":{"inner":4}}}"#,
    r#"{"byLetter":{},"countList":[],"counts":{},"nestedCounts":{}}"#,
    r#"{"byLetter":{},"countList":[],"counts":{},"maybeCounts":null,"nestedCounts":{}}"#,
    r#"{"byLetter":{"ab":1},"countList":[],"counts":{},"nestedCounts":{}}"#,
];

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

/// A map keyed by a `String` in each place a member holds one, and a map keyed by a `char`.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Tallybook {
    pub by_letter: BTreeMap<char, u32>,
    pub count_list: Vec<BTreeMap<String, u32>>,
    pub counts: BTreeMap<String, u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub maybe_counts: Option<BTreeMap<String, u32>>,
    pub nested_counts: BTreeMap<String, BTreeMap<String, u32>>,
}

/// What serde answers of `json`: the value it reads, written back, or `refused`.
fn serde_told(json: &str) -> String {
    serde_json::from_str::<Tallybook>(json).map_or_else(
        |_| "refused".to_owned(),
        |read| format!("read {}", serde_json::to_string(&read).unwrap()),
    )
}

/// Each payload is read and written back by the Dart read as serde reads and writes it, and the
/// `char` key serde refuses is refused.
#[test]
fn a_map_keyed_by_text_is_read_and_written_back_by_dart_as_serde_does() {
    let mut main = String::from("void main() {\n");
    let mut expected = String::new();
    for (at, json) in PAYLOADS.iter().enumerate() {
        let _ = writeln!(
            main,
            "  told('{at}', () => Tallybook.fromJson(jsonDecode(r'{json}')).toJson());"
        );
        let _ = writeln!(expected, "{at}: {}", serde_told(json));
    }
    main.push_str("}\n");
    assert!(expected.ends_with("3: refused\n"), "{expected}");
    let source = module(&[tallybook_dart::dart_definition()], &[TOLD], &main);
    let Some((ran, printed)) = dart("text-keys", &[], &source) else {
        return;
    };
    assert!(ran, "{printed}");
    assert_eq!(printed, expected, "the Dart read is not serde's");
}

/// The analyzer passes the type: no key is cast to the `String` it already is.
#[test]
fn the_emitted_type_has_no_issue() {
    let source = module(
        &[tallybook_dart::dart_definition()],
        &[TOLD],
        &format!(
            "void main() => told('0', () => Tallybook.fromJson(jsonDecode(r'{}')).toJson());\n",
            PAYLOADS[0]
        ),
    );
    let Some((clean, said)) = dart("text-keys-analyze", &["analyze"], &source) else {
        return;
    };
    assert!(clean, "{said}");
    assert!(said.contains("No issues found!"), "{said}");
}
