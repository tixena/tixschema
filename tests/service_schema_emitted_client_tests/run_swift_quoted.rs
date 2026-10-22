//! Models whose names on the wire hold a `$`, a quote, a double quote and a backslash, read and
//! written back by the emitted Swift types compiled with warnings as errors, against what serde
//! writes: a struct's members, its own tag key and its name, an alias of a member and of a
//! variant, an enum's variants under each tagging, its tag key and its content key, a plain enum's
//! values, and a struct and an externally tagged enum flattened beside a member of their holder's
//! own. A struct's tag of another value, or none, is refused in words that name the key and the
//! tag as serde writes them. The models are declared where the Dart client is built.

#![cfg(all(feature = "swift", feature = "dart", feature = "serde"))]

use alloc::collections::BTreeMap;
use core::fmt::Write as _;

use serde_json::Value;

use super::quoted_models::{
    UNDER_AN_ALIAS, WRITTEN, dutied_swift, excised_swift, leveed_swift, levied_swift,
    mulcted_swift, serde_written, tariffed_swift, tithed_swift, tolled_swift,
};
use super::run_dart_quoted::TARIFFED;
use super::run_dart_validate::from_until;
use super::runtime::judged;

/// Names the runtime to run, for a machine that has one somewhere other than `PATH`.
const RUNTIME_VAR: &str = "TIXSCHEMA_SWIFT";

/// What prints a row: the value read, written back with its keys sorted, or the description of
/// what the read throws.
const TOLD: &str = r#"
func told<Read: Codable>(_ at: Int, _ type: Read.Type, _ payload: String) {
  do {
    let encoder = JSONEncoder()
    encoder.outputFormatting = [.sortedKeys, .withoutEscapingSlashes]
    let read = try JSONDecoder().decode(type, from: Data(payload.utf8))
    print("\(at): read \(String(data: try encoder.encode(read), encoding: .utf8)!)")
  } catch DecodingError.dataCorrupted(let context) {
    print("\(at): \(context.debugDescription)")
  } catch {
    print("\(at): \(error)")
  }
}
"#;

/// A struct's payload with no tag, and with a tag of another value: each refused by the Swift
/// read, as TypeScript and Zod refuse it, where serde reads past the tag.
const REFUSED: [(&str, &str); 2] = [
    (
        r#"{"back\\slash":"b","cost$usd":"c","say \"hi\"":"s"}"#,
        "missing field `ki$nd`",
    ),
    (
        r#"{"ki$nd":"Other","back\\slash":"b","cost$usd":"c","say \"hi\"":"s"}"#,
        r#"expected tag "Price's$Tag\", found "Other""#,
    ),
];

/// `json` with the keys of every object sorted, as the Swift encoder of the run writes them.
fn sorted(json: &str) -> String {
    /// `value` with the keys of every object it holds sorted.
    fn sorting(value: Value) -> Value {
        match value {
            Value::Object(entries) => {
                let keyed: BTreeMap<String, Value> = entries
                    .into_iter()
                    .map(|(key, held)| (key, sorting(held)))
                    .collect();
                Value::Object(keyed.into_iter().collect())
            }
            Value::Array(items) => Value::Array(items.into_iter().map(sorting).collect()),
            held @ (Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_)) => held,
        }
    }
    serde_json::to_string(&sorting(serde_json::from_str(json).unwrap())).unwrap()
}

/// Each payload serde writes is read by the Swift read and written back as serde writes it, each
/// payload under an alias is written back under the name serde writes, and a struct's tag of
/// another value, or none, is refused in words that name the key and the tag as serde writes them.
#[test]
fn a_name_that_holds_a_quote_is_read_and_written_back_as_serde_writes_it() {
    let mut main = String::new();
    let mut expected = String::new();
    let mut at = 0_usize;
    for (read_as, json) in WRITTEN {
        let _ = writeln!(main, "told({at}, {read_as}.self, #\"{json}\"#)");
        let _ = writeln!(
            expected,
            "{at}: read {}",
            sorted(&serde_written(read_as, json))
        );
        at += 1;
    }
    for (read_as, json, back) in UNDER_AN_ALIAS {
        assert_eq!(
            serde_written(read_as, json),
            back,
            "serde no longer reads {json} as {back}"
        );
        let _ = writeln!(main, "told({at}, {read_as}.self, #\"{json}\"#)");
        let _ = writeln!(expected, "{at}: read {}", sorted(back));
        at += 1;
    }
    for (json, refused) in REFUSED {
        let _ = writeln!(main, "told({at}, Tariffed.self, #\"{json}\"#)");
        let _ = writeln!(expected, "{at}: {refused}");
        at += 1;
    }
    let source = [
        "import Foundation".to_owned(),
        tariffed_swift::swift_definition(),
        tolled_swift::swift_definition(),
        leveed_swift::swift_definition(),
        levied_swift::swift_definition(),
        dutied_swift::swift_definition(),
        excised_swift::swift_definition(),
        tithed_swift::swift_definition(),
        mulcted_swift::swift_definition(),
        TOLD.to_owned(),
        main,
    ]
    .join("\n\n");
    let Some(run) = judged(
        "swift-quoted",
        RUNTIME_VAR,
        "swift",
        &["-warnings-as-errors"],
        "main.swift",
        &source,
    ) else {
        return;
    };
    assert!(
        run.succeeded,
        "`swift -warnings-as-errors main.swift` failed.\n--- stdout ---\n{}\n--- stderr ---\n{}\n--- source ---\n{source}",
        run.stdout, run.stderr
    );
    assert_eq!(run.stdout, expected, "the Swift read is not serde's");
}

/// The README's sentence on a name that holds a quote declares the struct the run reads, and shows
/// the key it declares and the payload it reads.
#[test]
fn the_readme_shows_the_payload_the_run_reads() {
    let section = from_until(
        include_str!("../../README.md"),
        "#### A Swift read refuses what `validate()` refuses",
        "### Literal Values",
    );
    let declared = r#"case quoted = "say \"hi\"""#;
    assert!(
        tariffed_swift::swift_definition().contains(declared),
        "the struct no longer declares {declared}"
    );
    let (read_as, json) = WRITTEN[0];
    assert_eq!(
        read_as, "Tariffed",
        "the first row no longer reads the struct"
    );
    for shown in [TARIFFED, declared, json] {
        assert!(
            section.contains(&format!("`{shown}`")),
            "not in the README: {shown}"
        );
    }
}
