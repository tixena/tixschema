//! Each enum of [`empty_variant_models`](super::empty_variant_models) read by Zod under node and by
//! the emitted Dart, Kotlin and Swift types: every payload serde reads is read and written back as
//! serde writes it, and a key no member declares is refused by a strict enum and read past by a
//! `nonstrict` one, as serde does.

use core::fmt::Write as _;

#[cfg(feature = "dart")]
use super::empty_variant_models::{
    adjacent_signal_dart, hidden_adjacent_signal_dart, hidden_signal_dart,
    hidden_tagged_signal_dart, hidden_untagged_signal_dart, open_adjacent_signal_dart,
    open_hidden_adjacent_signal_dart, open_hidden_signal_dart, open_hidden_tagged_signal_dart,
    open_hidden_untagged_signal_dart, open_signal_dart, open_tagged_signal_dart,
    open_untagged_signal_dart, signal_dart, tagged_signal_dart, untagged_signal_dart,
};
#[cfg(feature = "kotlin")]
use super::empty_variant_models::{
    adjacent_signal_kotlin, hidden_adjacent_signal_kotlin, hidden_signal_kotlin,
    hidden_tagged_signal_kotlin, hidden_untagged_signal_kotlin, open_adjacent_signal_kotlin,
    open_hidden_adjacent_signal_kotlin, open_hidden_signal_kotlin,
    open_hidden_tagged_signal_kotlin, open_hidden_untagged_signal_kotlin, open_signal_kotlin,
    open_tagged_signal_kotlin, open_untagged_signal_kotlin, signal_kotlin, tagged_signal_kotlin,
    untagged_signal_kotlin,
};
#[cfg(feature = "swift")]
use super::empty_variant_models::{
    adjacent_signal_swift, hidden_adjacent_signal_swift, hidden_signal_swift,
    hidden_tagged_signal_swift, hidden_untagged_signal_swift, open_adjacent_signal_swift,
    open_hidden_adjacent_signal_swift, open_hidden_signal_swift, open_hidden_tagged_signal_swift,
    open_hidden_untagged_signal_swift, open_signal_swift, open_tagged_signal_swift,
    open_untagged_signal_swift, signal_swift, tagged_signal_swift, untagged_signal_swift,
};
use super::empty_variant_models::{expected, objects, rows, ts_definitions};
#[cfg(feature = "dart")]
use super::run_dart_validate::{dart, module};
#[cfg(feature = "swift")]
use super::runtime::judged;
#[cfg(feature = "kotlin")]
use super::runtime::ran_kotlin;
use super::runtime::{node_modules, ran_with_modules, stand_down_modules};

const REQUIRED_PACKAGES: &[&str] = &["zod"];

#[test]
fn zod_reads_each_enum_as_serde_reads_it() {
    let Some(modules) = node_modules(REQUIRED_PACKAGES) else {
        stand_down_modules(REQUIRED_PACKAGES, "Zod's read of the empty variants");
        return;
    };
    let mut main = String::new();
    for (at, (name, json)) in rows().into_iter().enumerate() {
        let _ = writeln!(
            main,
            "{{ const read = {name}$Schema.safeParse(JSON.parse({})); console.log(\"{at}: \" + (read.success ? JSON.stringify(read.data) : \"refused\")); }}",
            serde_json::to_string(json).unwrap()
        );
    }
    let source = format!(
        "import {{ z, type ZodType }} from \"zod\";\n\n{}\n\n{main}",
        ts_definitions().join("\n\n")
    );
    let Some(printed) = ran_with_modules(
        "empty-variants",
        "TIXSCHEMA_NODE",
        "node",
        "empty-variants.mts",
        &source,
        &modules,
    ) else {
        return;
    };
    assert_eq!(objects(&printed), expected());
}

#[cfg(feature = "dart")]
#[test]
fn the_dart_read_reads_each_enum_as_serde_reads_it() {
    let mut main = String::from("void main() {\n");
    for (at, (name, json)) in rows().into_iter().enumerate() {
        let _ = writeln!(
            main,
            "  try {{ print('{at}: ${{jsonEncode({name}.fromJson(jsonDecode(r'''{json}''')).toJson())}}'); }} on FormatException {{ print('{at}: refused'); }}"
        );
    }
    main.push_str("}\n");
    let source = module(
        &[
            signal_dart::dart_definition(),
            open_signal_dart::dart_definition(),
            tagged_signal_dart::dart_definition(),
            open_tagged_signal_dart::dart_definition(),
            adjacent_signal_dart::dart_definition(),
            open_adjacent_signal_dart::dart_definition(),
            untagged_signal_dart::dart_definition(),
            open_untagged_signal_dart::dart_definition(),
            hidden_signal_dart::dart_definition(),
            open_hidden_signal_dart::dart_definition(),
            hidden_tagged_signal_dart::dart_definition(),
            open_hidden_tagged_signal_dart::dart_definition(),
            hidden_adjacent_signal_dart::dart_definition(),
            open_hidden_adjacent_signal_dart::dart_definition(),
            hidden_untagged_signal_dart::dart_definition(),
            open_hidden_untagged_signal_dart::dart_definition(),
        ],
        &[],
        &main,
    );
    let Some((ran, printed)) = dart("empty-variants", &[], &source) else {
        return;
    };
    assert!(ran, "{printed}");
    assert_eq!(objects(&printed), expected());
}

