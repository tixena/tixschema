//! Each type of [`empty_struct_models`](super::empty_struct_models) read by Zod under node and by
//! the emitted Dart, Kotlin and Swift types: `{}` reads and is written back as `{}`, and a key
//! beside it is refused by a strict type and read past by a `nonstrict` one, as serde does.

use core::fmt::Write as _;

#[cfg(feature = "dart")]
use super::belfry_service::ring_bell_request_dart;
#[cfg(feature = "kotlin")]
use super::belfry_service::ring_bell_request_kotlin;
#[cfg(feature = "swift")]
use super::belfry_service::ring_bell_request_swift;
use super::empty_struct_models::{expected, rows, ts_definitions};
#[cfg(feature = "dart")]
use super::empty_struct_models::{knell_dart, open_knell_dart, open_unheld_dart, unheld_dart};
#[cfg(feature = "kotlin")]
use super::empty_struct_models::{
    knell_kotlin, open_knell_kotlin, open_unheld_kotlin, unheld_kotlin,
};
#[cfg(feature = "swift")]
use super::empty_struct_models::{knell_swift, open_knell_swift, open_unheld_swift, unheld_swift};
#[cfg(feature = "dart")]
use super::run_dart_validate::{dart, module};
#[cfg(feature = "swift")]
use super::runtime::judged;
#[cfg(feature = "kotlin")]
use super::runtime::ran_kotlin;
use super::runtime::{node_modules, ran_with_modules, stand_down_modules};

const REQUIRED_PACKAGES: &[&str] = &["zod"];

/// Each line of `printed` after its position.
fn told(printed: &str) -> Vec<String> {
    printed
        .lines()
        .enumerate()
        .map(|(at, line)| {
            let (position, written) = line.split_once(": ").unwrap();
            assert_eq!(position, at.to_string(), "{printed}");
            written.to_owned()
        })
        .collect()
}

#[test]
fn zod_reads_each_empty_struct_as_serde_reads_it() {
    let Some(modules) = node_modules(REQUIRED_PACKAGES) else {
        stand_down_modules(REQUIRED_PACKAGES, "Zod's read of the empty structs");
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
        "empty-structs",
        "TIXSCHEMA_NODE",
        "node",
        "empty-structs.mts",
        &source,
        &modules,
    ) else {
        return;
    };
    assert_eq!(told(&printed), expected());
}

#[cfg(feature = "dart")]
#[test]
fn the_dart_read_reads_each_empty_struct_as_serde_reads_it() {
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
            ring_bell_request_dart::dart_definition(),
            knell_dart::dart_definition(),
            open_knell_dart::dart_definition(),
            unheld_dart::dart_definition(),
            open_unheld_dart::dart_definition(),
        ],
        &[],
        &main,
    );
    let Some((ran, printed)) = dart("empty-structs", &[], &source) else {
        return;
    };
    assert!(ran, "{printed}");
    assert_eq!(told(&printed), expected());
}

#[cfg(feature = "kotlin")]
#[test]
fn the_kotlin_read_reads_each_empty_struct_as_serde_reads_it() {
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
        ring_bell_request_kotlin::kotlin_definition(),
        knell_kotlin::kotlin_definition(),
        open_knell_kotlin::kotlin_definition(),
        unheld_kotlin::kotlin_definition(),
        open_unheld_kotlin::kotlin_definition(),
        main,
    ];
    let Some(printed) = ran_kotlin(&source.join("\n\n")) else {
        return;
    };
    assert_eq!(told(&printed), expected());
}

#[cfg(feature = "swift")]
#[test]
fn the_swift_read_reads_each_empty_struct_as_serde_reads_it() {
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
        ring_bell_request_swift::swift_definition(),
        knell_swift::swift_definition(),
        open_knell_swift::swift_definition(),
        unheld_swift::swift_definition(),
        open_unheld_swift::swift_definition(),
        main,
    ]
    .join("\n\n");
    let Some(run) = judged(
        "swift-empty-structs",
        "TIXSCHEMA_SWIFT",
        "swift",
        &["-warnings-as-errors"],
        "main.swift",
        &source,
    ) else {
        return;
    };
    assert!(run.succeeded, "{}\n{}\n{source}", run.stdout, run.stderr);
    assert_eq!(told(&run.stdout), expected());
}
