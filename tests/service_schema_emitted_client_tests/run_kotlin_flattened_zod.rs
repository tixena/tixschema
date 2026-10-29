//! A flattened choice, the choice it holds and an internally tagged struct variant, read by the
//! emitted Kotlin types compiled with `-Werror`: each payload of the shared rows is read as serde
//! reads it, and each value of the checked rows is held to the messages Rust's `validate()`
//! answers.

#![cfg(feature = "kotlin")]

use core::fmt::Write as _;

use super::flattened_zod_models::{
    CHECKED_REPORT, ROWS, berthage_kotlin, block_kotlin, booking_kotlin, cargo_kotlin,
    checked_cases, checked_report, deck_kotlin, gap_kotlin, hatch_kotlin, hold_kotlin, mole_kotlin,
    moorage_kotlin, paragraph_kotlin, pause_kotlin, quote_kotlin, read_report, slip_kotlin,
    span_kotlin, tally_kotlin, tenancy_kotlin, tethered_kotlin, wharf_kotlin, wharfage_kotlin,
};
use super::runtime::ran_kotlin;

/// The imports the emitted Kotlin and its driver reach for.
const KOTLIN_IMPORTS: &str = "import kotlinx.serialization.*\n\
     import kotlinx.serialization.json.*\n\
     import kotlinx.serialization.descriptors.*\n\
     import kotlinx.serialization.encoding.*\n\
     import kotlinx.serialization.builtins.*";

/// The functions the models name, as a consumer writes them in Kotlin.
const FUNCTIONS: &str = r#"
fun asGiven(payload: JsonElement): JsonElement = payload

fun aSpan(value: Span): List<String> = when (value) {
    is SpanHours -> if (value.hours == 0u) listOf("'hours': a span is not empty") else emptyList()
    is SpanDays -> if (value.days == "9") listOf("'days': a span is under nine days") else emptyList()
}

fun notBlank(value: String): List<String> =
    if (value.isEmpty()) listOf("a day label is not blank") else emptyList()
"#;

/// What prints a row: the value read and written back with its keys sorted, or `refused`, and what
/// prints a checked row.
const HARNESS: &str = r#"
val everyMember = Json { encodeDefaults = true }

fun sorted(value: JsonElement): JsonElement = when (value) {
    is JsonObject -> JsonObject(value.entries.sortedBy { it.key }.associate { it.key to sorted(it.value) })
    is JsonArray -> JsonArray(value.map { sorted(it) })
    else -> value
}

fun <T> told(at: String, serializer: KSerializer<T>, payload: String) {
    val printed = try {
        "read ${sorted(everyMember.encodeToJsonElement(serializer, Json.decodeFromString(serializer, payload)))}"
    } catch (refused: SerializationException) {
        "refused"
    }
    println("$at: $printed")
}

fun <T> checked(at: String, serializer: KSerializer<T>, payload: String) {
    try {
        Json.decodeFromString(serializer, payload)
        println("$at: passes")
    } catch (refused: SerializationException) {
        println("$at: refused")
        for (violation in (refused.message ?: "").split("; ")) println("    $violation")
    }
}
"#;

fn definitions() -> Vec<String> {
    vec![
        moorage_kotlin::kotlin_definition(),
        berthage_kotlin::kotlin_definition(),
        tethered_kotlin::kotlin_definition(),
        mole_kotlin::kotlin_definition(),
        wharf_kotlin::kotlin_definition(),
        slip_kotlin::kotlin_definition(),
        wharfage_kotlin::kotlin_definition(),
        hatch_kotlin::kotlin_definition(),
        deck_kotlin::kotlin_definition(),
        cargo_kotlin::kotlin_definition(),
        hold_kotlin::kotlin_definition(),
        tally_kotlin::kotlin_definition(),
        gap_kotlin::kotlin_definition(),
        pause_kotlin::kotlin_definition(),
        span_kotlin::kotlin_definition(),
        booking_kotlin::kotlin_definition(),
        tenancy_kotlin::kotlin_definition(),
        block_kotlin::kotlin_definition(),
        paragraph_kotlin::kotlin_definition(),
        quote_kotlin::kotlin_definition(),
    ]
}

fn source(main: &str) -> String {
    let mut parts = vec![KOTLIN_IMPORTS.to_owned()];
    parts.extend(definitions());
    parts.extend([FUNCTIONS.to_owned(), HARNESS.to_owned(), main.to_owned()]);
    parts.join("\n\n")
}

/// Each payload of the rows is read by Kotlin as serde reads it, and refused where it holds a key
/// no member reads, as Zod refuses it.
#[test]
fn a_flattened_choice_is_read_by_kotlin_as_serde_reads_it() {
    let mut main = String::from("fun main() {\n");
    for row in ROWS {
        let _ = writeln!(
            main,
            "    told(\"{}\", serializer<{}>(), \"\"\"{}\"\"\")",
            row.name, row.schema, row.payload
        );
    }
    main.push_str("}\n");
    let Some(printed) = ran_kotlin(&source(&main)) else {
        return;
    };
    assert_eq!(printed, read_report(), "the Kotlin read is not serde's");
}

/// Each value of the checked rows is refused by Kotlin with the messages `validate()` answers.
#[test]
fn a_check_of_a_flattened_choice_answers_in_kotlin_what_validate_answers() {
    let cases = checked_cases();
    assert_eq!(checked_report(&cases), CHECKED_REPORT);
    let mut main = String::from("fun main() {\n");
    for case in &cases {
        let _ = writeln!(
            main,
            "    checked(\"{}\", serializer<{}>(), \"\"\"{}\"\"\")",
            case.name, case.schema, case.payload
        );
    }
    main.push_str("}\n");
    let Some(printed) = ran_kotlin(&source(&main)) else {
        return;
    };
    assert_eq!(printed, CHECKED_REPORT, "the Kotlin validate is not Rust's");
}
