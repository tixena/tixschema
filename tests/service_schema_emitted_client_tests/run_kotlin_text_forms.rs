//! A `char` and a chrono date, time and date and time read by the emitted Kotlin types, compiled
//! with `-Werror`: each payload of [`TEXT_FORMS`] is read and written back as it was, or refused
//! with `SerializationException`, where the table states.

#![cfg(all(feature = "kotlin", feature = "chrono"))]

use core::fmt::Write as _;

use super::runtime::ran_kotlin;
use super::text_forms_models::{
    TEXT_FORMS, chronometer_kotlin, day_lines_kotlin, day_log_kotlin, instant_lines_kotlin,
    instant_log_kotlin, letter_lines_kotlin, letter_log_kotlin, stamp_lines_kotlin,
    stamp_log_kotlin, time_lines_kotlin, time_log_kotlin,
};

const KOTLIN_IMPORTS: &str = "import kotlinx.serialization.*\n\
     import kotlinx.serialization.json.*\n\
     import kotlinx.serialization.descriptors.*\n\
     import kotlinx.serialization.encoding.*\n\
     import kotlinx.serialization.builtins.*";

/// What prints a row: what the read writes back of the payload, or the full name of the class of
/// what the read throws, and its message.
const TOLD: &str = r#"
fun <T> told(at: String, serializer: KSerializer<T>, payload: String) {
    val said = try {
        "read " + Json.encodeToString(serializer, Json.decodeFromString(serializer, payload))
    } catch (refusal: Exception) {
        "${refusal::class.qualifiedName}: ${refusal.message}"
    }
    println("$at: $said")
}
"#;

/// Each payload is read and written back as it was, or refused, as [`TEXT_FORMS`] states: what Zod
/// refuses, a `DateTime<Utc>` apart.
#[test]
fn each_text_of_one_form_reads_in_kotlin_where_the_table_states() {
    let mut expected = String::new();
    let mut main = String::from(TOLD);
    main.push_str("fun main() {\n");
    for (at, row) in TEXT_FORMS.iter().enumerate() {
        if row.kotlin == "read" {
            let _ = writeln!(expected, "{at}: read {}", row.json);
        } else {
            let _ = writeln!(
                expected,
                "{at}: kotlinx.serialization.SerializationException: {}",
                row.kotlin
            );
        }
        let _ = writeln!(
            main,
            "    told(\"{at}\", {}.serializer(), \"\"\"{}\"\"\")",
            row.read_as, row.json
        );
    }
    main.push_str("}\n");
    let source = [
        KOTLIN_IMPORTS.to_owned(),
        chronometer_kotlin::kotlin_definition(),
        letter_log_kotlin::kotlin_definition(),
        letter_lines_kotlin::kotlin_definition(),
        day_log_kotlin::kotlin_definition(),
        day_lines_kotlin::kotlin_definition(),
        time_log_kotlin::kotlin_definition(),
        time_lines_kotlin::kotlin_definition(),
        stamp_log_kotlin::kotlin_definition(),
        stamp_lines_kotlin::kotlin_definition(),
        instant_log_kotlin::kotlin_definition(),
        instant_lines_kotlin::kotlin_definition(),
        main,
    ];
    let Some(printed) = ran_kotlin(&source.join("\n\n")) else {
        return;
    };
    assert_eq!(
        printed, expected,
        "the Kotlin read is not what the table states"
    );
}

/// The README's sentences on the read of a `char` and a date show payloads and messages the run
/// above measures.
#[test]
fn the_readme_shows_texts_the_run_measures() {
    let readme = include_str!("../../README.md");
    for (shown, read_as, kotlin) in [
        ("\"ab\"", "LetterLog", "expected a character, found \"ab\""),
        ("\"\"", "LetterLog", "expected a character, found \"\""),
        ("\"\u{1f600}\"", "LetterLog", "read"),
        ("\"x\"", "DayLog", "expected a date, found \"x\""),
        (
            "\"2024-1-6\"",
            "DayLog",
            "expected a date, found \"2024-1-6\"",
        ),
        (
            "\"+12024-01-06\"",
            "DayLog",
            "expected a date, found \"+12024-01-06\"",
        ),
        (
            "\"23:59:60\"",
            "TimeLog",
            "expected a time, found \"23:59:60\"",
        ),
        ("\"2024-01-05T11:20:30Z\"", "StampLog", "read"),
        ("\"2024-01-05T11:20\"", "StampLog", "read"),
    ] {
        assert!(
            TEXT_FORMS.iter().any(|row| row.read_as == read_as
                && row.json.contains(shown)
                && row.kotlin == kotlin),
            "no row of {read_as} holds {shown} where the Kotlin read answers {kotlin}"
        );
        assert!(
            readme.contains(&format!("`{shown}`")),
            "not in the README: {shown}"
        );
    }
    for refusal in [
        "`SerializationException: expected a character, found \"ab\"`",
        "`SerializationException: expected a date, found \"x\"`",
    ] {
        assert!(readme.contains(refusal), "not in the README: {refusal}");
    }
}
