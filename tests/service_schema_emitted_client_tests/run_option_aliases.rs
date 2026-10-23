//! An alias of an `Option` held outside a field, read and written back by the emitted Dart, Swift
//! and Kotlin types under their toolchains as serde writes it: `None` as `null` in an item of a
//! list, a value of a map, a position of a tuple, a type argument, an arm of a `Result`, a slot of
//! a tuple struct and the slot of a variant. Beside it, a field of the alias, its key left out for
//! `None` or, under `nullable`, written as `null`.

#![cfg(all(
    feature = "serde",
    any(feature = "dart", feature = "swift", feature = "kotlin")
))]

use core::fmt::Write as _;

use serde_json::Value;

use super::option_alias_models::{LEADS, serde_wrote};
#[cfg(feature = "dart")]
use super::option_alias_models::{
    chanced_dart, deepsea_dart, handlead_dart, haply_dart, heaved_dart, leadsman_dart,
    lineman_dart, plumbline_dart, plummet_dart, reckoned_dart, relead_dart, shoaled_dart,
    sinker_dart, swung_dart,
};
#[cfg(feature = "kotlin")]
use super::option_alias_models::{
    chanced_kotlin, deepsea_kotlin, handlead_kotlin, haply_kotlin, heaved_kotlin, leadsman_kotlin,
    lineman_kotlin, plumbline_kotlin, plummet_kotlin, reckoned_kotlin, relead_kotlin,
    shoaled_kotlin, sinker_kotlin, swung_kotlin,
};
#[cfg(feature = "swift")]
use super::option_alias_models::{
    chanced_swift, deepsea_swift, handlead_swift, haply_swift, heaved_swift, leadsman_swift,
    lineman_swift, plumbline_swift, plummet_swift, reckoned_swift, relead_swift, shoaled_swift,
    sinker_swift, swung_swift,
};
#[cfg(feature = "dart")]
use super::run_dart_validate::{dart, module};
#[cfg(feature = "swift")]
use super::runtime::judged;
#[cfg(feature = "kotlin")]
use super::runtime::ran_kotlin;

/// Prints a row: the value written back, or `refused`.
#[cfg(feature = "dart")]
const DART_WRITTEN: &str = "
void written(Object Function() read) {
  try {
    print(jsonEncode((read() as dynamic).toJson()));
  } catch (_) {
    print('refused');
  }
}
";

/// Prints a row: the value written back, or `refused`.
#[cfg(feature = "swift")]
const SWIFT_WRITTEN: &str = r#"
func written<Read: Codable>(_ type: Read.Type, _ payload: String) {
  do {
    let read = try JSONDecoder().decode(type, from: Data(payload.utf8))
    print(String(data: try JSONEncoder().encode(read), encoding: .utf8)!)
  } catch {
    print("refused")
  }
}
"#;

/// Prints a row: the value written back, or `refused`.
#[cfg(feature = "kotlin")]
const KOTLIN_WRITTEN: &str = r#"
fun <T : Any> written(serializer: KSerializer<T>, payload: String) {
    val written = try {
        Json.encodeToString(serializer, Json.decodeFromString(serializer, payload))
    } catch (refusal: Exception) {
        if (refusal is SerializationException || refusal is IllegalArgumentException) "refused"
        else throw refusal
    }
    println(written)
}
"#;

/// The imports the emitted Kotlin and its driver reach for.
#[cfg(feature = "kotlin")]
const KOTLIN_IMPORTS: &str = "import kotlinx.serialization.*\n\
     import kotlinx.serialization.json.*\n\
     import kotlinx.serialization.descriptors.*\n\
     import kotlinx.serialization.encoding.*\n\
     import kotlinx.serialization.builtins.*";

/// The rows of [`LEADS`] but those read as one of `left_out`.
#[cfg(any(feature = "dart", feature = "kotlin"))]
fn rows_but(left_out: &[&str]) -> Vec<(&'static str, &'static str)> {
    LEADS
        .into_iter()
        .filter(|(read_as, _)| !left_out.contains(read_as))
        .collect()
}

/// Holds what a client printed, one line per row, to what serde writes back of each row.
fn held_to_serde(rows: &[(&str, &str)], printed: &str, client: &str) {
    let written: Vec<Option<Value>> = printed
        .lines()
        .map(|line| serde_json::from_str(line).ok())
        .collect();
    let expected: Vec<Option<Value>> = rows
        .iter()
        .map(|&(read_as, json)| {
            assert_eq!(
                serde_wrote(read_as, json).as_deref(),
                Some(json),
                "serde does not write back {json} read as {read_as}"
            );
            serde_json::from_str(json).ok()
        })
        .collect();
    assert_eq!(
        written, expected,
        "the {client} read does not write back what serde writes:\n{printed}"
    );
}

