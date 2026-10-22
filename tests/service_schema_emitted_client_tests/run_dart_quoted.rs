//! Models whose names on the wire hold a `$`, a quote, a double quote and a backslash, read and
//! written back by the emitted Dart types under the Dart VM, against what serde writes: a struct's
//! members, its own tag key and its name, an alias of a member and of a variant, an enum's
//! variants under each tagging, its tag key and its content key, a plain enum's values, and a
//! struct and an externally tagged enum flattened beside a member of their holder's own. A struct's
//! tag of another value, or none, is refused in words that name the key and the tag as serde
//! writes them. The analyzer passes every type.

#![cfg(all(feature = "dart", feature = "serde"))]

use core::fmt::Write as _;

use super::quoted_models::{
    UNDER_AN_ALIAS, WRITTEN, dutied_dart, excised_dart, leveed_dart, levied_dart, mulcted_dart,
    serde_written, tariffed_dart, tithed_dart, tolled_dart,
};
use super::run_dart_validate::{dart, from_until, module};

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

/// A struct's payload with no tag, and with a tag of another value: each refused by the Dart read,
/// as TypeScript and Zod refuse it, where serde reads past the tag.
const REFUSED: [(&str, &str); 2] = [
    (
        r#"{"back\\slash":"b","cost$usd":"c","say \"hi\"":"s"}"#,
        "FormatException: missing field `ki$nd`",
    ),
    (
        r#"{"ki$nd":"Other","back\\slash":"b","cost$usd":"c","say \"hi\"":"s"}"#,
        r#"FormatException: expected tag "Price's$Tag\", found "Other""#,
    ),
];

/// The struct the README's sentence declares, as `quoted_models.rs` declares it.
pub const TARIFFED: &str = concat!(
    r#"#[serde(tag = "ki$nd", rename = "Price's$Tag\\")] struct Tariffed { "#,
    r#"#[serde(rename = "back\\slash")] backslash: String, "#,
    r#"#[serde(rename = "cost$usd", alias = "cost's")] cost: String, "#,
    r#"#[serde(rename = "say \"hi\"")] quoted: String }"#,
);

/// The emitted types every row reads.
fn definitions() -> Vec<String> {
    vec![
        tariffed_dart::dart_definition(),
        tolled_dart::dart_definition(),
        leveed_dart::dart_definition(),
        levied_dart::dart_definition(),
        dutied_dart::dart_definition(),
        excised_dart::dart_definition(),
        tithed_dart::dart_definition(),
        mulcted_dart::dart_definition(),
    ]
}

/// What `told` prints of `json` read as `read_as` and written back.
fn told(at: usize, read_as: &str, json: &str) -> String {
    format!("  told('{at}', () => {read_as}.fromJson(jsonDecode(r'''{json}''')).toJson());\n")
}

/// Each payload serde writes is read by the Dart read and written back as serde wrote it, each
/// payload under an alias is written back under the name serde writes, and a struct's tag of
/// another value, or none, is refused in words that name the key and the tag as serde writes them.
#[test]
fn a_name_that_holds_a_quote_is_read_and_written_back_as_serde_writes_it() {
    let mut main = String::from("void main() {\n");
    let mut expected = String::new();
    let mut at = 0;
    for (read_as, json) in WRITTEN {
        main.push_str(&told(at, read_as, json));
        let _ = writeln!(expected, "{at}: read {}", serde_written(read_as, json));
        at += 1;
    }
    for (read_as, json, back) in UNDER_AN_ALIAS {
        assert_eq!(
            serde_written(read_as, json),
            back,
            "serde no longer reads {json} as {back}"
        );
        main.push_str(&told(at, read_as, json));
        let _ = writeln!(expected, "{at}: read {back}");
        at += 1;
    }
    for (json, refused) in REFUSED {
        main.push_str(&told(at, "Tariffed", json));
        let _ = writeln!(expected, "{at}: {refused}");
        at += 1;
    }
    main.push_str("}\n");
    let source = module(&definitions(), &[TOLD], &main);
    let Some((ran, printed)) = dart("quoted", &[], &source) else {
        return;
    };
    assert!(ran, "{printed}");
    assert_eq!(printed, expected, "the Dart read is not serde's");
}

/// The analyzer passes the types the rows read. `Levied` and `Tithed` are run above alone: the
/// `toJson` of each casts a value whose type the analyzer already knows.
#[test]
fn the_emitted_types_have_no_issue() {
    let source = module(
        &[
            tariffed_dart::dart_definition(),
            tolled_dart::dart_definition(),
            leveed_dart::dart_definition(),
            dutied_dart::dart_definition(),
            excised_dart::dart_definition(),
            mulcted_dart::dart_definition(),
        ],
        &[TOLD],
        r#"void main() => told('calm', () => Tolled.fromJson(jsonDecode(r'"Calm\""')).toJson());"#,
    );
    let Some((clean, said)) = dart("quoted-analyze", &["analyze"], &source) else {
        return;
    };
    assert!(clean, "{said}");
    assert!(said.contains("No issues found!"), "{said}");
}

/// The README's sentence on a name that holds a quote declares the struct the run reads, and shows
/// the payload it reads.
#[test]
fn the_readme_shows_the_payload_the_run_reads() {
    let section = from_until(
        include_str!("../../README.md"),
        "#### The Dart read refuses what `validate()` refuses",
        "#### What stays yours",
    );
    let (read_as, json) = WRITTEN[0];
    assert_eq!(
        read_as, "Tariffed",
        "the first row no longer reads the struct"
    );
    for shown in [TARIFFED, json] {
        assert!(
            section.contains(&format!("`{shown}`")),
            "not in the README: {shown}"
        );
    }
}
