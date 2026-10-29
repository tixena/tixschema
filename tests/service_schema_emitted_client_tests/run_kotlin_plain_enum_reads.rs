//! A plain enum that names a check, read on its own and held by another type, by the emitted
//! Kotlin types compiled with `-Werror`: each payload of
//! [`plain_enum_check_rows`](super::plain_enum_check_rows) prints what the serde read prints.

#![cfg(feature = "kotlin")]

use core::fmt::Write as _;

use super::plain_enum_check_rows::{
    appeal_kotlin, bench_kotlin, expected, hearing_kotlin, rows, ruling_kotlin, verdict_kotlin,
};
use super::runtime::ran_kotlin;

const KOTLIN_IMPORTS: &str = "import kotlinx.serialization.*\n\
     import kotlinx.serialization.json.*\n\
     import kotlinx.serialization.descriptors.*\n\
     import kotlinx.serialization.encoding.*\n\
     import kotlinx.serialization.builtins.*";

const UPHELD: &str = r#"
fun tidied(payload: JsonElement): JsonElement =
    if (payload is JsonPrimitive && payload.isString) JsonPrimitive(payload.content.trim()) else payload

fun allowed(value: Ruling): List<String> =
    if (value == Ruling.Denied) listOf("a ruling is allowed") else emptyList()

fun affirmed(value: Verdict): List<String> =
    if (value == Verdict.Overturned) listOf("a verdict is upheld") else emptyList()
"#;

const HARNESS: &str = r#"
fun <T> row(name: String, serializer: KSerializer<T>, payload: String, detail: Boolean) {
    try {
        val read = Json.decodeFromString(serializer, payload)
        println("$name: read ${Json.encodeToString(serializer, read)}")
    } catch (refused: SerializationException) {
        println("$name: refused")
        if (detail) (refused.message ?: "").split("; ").forEach { println("    $it") }
    }
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

#[test]
fn a_plain_enum_with_a_check_reads_in_kotlin_as_serde_reads_it() {
    let rows = rows();
    let mut main = String::from("fun main() {\n");
    for row in &rows {
        let _ = writeln!(
            main,
            "    row({}, serializer<{}>(), {}, {})",
            kotlin_string(row.name),
            row.read_as,
            kotlin_string(row.payload),
            row.detail
        );
    }
    main.push_str("}\n");
    let source = [
        KOTLIN_IMPORTS.to_owned(),
        verdict_kotlin::kotlin_definition(),
        bench_kotlin::kotlin_definition(),
        appeal_kotlin::kotlin_definition(),
        hearing_kotlin::kotlin_definition(),
        ruling_kotlin::kotlin_definition(),
        UPHELD.to_owned(),
        HARNESS.to_owned(),
        main,
    ]
    .join("\n\n");
    let Some(printed) = ran_kotlin(&source) else {
        return;
    };
    assert_eq!(printed, expected(&rows));
}
