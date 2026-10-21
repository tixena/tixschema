//! A set read by the emitted Kotlin types, compiled with `-Werror`: a payload that repeats an item
//! of a set is refused by the type's read, in the words `validate()` answers, under the set's
//! place.

#![cfg(feature = "kotlin")]

use core::fmt::Write as _;

use super::runtime::ran_kotlin;
use super::set_models::{
    PAYLOADS, PRINTED, assert_rust_keeps_one_item, coin_kotlin, declares_a_parameter,
    every_list_kotlin, folder_kotlin, lanyard_kotlin, nesting_kotlin, point_kotlin, pouch_kotlin,
    shelving_kotlin, sorting_kotlin, ticket_kotlin, wallet_kotlin,
};

const KOTLIN_IMPORTS: &str = "import kotlinx.serialization.*\n\
     import kotlinx.serialization.json.*\n\
     import kotlinx.serialization.descriptors.*\n\
     import kotlinx.serialization.encoding.*\n\
     import kotlinx.serialization.builtins.*";

/// What prints a row: `read`, or `refused` with each message of what `validate()` refused, which
/// the read of a type throws as a `SerializationException` of its own.
const TOLD: &str = r#"
fun <T> row(name: String, serializer: KSerializer<T>, payload: String) {
    try {
        Json.decodeFromString(serializer, payload)
        println("$name: read")
    } catch (refused: SerializationException) {
        println("$name: refused")
        (refused.message ?: "").split("; ").forEach { println("    $it") }
    }
}
"#;

/// Each payload that repeats an item of a set is refused under the set's place, where Rust's read
/// keeps one item of it, and `validate()` answers the repeat of a set built in code.
#[test]
fn a_set_that_repeats_an_item_is_refused_by_the_kotlin_read() {
    assert_rust_keeps_one_item();
    let mut main = String::from("fun main() {\n");
    for payload in &PAYLOADS {
        let filled = if declares_a_parameter(payload.read_as) {
            "String.serializer()"
        } else {
            ""
        };
        let _ = writeln!(
            main,
            "    row(\"{}\", {}.serializer({filled}), \"\"\"{}\"\"\")",
            payload.name, payload.read_as, payload.json
        );
    }
    main.push_str("    println(Ticket(listOf(\"a\", \"a\")).validate())\n}\n");
    let source = [
        KOTLIN_IMPORTS.to_owned(),
        point_kotlin::kotlin_definition(),
        ticket_kotlin::kotlin_definition(),
        folder_kotlin::kotlin_definition(),
        every_list_kotlin::kotlin_definition(),
        nesting_kotlin::kotlin_definition(),
        lanyard_kotlin::kotlin_definition(),
        pouch_kotlin::kotlin_definition(),
        shelving_kotlin::kotlin_definition(),
        sorting_kotlin::kotlin_definition(),
        coin_kotlin::kotlin_definition(),
        wallet_kotlin::kotlin_definition(),
        TOLD.to_owned(),
        main,
    ]
    .join("\n\n");
    let Some(printed) = ran_kotlin(&source) else {
        return;
    };
    assert_eq!(
        printed,
        format!("{PRINTED}['tags': repeats an item: a set holds each item once]\n")
    );
}
