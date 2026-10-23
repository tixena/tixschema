//! Untagged enums with a newtype variant over an `Option`, read by the generated Zod schema as the
//! value serde writes back and read and written back by the emitted Dart, Swift and Kotlin types
//! under their toolchains, beside what serde answers of the same payload: each client reads `null`
//! as the first variant whose payload reads it, the variant over an `Option` holding nothing among
//! them, and writes it back as `null`.

#![cfg(all(
    feature = "serde",
    any(
        all(feature = "zod", feature = "typescript"),
        feature = "dart",
        feature = "swift",
        feature = "kotlin"
    )
))]

use core::fmt::Write as _;

#[cfg(all(feature = "zod", feature = "typescript"))]
use super::null_models::{Drogue, Drogued, Fathom, Kedged, Plumb, sounded_schema};
use super::null_models::{NULL_ROWS, NullRow, serde_told};
#[cfg(feature = "dart")]
use super::null_models::{
    drogue_dart, drogued_dart, fathom_dart, kedged_dart, plumb_dart, sounded_dart,
};
#[cfg(feature = "kotlin")]
use super::null_models::{
    drogue_kotlin, drogued_kotlin, fathom_kotlin, kedged_kotlin, plumb_kotlin, sounded_kotlin,
};
#[cfg(feature = "swift")]
use super::null_models::{
    drogue_swift, drogued_swift, fathom_swift, kedged_swift, plumb_swift, sounded_swift,
};
#[cfg(feature = "dart")]
use super::run_dart_validate::{dart, module};
#[cfg(feature = "swift")]
use super::runtime::judged;
#[cfg(feature = "kotlin")]
use super::runtime::ran_kotlin;
#[cfg(all(feature = "zod", feature = "typescript"))]
use super::runtime::{node_modules, ran_with_modules, stand_down_modules};

/// Prints a row: the class of the value read and the value written back, or `refused`.
#[cfg(feature = "dart")]
const DART_TOLD: &str = "
void told(String at, Object Function() read) {
  String printed;
  try {
    final dynamic value = read();
    printed = 'read ${value.runtimeType} ${jsonEncode(value.toJson())}';
  } catch (_) {
    printed = 'refused';
  }
  print('$at: $printed');
}
";

/// Prints a row: the case read and the value written back, or `refused`.
#[cfg(feature = "swift")]
const SWIFT_TOLD: &str = r#"
func told<Read: Codable>(_ at: Int, _ type: Read.Type, _ payload: String) {
  do {
    let read = try JSONDecoder().decode(type, from: Data(payload.utf8))
    let taken = Mirror(reflecting: read).children.first?.label ?? "\(read)"
    print("\(at): read \(taken) \(String(data: try JSONEncoder().encode(read), encoding: .utf8)!)")
  } catch {
    print("\(at): refused")
  }
}
"#;

