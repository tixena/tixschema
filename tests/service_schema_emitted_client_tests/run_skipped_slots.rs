//! Tuple variants with a slot serde neither writes nor reads, read and written back by the emitted
//! Dart, Swift and Kotlin types under their toolchains, beside what serde and the generated Zod
//! schema answer of the same payload: each client declares, reads and writes the slots that
//! remain, and nothing of the slot dropped.

#![cfg(all(
    feature = "serde",
    any(feature = "dart", feature = "swift", feature = "kotlin")
))]

use core::fmt::Write as _;

#[cfg(feature = "dart")]
use super::run_dart_validate::{dart, module};
#[cfg(feature = "swift")]
use super::run_swift_tagged_slots::sorted;
#[cfg(feature = "swift")]
use super::runtime::judged;
#[cfg(feature = "kotlin")]
use super::runtime::ran_kotlin;
#[cfg(all(feature = "zod", feature = "typescript"))]
use super::runtime::{node_modules, ran_with_modules, stand_down_modules};
#[cfg(all(feature = "zod", feature = "typescript"))]
use super::skipped_slot_models::{Bowsed, Clewed, Row};
use super::skipped_slot_models::{ROWS, serde_told};
#[cfg(feature = "dart")]
use super::skipped_slot_models::{bowsed_dart, clewed_dart};
#[cfg(feature = "kotlin")]
use super::skipped_slot_models::{bowsed_kotlin, clewed_kotlin};
#[cfg(feature = "swift")]
use super::skipped_slot_models::{bowsed_swift, clewed_swift};

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

/// serde reads each payload of [`ROWS`] it is said to read and writes it back as it was, and
/// refuses each other one.
#[test]
fn serde_answers_each_payload_as_pinned() {
    for row in &ROWS {
        assert_eq!(
            serde_told(row.read_as, row.json),
            row.told(),
            "serde answers {} otherwise",
            row.json
        );
    }
}

/// The generated Zod schema answers each payload as serde does.
#[cfg(all(feature = "zod", feature = "typescript"))]
#[test]
fn zod_answers_each_payload_as_serde_does() {
    const REQUIRED: &[&str] = &["zod"];
    let Some(modules) = node_modules(REQUIRED) else {
        stand_down_modules(REQUIRED, "the generated Zod schemas of a skipped slot");
        return;
    };
    let mut source = [
        "import { z, type ZodType } from \"zod\";".to_owned(),
        Bowsed::ts_definition(),
        Bowsed::zod_schema(),
        Clewed::ts_definition(),
        Clewed::zod_schema(),
    ]
    .join("\n\n");
    source.push_str("\n\nconst told: string[] = [];\n");
    for row in &ROWS {
        let _ = writeln!(
            source,
            "told.push(((parsed) => parsed.success ? \"read \" + JSON.stringify(parsed.data) : \
             \"refused\")({}$Schema.safeParse(JSON.parse({}))));",
            row.read_as,
            serde_json::to_string(row.json).unwrap()
        );
    }
    source.push_str("console.log(JSON.stringify(told));\n");
    let Some(printed) = ran_with_modules(
        "skipped-slots",
        "TIXSCHEMA_NODE",
        "node",
        "slots.mts",
        &source,
        &modules,
    ) else {
        return;
    };
    let told: Vec<String> = serde_json::from_str(printed.trim()).unwrap();
    let pinned: Vec<String> = ROWS.iter().map(Row::told).collect();
    assert_eq!(told, pinned, "the Zod schema answers otherwise than serde");
}

/// The Dart read reads and writes back each payload serde reads, and refuses each other one.
#[cfg(feature = "dart")]
#[test]
fn dart_reads_and_writes_the_slots_serde_writes() {
    let mut main = String::from("void main() {\n");
    let mut expected = String::new();
    for (at, row) in ROWS.iter().enumerate() {
        let _ = writeln!(
            main,
            "  told('{at}', () => {}.fromJson(jsonDecode(r'''{}''')).toJson());",
            row.read_as, row.json
        );
        let _ = writeln!(expected, "{at}: {}", row.told());
    }
    main.push_str("}\n");
    let source = module(
        &[
            bowsed_dart::dart_definition(),
            clewed_dart::dart_definition(),
        ],
        &[DART_TOLD],
        &main,
    );
    let Some((ran, printed)) = dart("skipped-slots", &[], &source) else {
        return;
    };
    assert!(ran, "{printed}");
    assert_eq!(printed, expected, "the Dart read is not serde's");
    let Some((clean, said)) = dart("skipped-slots-analyze", &["analyze"], &source) else {
        return;
    };
    assert!(clean && said.contains("No issues found!"), "{said}");
}

