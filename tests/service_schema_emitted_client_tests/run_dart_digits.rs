//! An `i64` read from the string of its digits and written back as it by the emitted Dart types
//! under the Dart VM, against serde, in each place a member holds one, and each payload serde
//! refuses refused with a `FormatException`. The analyzer passes every type.

#![cfg(all(feature = "dart", feature = "serde"))]

use core::fmt::Write as _;

use super::digit_models::{
    REFUSED, WRITTEN, serde_refusal, serde_written, sixty_event_dart, sixty_id_dart,
    sixty_level_dart, sixty_tally_dart,
};
use super::run_dart_validate::{dart, module};

/// What prints a row: the value read, written back, or the class of what the read throws and its
/// message.
const TOLD: &str = "
void told(String at, Object? Function() read) {
  String printed;
  try {
    printed = 'read ${jsonEncode(read())}';
  } on FormatException catch (refusal) {
    printed = '${refusal.runtimeType}: ${refusal.message}';
  } catch (refusal) {
    printed = '${refusal.runtimeType}: $refusal';
  }
  print('$at: $printed');
}
";

/// What the Dart read answers of each payload of [`REFUSED`], in its order.
const DART_REFUSED: [&str; 5] = [
    "FormatException: expected an i64 as a string of its decimal digits, found \
     \"9223372036854775808\"",
    "FormatException: expected an i64 as a string of its decimal digits, found \"seven\"",
    "FormatException: expected an i64 as a string of its decimal digits, found \
     \"-9223372036854775809\"",
    "FormatException: expected an i64 as a string of its decimal digits, found \"1.5\"",
    "FormatException: 'level': too large: maximum is 5, got 6",
];

/// The emitted types every row reads.
fn definitions() -> Vec<String> {
    vec![
        sixty_tally_dart::dart_definition(),
        sixty_id_dart::dart_definition(),
        sixty_event_dart::dart_definition(),
        sixty_level_dart::dart_definition(),
    ]
}

/// What `told` prints of `json` read as `read_as` and written back.
fn told(at: usize, read_as: &str, json: &str) -> String {
    format!("  told('{at}', () => {read_as}.fromJson(jsonDecode(r'''{json}''')).toJson());\n")
}

/// Each payload serde writes is read and written back as serde writes it back, and each payload
/// serde refuses is refused.
#[test]
fn an_i64_is_read_from_its_digits_and_written_as_them_in_dart() {
    let mut main = String::from("void main() {\n");
    let mut expected = String::new();
    let mut at = 0;
    for (read_as, json) in WRITTEN {
        main.push_str(&told(at, read_as, json));
        let _ = writeln!(expected, "{at}: read {}", serde_written(read_as, json));
        at += 1;
    }
    for ((read_as, json, said), refused) in REFUSED.into_iter().zip(DART_REFUSED) {
        assert_eq!(
            serde_refusal(read_as, json).as_deref(),
            Some(said),
            "serde no longer refuses {json}"
        );
        main.push_str(&told(at, read_as, json));
        let _ = writeln!(expected, "{at}: {refused}");
        at += 1;
    }
    main.push_str("}\n");
    let source = module(&definitions(), &[TOLD], &main);
    let Some((ran, printed)) = dart("digits", &[], &source) else {
        return;
    };
    assert!(ran, "{printed}");
    assert_eq!(printed, expected, "the Dart read is not serde's");
}

/// The analyzer passes the types the rows read.
#[test]
fn the_emitted_digit_types_have_no_issue() {
    let source = module(
        &definitions(),
        &[TOLD],
        "void main() => told('id', () => SixtyId.fromJson('1').toJson());",
    );
    let Some((clean, said)) = dart("digits-analyze", &["analyze"], &source) else {
        return;
    };
    assert!(clean, "{said}");
    assert!(said.contains("No issues found!"), "{said}");
}
