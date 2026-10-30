//! A check or a function named after a member of its own type, in the emitted Kotlin types
//! compiled with `-Werror`: each row prints what Rust prints.

#![cfg(feature = "kotlin")]

use core::fmt::Write as _;

use super::member_named_rows::{
    Kind, couple_kotlin, crate_kotlin, expected, freight_kotlin, gauge_kotlin, hold_kotlin,
    ledger_kotlin, parcel_kotlin, plain_kotlin, rating_kotlin, rows, stamp_kotlin,
};
use super::runtime::ran_kotlin;

const KOTLIN_IMPORTS: &str = "import kotlinx.serialization.*\n\
     import kotlinx.serialization.json.*\n\
     import kotlinx.serialization.descriptors.*\n\
     import kotlinx.serialization.encoding.*\n\
     import kotlinx.serialization.builtins.*";

const FUNCTIONS: &str = r#"
fun rated(value: Rating): List<String> =
    if (value == Rating.Rated) listOf("a rating is unrated") else emptyList()

fun unrated(payload: JsonElement): JsonElement =
    if (payload is JsonPrimitive && payload.isString) JsonPrimitive(payload.content.trim()) else payload

fun label(value: Parcel): List<String> =
    if (value.label.isEmpty()) listOf("a parcel is labelled") else emptyList()

fun code(value: String): List<String> =
    if (value == "dup") listOf("a code is not dup") else emptyList()

fun note(payload: JsonElement): JsonElement =
    if (payload is JsonObject && "note" !in payload) JsonObject(payload + ("note" to JsonPrimitive("none"))) else payload

fun seal(payload: JsonElement): JsonElement =
    if (payload is JsonPrimitive && payload.isString) JsonPrimitive(payload.content.trim()) else payload

fun validate(value: Gauge): List<String> =
    if (value.size > 9u) listOf("a gauge is at most 9") else emptyList()

fun value(value: Stamp): List<String> =
    if (value.value.isEmpty()) listOf("a stamp is not empty") else emptyList()

fun tag(value: Crate): List<String> =
    if (value.tag == "x") listOf("a crate is not x") else emptyList()

fun title(value: Freight): List<String> =
    if (value is FreightHeld && value.title.isEmpty()) listOf("a freight is titled") else emptyList()

fun <CodeType> codes(value: Hold<CodeType>): List<String> =
    if (value.codes.isEmpty()) listOf("a hold has a code") else emptyList()

fun spare(value: List<String>): List<String> =
    if (value.size > 1) listOf("a hold has one spare") else emptyList()

fun pair(value: %TUPLE%): List<String> =
    if (value.slot1 < 0) listOf("a pair is not negative") else emptyList()

fun entries(value: Map<UInt, String>): List<String> =
    if (value.isEmpty()) listOf("a ledger has an entry") else emptyList()

fun ruled(value: Plain): List<String> =
    if (value.size == 0u) listOf("a plain is sized") else emptyList()

fun report(name: String, violations: List<String>) {
    if (violations.isEmpty()) {
        println("$name: passes")
    } else {
        println("$name: refused")
        violations.forEach { println("    $it") }
    }
}

fun <T> written(name: String, serializer: KSerializer<T>, payload: String) {
    val held = Json.decodeFromString(serializer, payload)
    println("$name: read ${Json.encodeToString(serializer, held)}")
}
"#;

fn kotlin_string(text: &str) -> String {
    format!(
        "\"{}\"",
        text.replace('\\', "\\\\")
            .replace('"', "\\\"")
            .replace('$', "\\$")
    )
}

fn unjudged(read_as: &str) -> String {
    read_as.split_once('<').map_or_else(
        || format!("{read_as}Unjudged"),
        |(named, argument)| {
            format!(
                "{named}Unjudged(serializer<{}>())",
                argument.strip_suffix('>').unwrap_or(argument)
            )
        },
    )
}

#[test]
fn a_check_named_after_a_member_runs_in_kotlin_as_it_runs_in_rust() {
    let mut main = String::from("fun main() {\n");
    for row in rows() {
        let payload = kotlin_string(&row.payload);
        match row.kind {
            Kind::Validate => writeln!(
                main,
                "    report({}, Json.decodeFromString({}, {payload}).validate())",
                kotlin_string(row.name),
                unjudged(row.read_as)
            ),
            Kind::Read => writeln!(
                main,
                "    written({}, serializer<{}>(), {payload})",
                kotlin_string(row.name),
                row.read_as
            ),
        }
        .unwrap();
    }
    main.push_str("}\n");
    let couple = couple_kotlin::kotlin_definition();
    let tuple = couple
        .split("class ")
        .find_map(|after| after.strip_prefix("KotlinTuple"))
        .and_then(|after| {
            after
                .split_once('(')
                .map(|(number, _)| format!("KotlinTuple{number}"))
        })
        .unwrap();
    let source = [
        KOTLIN_IMPORTS.to_owned(),
        rating_kotlin::kotlin_definition(),
        parcel_kotlin::kotlin_definition(),
        gauge_kotlin::kotlin_definition(),
        stamp_kotlin::kotlin_definition(),
        crate_kotlin::kotlin_definition(),
        freight_kotlin::kotlin_definition(),
        hold_kotlin::kotlin_definition(),
        couple,
        ledger_kotlin::kotlin_definition(),
        plain_kotlin::kotlin_definition(),
        FUNCTIONS.replace("%TUPLE%", &tuple),
        main,
    ]
    .join("\n\n");
    let Some(printed) = ran_kotlin(&source) else {
        return;
    };
    assert_eq!(printed, expected(&rows()));
}