/// The Dart read takes each payload serde writes and writes it back as serde does.
#[cfg(feature = "dart")]
#[test]
fn dart_writes_back_the_null_serde_writes_for_an_alias_of_an_option() {
    // Dart reads a struct of one slot as an array of the slot, where serde writes the slot's value.
    let rows = rows_but(&["Deepsea"]);
    let mut main = String::from("void main() {\n");
    for &(read_as, json) in &rows {
        let _ = writeln!(
            main,
            "  written(() => {read_as}.fromJson(jsonDecode(r'''{json}''')));"
        );
    }
    main.push_str("}\n");
    let source = module(
        &[
            plummet_dart::dart_definition(),
            relead_dart::dart_definition(),
            haply_dart::dart_definition(),
            swung_dart::dart_definition(),
            leadsman_dart::dart_definition(),
            chanced_dart::dart_definition(),
            handlead_dart::dart_definition(),
            reckoned_dart::dart_definition(),
            deepsea_dart::dart_definition(),
            sinker_dart::dart_definition(),
            heaved_dart::dart_definition(),
            shoaled_dart::dart_definition(),
            lineman_dart::dart_definition(),
            plumbline_dart::dart_definition(),
        ],
        &[DART_WRITTEN],
        &main,
    );
    let Some((ran, printed)) = dart("option-aliases", &[], &source) else {
        return;
    };
    assert!(ran, "{printed}");
    held_to_serde(&rows, &printed, "Dart");
}

/// The Swift read takes each payload serde writes and writes it back as serde does.
#[cfg(feature = "swift")]
#[test]
fn swift_writes_back_the_null_serde_writes_for_an_alias_of_an_option() {
    let mut main = String::new();
    for (read_as, json) in LEADS {
        let _ = writeln!(main, "written({read_as}.self, #\"{json}\"#)");
    }
    let source = [
        "import Foundation".to_owned(),
        plummet_swift::swift_definition(),
        relead_swift::swift_definition(),
        haply_swift::swift_definition(),
        swung_swift::swift_definition(),
        leadsman_swift::swift_definition(),
        chanced_swift::swift_definition(),
        handlead_swift::swift_definition(),
        reckoned_swift::swift_definition(),
        deepsea_swift::swift_definition(),
        sinker_swift::swift_definition(),
        heaved_swift::swift_definition(),
        shoaled_swift::swift_definition(),
        lineman_swift::swift_definition(),
        plumbline_swift::swift_definition(),
        SWIFT_WRITTEN.to_owned(),
        main,
    ]
    .join("\n\n");
    let Some(run) = judged(
        "swift-option-aliases",
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
    held_to_serde(&LEADS, &run.stdout, "Swift");
}

/// The Kotlin read takes each payload serde writes and writes it back as serde does.
#[cfg(feature = "kotlin")]
#[test]
fn kotlin_writes_back_the_null_serde_writes_for_an_alias_of_an_option() {
    // Kotlin reads a struct of one slot as an array of the slot, where serde writes the slot's
    // value.
    let rows = rows_but(&["Deepsea"]);
    let mut main = String::from("fun main() {\n");
    for &(read_as, json) in &rows {
        let _ = writeln!(
            main,
            "    written(serializer<{read_as}>(), \"\"\"{json}\"\"\")"
        );
    }
    main.push_str("}\n");
    let source = [
        KOTLIN_IMPORTS.to_owned(),
        plummet_kotlin::kotlin_definition(),
        relead_kotlin::kotlin_definition(),
        haply_kotlin::kotlin_definition(),
        swung_kotlin::kotlin_definition(),
        leadsman_kotlin::kotlin_definition(),
        chanced_kotlin::kotlin_definition(),
        handlead_kotlin::kotlin_definition(),
        reckoned_kotlin::kotlin_definition(),
        deepsea_kotlin::kotlin_definition(),
        sinker_kotlin::kotlin_definition(),
        heaved_kotlin::kotlin_definition(),
        shoaled_kotlin::kotlin_definition(),
        lineman_kotlin::kotlin_definition(),
        plumbline_kotlin::kotlin_definition(),
        KOTLIN_WRITTEN.to_owned(),
        main,
    ];
    let Some(printed) = ran_kotlin(&source.join("\n\n")) else {
        return;
    };
    held_to_serde(&rows, &printed, "Kotlin");
}
