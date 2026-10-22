//! A date under `as_number` in a member of an untagged enum, read and written back by the emitted
//! Dart, Swift and Kotlin types under their toolchains: each reads the epoch milliseconds serde
//! writes, writes them back as serde does, and refuses each payload serde refuses.

#![cfg(any(feature = "dart", feature = "swift", feature = "kotlin"))]

use core::fmt::Write as _;

#[cfg(feature = "dart")]
use super::run_dart_validate::{dart, module};
#[cfg(feature = "swift")]
use super::runtime::judged;
#[cfg(feature = "kotlin")]
use super::runtime::ran_kotlin;
use super::untagged_millis_models::{MILLIS_ROWS, serde_written};
#[cfg(feature = "dart")]
use super::untagged_millis_models::{
    millis_later_dart, millis_maybe_dart, millis_named_dart, millis_or_flag_dart,
};
#[cfg(feature = "kotlin")]
use super::untagged_millis_models::{
    millis_later_kotlin, millis_maybe_kotlin, millis_named_kotlin, millis_or_flag_kotlin,
};
#[cfg(feature = "swift")]
use super::untagged_millis_models::{
    millis_later_swift, millis_maybe_swift, millis_named_swift, millis_or_flag_swift,
};

/// Prints a row: the value read and written back, or `refused`.
#[cfg(feature = "dart")]
const DART_TOLD: &str = "
void told(String at, Object? Function() read) {
  String printed;
  try {
    printed = 'read ${jsonEncode(read())}';
  } catch (_) {
    printed = 'refused';
  }
  print('$at: $printed');
}
";

/// Prints a row: the value read and written back with its keys sorted, or `refused`.
#[cfg(feature = "swift")]
const SWIFT_TOLD: &str = r#"
func told<Read: Codable>(_ at: Int, _ type: Read.Type, _ payload: String) {
  do {
    let encoder = JSONEncoder()
    encoder.outputFormatting = [.sortedKeys, .withoutEscapingSlashes]
    let read = try JSONDecoder().decode(type, from: Data(payload.utf8))
    print("\(at): read \(String(data: try encoder.encode(read), encoding: .utf8)!)")
  } catch {
    print("\(at): refused")
  }
}
"#;

/// Prints a row: the value read and written back, or `refused`.
#[cfg(feature = "kotlin")]
const KOTLIN_TOLD: &str = r#"
fun <T> told(at: Int, serializer: KSerializer<T>, payload: String) {
    val told = try {
        "read " + Json.encodeToString(serializer, Json.decodeFromString(serializer, payload))
    } catch (refusal: Exception) {
        if (refusal is SerializationException || refusal is IllegalArgumentException) "refused"
        else throw refusal
    }
    println("$at: $told")
}
"#;

/// The imports the emitted Kotlin and its driver reach for.
#[cfg(feature = "kotlin")]
const KOTLIN_IMPORTS: &str = "import kotlinx.serialization.*\n\
     import kotlinx.serialization.json.*\n\
     import kotlinx.serialization.descriptors.*\n\
     import kotlinx.serialization.encoding.*\n\
     import kotlinx.serialization.builtins.*";

/// What each client prints of [`MILLIS_ROWS`], one line per row, each held to what serde answers
/// of the same payload.
fn expected() -> String {
    MILLIS_ROWS.iter().enumerate().fold(
        String::new(),
        |mut lines, (at, (read_as, json, written, _))| {
            assert_eq!(
                serde_written(read_as, json).as_deref(),
                *written,
                "serde answers {json} read as {read_as} otherwise"
            );
            let told = written.map_or_else(|| "refused".to_owned(), |back| format!("read {back}"));
            let _ = writeln!(lines, "{at}: {told}");
            lines
        },
    )
}

/// The Dart read reads and writes back each payload serde reads, and refuses each other one.
#[cfg(feature = "dart")]
#[test]
fn dart_reads_and_writes_the_millis_serde_writes_in_an_untagged_member() {
    let mut main = String::from("void main() {\n");
    for (at, (read_as, json, _, _)) in MILLIS_ROWS.iter().enumerate() {
        let _ = writeln!(
            main,
            "  told('{at}', () => {read_as}.fromJson(jsonDecode(r'''{json}''')).toJson());"
        );
    }
    main.push_str("}\n");
    let source = module(
        &[
            millis_or_flag_dart::dart_definition(),
            millis_named_dart::dart_definition(),
            millis_maybe_dart::dart_definition(),
            millis_later_dart::dart_definition(),
        ],
        &[DART_TOLD],
        &main,
    );
    let Some((ran, printed)) = dart("untagged-millis", &[], &source) else {
        return;
    };
    assert!(ran, "{printed}");
    assert_eq!(printed, expected(), "the Dart read is not serde's");
}

/// The Swift read reads and writes back each payload serde reads, and refuses each other one.
#[cfg(feature = "swift")]
#[test]
fn swift_reads_and_writes_the_millis_serde_writes_in_an_untagged_member() {
    let mut main = String::new();
    for (at, (read_as, json, _, _)) in MILLIS_ROWS.iter().enumerate() {
        let _ = writeln!(main, "told({at}, {read_as}.self, #\"{json}\"#)");
    }
    let source = [
        "import Foundation".to_owned(),
        millis_or_flag_swift::swift_definition(),
        millis_named_swift::swift_definition(),
        millis_maybe_swift::swift_definition(),
        millis_later_swift::swift_definition(),
        SWIFT_TOLD.to_owned(),
        main,
    ]
    .join("\n\n");
    let Some(run) = judged(
        "swift-untagged-millis",
        "TIXSCHEMA_SWIFT",
        "swift",
        &["-warnings-as-errors"],
        "main.swift",
        &source,
    ) else {
        return;
    };
    assert!(
        run.succeeded,
        "`swift -warnings-as-errors main.swift` failed.\n--- stdout ---\n{}\n--- stderr \
         ---\n{}\n--- source ---\n{source}",
        run.stdout, run.stderr
    );
    assert_eq!(run.stdout, expected(), "the Swift read is not serde's");
}

/// The Kotlin read reads and writes back each payload serde reads, and refuses each other one.
#[cfg(feature = "kotlin")]
#[test]
fn kotlin_reads_and_writes_the_millis_serde_writes_in_an_untagged_member() {
    let mut main = String::from("fun main() {\n");
    for (at, (read_as, json, _, _)) in MILLIS_ROWS.iter().enumerate() {
        let _ = writeln!(
            main,
            "    told({at}, serializer<{read_as}>(), \"\"\"{json}\"\"\")"
        );
    }
    main.push_str("}\n");
    let source = [
        KOTLIN_IMPORTS.to_owned(),
        millis_or_flag_kotlin::kotlin_definition(),
        millis_named_kotlin::kotlin_definition(),
        millis_maybe_kotlin::kotlin_definition(),
        millis_later_kotlin::kotlin_definition(),
        KOTLIN_TOLD.to_owned(),
        main,
    ];
    let Some(printed) = ran_kotlin(&source.join("\n\n")) else {
        return;
    };
    assert_eq!(printed, expected(), "the Kotlin read is not serde's");
}