/// Prints a row: the class of the value read and the value written back, or `refused`.
#[cfg(feature = "kotlin")]
const KOTLIN_TOLD: &str = r#"
fun <T : Any> told(at: Int, serializer: KSerializer<T>, payload: String) {
    val told = try {
        val read = Json.decodeFromString(serializer, payload)
        "read " + read::class.simpleName + " " + Json.encodeToString(serializer, read)
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

/// Holds `row` to what serde answers of its payload.
fn held_to_serde(row: &NullRow) {
    assert_eq!(
        serde_told(row.read_as, row.json),
        row.told(str::to_owned),
        "serde answers {} read as {} otherwise",
        row.json,
        row.read_as
    );
}

/// What a client prints of [`NULL_ROWS`], one line per row, each variant as `named` names it, held
/// to what serde answers of the same payload.
#[cfg(any(feature = "dart", feature = "swift", feature = "kotlin"))]
fn expected<F>(named: F) -> String
where
    F: Fn(&NullRow, &str) -> String,
{
    NULL_ROWS
        .iter()
        .enumerate()
        .fold(String::new(), |mut lines, (at, row)| {
            held_to_serde(row);
            let _ = writeln!(lines, "{at}: {}", row.told(|taken| named(row, taken)));
            lines
        })
}

/// The class Dart and Kotlin declare the variant `taken` of the enum `row` is read as under.
#[cfg(any(feature = "dart", feature = "kotlin"))]
fn variant_class(row: &NullRow, taken: &str) -> String {
    format!("{}{taken}", row.read_as)
}

/// The generated Zod schema reads each payload serde reads as the value serde writes back, `null`
/// as `null`, and refuses each other one.
#[cfg(all(feature = "zod", feature = "typescript"))]
#[test]
fn zod_reads_each_payload_as_the_value_serde_writes_back() {
    const REQUIRED: &[&str] = &["zod"];
    let Some(modules) = node_modules(REQUIRED) else {
        stand_down_modules(
            REQUIRED,
            "the generated Zod schemas of an untagged `Option`",
        );
        return;
    };
    let mut source = [
        "import { z, type ZodType } from \"zod\";".to_owned(),
        Fathom::ts_definition(),
        Fathom::zod_schema(),
        Kedged::ts_definition(),
        Kedged::zod_schema(),
        Drogue::ts_definition(),
        Drogue::zod_schema(),
        Drogued::ts_definition(),
        Drogued::zod_schema(),
        sounded_schema::Schema::ts_definition(),
        sounded_schema::Schema::zod_schema(),
        Plumb::ts_definition(),
        Plumb::zod_schema(),
    ]
    .join("\n\n");
    source.push_str("\n\nconst told: string[] = [];\n");
    for row in &NULL_ROWS {
        let _ = writeln!(
            source,
            "{{ const read = {}$Schema.safeParse(JSON.parse({})); told.push(read.success ? \
             \"read \" + JSON.stringify(read.data) : \"refused\"); }}",
            row.read_as,
            serde_json::to_string(row.json).unwrap()
        );
    }
    source.push_str("console.log(JSON.stringify(told));\n");
    let Some(printed) = ran_with_modules(
        "untagged-nulls",
        "TIXSCHEMA_NODE",
        "node",
        "nulls.mts",
        &source,
        &modules,
    ) else {
        return;
    };
    let told: Vec<String> = serde_json::from_str(printed.trim()).unwrap();
    // serde writes back each payload it reads as it was.
    let pinned: Vec<String> = NULL_ROWS
        .iter()
        .map(|row| {
            held_to_serde(row);
            row.taken
                .map_or_else(|| "refused".to_owned(), |_| format!("read {}", row.json))
        })
        .collect();
    assert_eq!(told, pinned, "the Zod schema answers otherwise than serde");
}

/// The Dart read takes the variant serde takes for each payload and writes it back, and refuses
/// each payload serde refuses.
#[cfg(feature = "dart")]
#[test]
fn dart_reads_null_as_the_variant_serde_takes() {
    let mut main = String::from("void main() {\n");
    for (at, row) in NULL_ROWS.iter().enumerate() {
        let _ = writeln!(
            main,
            "  told('{at}', () => {}.fromJson(jsonDecode(r'''{}''')));",
            row.read_as, row.json
        );
    }
    main.push_str("}\n");
    let source = module(
        &[
            fathom_dart::dart_definition(),
            kedged_dart::dart_definition(),
            drogue_dart::dart_definition(),
            drogued_dart::dart_definition(),
            sounded_dart::dart_definition(),
            plumb_dart::dart_definition(),
        ],
        &[DART_TOLD],
        &main,
    );
    let Some((ran, printed)) = dart("untagged-nulls", &[], &source) else {
        return;
    };
    assert!(ran, "{printed}");
    assert_eq!(
        printed,
        expected(variant_class),
        "the Dart read is not serde's"
    );
}

/// The Swift read takes the variant serde takes for each payload and writes it back, and refuses
/// each payload serde refuses.
#[cfg(feature = "swift")]
#[test]
fn swift_reads_null_as_the_variant_serde_takes() {
    let mut main = String::new();
    for (at, row) in NULL_ROWS.iter().enumerate() {
        let _ = writeln!(main, "told({at}, {}.self, #\"{}\"#)", row.read_as, row.json);
    }
    let source = [
        "import Foundation".to_owned(),
        fathom_swift::swift_definition(),
        kedged_swift::swift_definition(),
        drogue_swift::swift_definition(),
        drogued_swift::swift_definition(),
        sounded_swift::swift_definition(),
        plumb_swift::swift_definition(),
        SWIFT_TOLD.to_owned(),
        main,
    ]
    .join("\n\n");
    let Some(run) = judged(
        "swift-untagged-nulls",
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
    // Swift declares a variant as a case named in camel case.
    let case = |_: &NullRow, taken: &str| {
        let mut letters = taken.chars();
        letters.next().map_or_else(String::new, |first| {
            first.to_lowercase().chain(letters).collect()
        })
    };
    assert_eq!(run.stdout, expected(case), "the Swift read is not serde's");
}

/// The Swift read of a newtype variant over an `Option` tells a read that answered `nil` from one
/// that threw, beside a bound, through a reader and over an alias of an `Option`. A variant over
/// any other type keeps its attempt.
#[cfg(feature = "swift")]
#[test]
fn the_swift_read_of_a_variant_over_an_option_takes_the_nil_it_reads() {
    for (swift, attempts) in [
        (
            fathom_swift::swift_definition(),
            [
                "do { let value = try UInt32?(from: decoder); self = .count(value); return } \
                 catch {}; ",
                "if let value = try? Bool(from: decoder) { self = .flag(value); return }; ",
            ],
        ),
        (
            kedged_swift::swift_definition(),
            [
                "do { let value = try UInt32?(from: decoder); if \
                 Kedged.swiftSchemaBoundAbove(value).isEmpty { self = .above(value); return } } \
                 catch {}; ",
                "if let value = try? String(from: decoder) { self = .text(value); return }; ",
            ],
        ),
        (
            drogued_swift::swift_definition(),
            [
                "do { let value = try Drogued.swiftSchemaReadHeld(decoder); self = .held(value); \
                 return } catch {}; ",
                "do { let value = try Drogued.swiftSchemaReadListed(decoder); self = \
                 .listed(value); return } catch {}; ",
            ],
        ),
        (
            plumb_swift::swift_definition(),
            [
                "do { let value = try SoundedType(from: decoder); self = .depth(value); return } \
                 catch {}; ",
                "if let value = try? Bool(from: decoder) { self = .flag(value); return }; ",
            ],
        ),
    ] {
        for attempt in attempts {
            assert!(swift.contains(attempt), "no `{attempt}` in: {swift}");
        }
    }
}

/// The Kotlin read takes the variant serde takes for each payload and writes it back, and refuses
/// each payload serde refuses.
#[cfg(feature = "kotlin")]
#[test]
fn kotlin_reads_null_as_the_variant_serde_takes() {
    let mut main = String::from("fun main() {\n");
    for (at, row) in NULL_ROWS.iter().enumerate() {
        let _ = writeln!(
            main,
            "    told({at}, serializer<{}>(), \"\"\"{}\"\"\")",
            row.read_as, row.json
        );
    }
    main.push_str("}\n");
    let source = [
        KOTLIN_IMPORTS.to_owned(),
        fathom_kotlin::kotlin_definition(),
        kedged_kotlin::kotlin_definition(),
        drogue_kotlin::kotlin_definition(),
        drogued_kotlin::kotlin_definition(),
        sounded_kotlin::kotlin_definition(),
        plumb_kotlin::kotlin_definition(),
        KOTLIN_TOLD.to_owned(),
        main,
    ];
    let Some(printed) = ran_kotlin(&source.join("\n\n")) else {
        return;
    };
    assert_eq!(
        printed,
        expected(variant_class),
        "the Kotlin read is not serde's"
    );
}
