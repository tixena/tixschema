//! The four std ranges read and written by the emitted Kotlin types, compiled with `-Werror`: each
//! payload prints what the serde read prints of it, a range of a model or of a brand judged by
//! `validate()` in the words Rust's `validate()` writes.

#![cfg(feature = "kotlin")]

use core::fmt::Write as _;

use super::range_models::{
    atlas_kotlin, compass_kotlin, course_kotlin, detour_kotlin, excursion_kotlin, frontier_kotlin,
    itinerary_kotlin, milestone_kotlin, positioned_kotlin, signposted_kotlin, spanned_kotlin,
    stride_kotlin, waymark_kotlin, waypost_kotlin,
};
use super::range_rows::{declares_a_parameter, expected, judged, rows};
use super::runtime::ran_kotlin;

const KOTLIN_IMPORTS: &str = "import kotlinx.serialization.*\n\
     import kotlinx.serialization.json.*\n\
     import kotlinx.serialization.descriptors.*\n\
     import kotlinx.serialization.encoding.*\n\
     import kotlinx.serialization.builtins.*";

/// What prints a row: the value read, written back, or `refused`, with each message of what
/// `validate()` refused where the type judges a model or a brand it holds, which the read of a type
/// throws as a `SerializationException` of its own.
const TOLD: &str = r#"
fun <T> row(name: String, serializer: KSerializer<T>, payload: String, judged: Boolean) {
    try {
        val read = Json.decodeFromString(serializer, payload)
        println("$name: read ${Json.encodeToString(serializer, read)}")
    } catch (refused: Exception) {
        println("$name: refused")
        if (judged && refused::class == SerializationException::class) {
            (refused.message ?: "").split("; ").forEach { println("    $it") }
        }
    }
}
"#;

/// `text` as a Kotlin raw string writes it.
fn raw(text: &str) -> String {
    format!("\"\"\"{text}\"\"\"")
}

/// `{"start":8,"end":13}`, `{"start":8}` and `{"end":13}` read into the class of each range and
/// written back as they were, in every position a range is held; a bound left out or of another
/// type refused; and each broken bound reported under its key.
#[test]
fn each_range_reads_and_writes_in_kotlin_as_serde_does() {
    let rows = rows();
    let expected = expected(&rows);
    assert!(
        expected.contains("    'span.end': too short: minimum length is 2, got 1"),
        "{expected}"
    );
    let mut main = String::from("fun main() {\n");
    for row in &rows {
        let filled = if declares_a_parameter(row.read_as) {
            "UInt.serializer()"
        } else {
            ""
        };
        let serializer = format!("{}.serializer({filled})", row.read_as);
        let _ = writeln!(
            main,
            "    row(\"{}\", {serializer}, {}, {})",
            row.name,
            raw(&row.json),
            judged(row.read_as)
        );
    }
    main.push_str("}\n");
    let source = [
        KOTLIN_IMPORTS.to_owned(),
        milestone_kotlin::kotlin_definition(),
        waymark_kotlin::kotlin_definition(),
        itinerary_kotlin::kotlin_definition(),
        spanned_kotlin::kotlin_definition(),
        positioned_kotlin::kotlin_definition(),
        detour_kotlin::kotlin_definition(),
        course_kotlin::kotlin_definition(),
        compass_kotlin::kotlin_definition(),
        atlas_kotlin::kotlin_definition(),
        excursion_kotlin::kotlin_definition(),
        waypost_kotlin::kotlin_definition(),
        stride_kotlin::kotlin_definition(),
        signposted_kotlin::kotlin_definition(),
        frontier_kotlin::kotlin_definition(),
        #[cfg(feature = "chrono")]
        super::range_models::rostered_kotlin::kotlin_definition(),
        TOLD.to_owned(),
        main,
    ]
    .join("\n\n");
    let Some(printed) = ran_kotlin(&source) else {
        return;
    };
    assert_eq!(printed, expected);
}