/// The Dart types declare the slots serde writes alone.
#[cfg(feature = "dart")]
#[test]
fn the_dart_text_declares_no_slot_serde_drops() {
    let bowsed = bowsed_dart::dart_definition();
    for held in [
        "class BowsedDuo extends Bowsed { const BowsedDuo(this.value); final (String,) value;",
        "class BowsedGone extends Bowsed { const BowsedGone(this.value); final List<()> value;",
        "class BowsedLone extends Bowsed { const BowsedLone();",
        "class BowsedPair extends Bowsed { const BowsedPair(this.value); final (String, int) \
         value;",
        "class BowsedTallied extends Bowsed { const BowsedTallied(this.value); final (String, \
         int) value;",
    ] {
        assert!(bowsed.contains(held), "no `{held}` in: {bowsed}");
    }
    assert!(
        !bowsed.contains("Value"),
        "a dropped slot is declared: {bowsed}"
    );
}

/// The Swift read reads and writes back each payload serde reads, and refuses each other one.
#[cfg(feature = "swift")]
#[test]
fn swift_reads_and_writes_the_slots_serde_writes() {
    let mut main = String::new();
    let mut expected = String::new();
    for (at, row) in ROWS.iter().enumerate() {
        let _ = writeln!(main, "told({at}, {}.self, #\"{}\"#)", row.read_as, row.json);
        let told = if row.read {
            format!("read {}", sorted(row.json))
        } else {
            row.told()
        };
        let _ = writeln!(expected, "{at}: {told}");
    }
    let source = [
        "import Foundation".to_owned(),
        bowsed_swift::swift_definition(),
        clewed_swift::swift_definition(),
        SWIFT_TOLD.to_owned(),
        main,
    ]
    .join("\n\n");
    let Some(run) = judged(
        "swift-skipped-slots",
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
        "`swift -warnings-as-errors main.swift` failed.\n--- stdout ---\n{}\n--- stderr ---\n{}\n--- source ---\n{source}",
        run.stdout, run.stderr
    );
    assert_eq!(run.stdout, expected, "the Swift read is not serde's");
}

/// The Swift types declare the slots serde writes alone.
#[cfg(feature = "swift")]
#[test]
fn the_swift_text_declares_no_slot_serde_drops() {
    let bowsed = bowsed_swift::swift_definition();
    for held in [
        "public struct BowsedDuoTuple: Codable, Sendable { let slot0: String;  public \
         init(slot0: String)",
        "public struct BowsedGoneTuple: Codable, Sendable {  public init() {  };",
        "public struct BowsedPairTuple: Codable, Sendable { let slot0: String; let slot1: UInt8; \
         ",
        "public struct BowsedTalliedTuple: Codable, Sendable { let slot0: String; let slot1: \
         UInt8; ",
        "case duo(BowsedDuoTuple); case gone(BowsedGoneTuple); case lone; case \
         open(BowsedOpenPayload);",
    ] {
        assert!(bowsed.contains(held), "no `{held}` in: {bowsed}");
    }
    for dropped in ["UInt64", ": Value"] {
        assert!(
            !bowsed.contains(dropped),
            "a dropped slot is declared as `{dropped}`: {bowsed}"
        );
    }
}

/// The Kotlin read reads and writes back each payload serde reads, and refuses each other one.
#[cfg(feature = "kotlin")]
#[test]
fn kotlin_reads_and_writes_the_slots_serde_writes() {
    let mut main = String::from("fun main() {\n");
    let mut expected = String::new();
    for (at, row) in ROWS.iter().enumerate() {
        let _ = writeln!(
            main,
            "    told({at}, serializer<{}>(), \"\"\"{}\"\"\")",
            row.read_as, row.json
        );
        let _ = writeln!(expected, "{at}: {}", row.told());
    }
    main.push_str("}\n");
    let source = [
        KOTLIN_IMPORTS.to_owned(),
        bowsed_kotlin::kotlin_definition(),
        clewed_kotlin::kotlin_definition(),
        KOTLIN_TOLD.to_owned(),
        main,
    ];
    let Some(printed) = ran_kotlin(&source.join("\n\n")) else {
        return;
    };
    assert_eq!(printed, expected, "the Kotlin read is not serde's");
}

/// The Kotlin types declare the slots serde writes alone.
#[cfg(feature = "kotlin")]
#[test]
fn the_kotlin_text_declares_no_slot_serde_drops() {
    let bowsed = bowsed_kotlin::kotlin_definition();
    // The wrappers are numbered across the items expanded ahead of this one.
    for held in [
        "(val slot0: String) object KotlinTuple",
        "(val slot0: String, val slot1: UByte) object KotlinTuple",
        "data class BowsedDuo(val value: KotlinTuple",
        "data class BowsedGone(val value: @Serializable(with = KotlinArray",
        "data object BowsedLone : Bowsed",
        "data class BowsedPair(val value: KotlinTuple",
        "data class BowsedTallied(val value: KotlinTuple",
    ] {
        assert!(bowsed.contains(held), "no `{held}` in: {bowsed}");
    }
    for dropped in ["Long", "Value"] {
        assert!(
            !bowsed.contains(dropped),
            "a dropped slot is declared as `{dropped}`: {bowsed}"
        );
    }
}
