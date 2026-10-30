//! A set read and written by the emitted Kotlin types at one list level, as serde writes it,
//! compiled with `-Werror`.

#![cfg(feature = "kotlin")]

use core::fmt::Write as _;

use super::runtime::ran_kotlin;
#[cfg(feature = "indexmap")]
use super::set_level_models::indexed_kotlin;
use super::set_level_models::{
    BROKEN, BROKEN_MESSAGE, ROUND_TRIPS, assert_rust_writes_back, entry_kotlin, groups_kotlin,
    probe_sets_kotlin, shelf_kotlin, wrappers_kotlin,
};

const KOTLIN_IMPORTS: &str = "import kotlinx.serialization.*\n\
     import kotlinx.serialization.json.*\n\
     import kotlinx.serialization.descriptors.*\n\
     import kotlinx.serialization.encoding.*\n\
     import kotlinx.serialization.builtins.*";

const TOLD: &str = r#"
fun <T> row(name: String, serializer: KSerializer<T>, payload: String) {
    try {
        println("$name: ${Json.encodeToString(serializer, Json.decodeFromString(serializer, payload))}")
    } catch (refused: SerializationException) {
        println("$name: refused")
        (refused.message ?: "").split("; ").forEach { println("    $it") }
    }
}
"#;

#[test]
fn a_set_is_one_list_level_in_the_kotlin_types() {
    assert_rust_writes_back();
    let mut main = String::from("fun main() {\n");
    for &(name, json) in ROUND_TRIPS {
        let _ = writeln!(
            main,
            "    row(\"{name}\", {name}.serializer(), \"\"\"{json}\"\"\")"
        );
    }
    let (broken_name, broken_json) = BROKEN;
    let _ = writeln!(
        main,
        "    row(\"broken\", {broken_name}.serializer(), \"\"\"{broken_json}\"\"\")"
    );
    main.push_str(
        "    println(Shelf(listOf(Entry(\"a\")), mapOf(), null, listOf(), KotlinTuple211(listOf(), 1.toUByte())).validate())\n}\n",
    );
    let mut parts = vec![
        KOTLIN_IMPORTS.to_owned(),
        entry_kotlin::kotlin_definition(),
        groups_kotlin::kotlin_definition(),
        probe_sets_kotlin::kotlin_definition(),
        shelf_kotlin::kotlin_definition(),
        wrappers_kotlin::kotlin_definition(),
    ];
    #[cfg(feature = "indexmap")]
    parts.push(indexed_kotlin::kotlin_definition());
    parts.extend([TOLD.to_owned(), main]);
    let source = parts.join("\n\n");
    let Some(printed) = ran_kotlin(&source) else {
        return;
    };
    let mut expected = String::new();
    for &(name, json) in ROUND_TRIPS {
        let _ = writeln!(expected, "{name}: {json}");
    }
    let _ = write!(
        expected,
        "broken: refused\n    {BROKEN_MESSAGE}\n[{BROKEN_MESSAGE}]\n"
    );
    assert_eq!(printed, expected, "{source}");
}