#[cfg(feature = "kotlin")]
#[test]
fn the_kotlin_read_reads_each_enum_as_serde_reads_it() {
    let mut main = String::from("fun main() {\n");
    for (at, (name, json)) in rows().into_iter().enumerate() {
        let _ = writeln!(
            main,
            "    try {{ println(\"{at}: \" + Json.encodeToString({name}.serializer(), \
             Json.decodeFromString({name}.serializer(), \"\"\"{json}\"\"\"))) }} catch (_: \
             IllegalArgumentException) {{ println(\"{at}: refused\") }}"
        );
    }
    main.push_str("}\n");
    let source = [
        "import kotlinx.serialization.*\n\
         import kotlinx.serialization.json.*\n\
         import kotlinx.serialization.descriptors.*\n\
         import kotlinx.serialization.encoding.*\n\
         import kotlinx.serialization.builtins.*"
            .to_owned(),
        signal_kotlin::kotlin_definition(),
        open_signal_kotlin::kotlin_definition(),
        tagged_signal_kotlin::kotlin_definition(),
        open_tagged_signal_kotlin::kotlin_definition(),
        adjacent_signal_kotlin::kotlin_definition(),
        open_adjacent_signal_kotlin::kotlin_definition(),
        untagged_signal_kotlin::kotlin_definition(),
        open_untagged_signal_kotlin::kotlin_definition(),
        hidden_signal_kotlin::kotlin_definition(),
        open_hidden_signal_kotlin::kotlin_definition(),
        hidden_tagged_signal_kotlin::kotlin_definition(),
        open_hidden_tagged_signal_kotlin::kotlin_definition(),
        hidden_adjacent_signal_kotlin::kotlin_definition(),
        open_hidden_adjacent_signal_kotlin::kotlin_definition(),
        hidden_untagged_signal_kotlin::kotlin_definition(),
        open_hidden_untagged_signal_kotlin::kotlin_definition(),
        main,
    ];
    let Some(printed) = ran_kotlin(&source.join("\n\n")) else {
        return;
    };
    assert_eq!(objects(&printed), expected());
}

#[cfg(feature = "swift")]
#[test]
fn the_swift_read_reads_each_enum_as_serde_reads_it() {
    let mut main = String::from(
        "func told<Read: Codable>(_ at: Int, _ type: Read.Type, _ payload: String) {\n  do {\n    \
         let read = try JSONDecoder().decode(type, from: Data(payload.utf8))\n    \
         print(\"\\(at): \\(String(data: try JSONEncoder().encode(read), encoding: .utf8)!)\")\n  \
         } catch {\n    print(\"\\(at): refused\")\n  }\n}\n",
    );
    for (at, (name, json)) in rows().into_iter().enumerate() {
        let _ = writeln!(main, "told({at}, {name}.self, #\"{json}\"#)");
    }
    let source = [
        "import Foundation".to_owned(),
        signal_swift::swift_definition(),
        open_signal_swift::swift_definition(),
        tagged_signal_swift::swift_definition(),
        open_tagged_signal_swift::swift_definition(),
        adjacent_signal_swift::swift_definition(),
        open_adjacent_signal_swift::swift_definition(),
        untagged_signal_swift::swift_definition(),
        open_untagged_signal_swift::swift_definition(),
        hidden_signal_swift::swift_definition(),
        open_hidden_signal_swift::swift_definition(),
        hidden_tagged_signal_swift::swift_definition(),
        open_hidden_tagged_signal_swift::swift_definition(),
        hidden_adjacent_signal_swift::swift_definition(),
        open_hidden_adjacent_signal_swift::swift_definition(),
        hidden_untagged_signal_swift::swift_definition(),
        open_hidden_untagged_signal_swift::swift_definition(),
        main,
    ]
    .join("\n\n");
    let Some(run) = judged(
        "swift-empty-variants",
        "TIXSCHEMA_SWIFT",
        "swift",
        &["-warnings-as-errors"],
        "main.swift",
        &source,
    ) else {
        return;
    };
    assert!(run.succeeded, "{}\n{}\n{source}", run.stdout, run.stderr);
    assert_eq!(objects(&run.stdout), expected());
}
