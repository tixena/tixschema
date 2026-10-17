//! The std `Result` read and written by the emitted Kotlin types, compiled with `-Werror`: each
//! payload prints what the serde read prints of it, a `Result` of a model or of a brand judged by
//! `validate()` in the words Rust's `validate()` writes.

#![cfg(feature = "kotlin")]

use core::fmt::Write as _;

use super::result_models::{
    attempted_kotlin, ballot_kotlin, bureau_kotlin, errand_kotlin, errata_kotlin, jacket_kotlin,
    mailbag_kotlin, masthead_kotlin, missive_kotlin, notebook_kotlin, postbox_kotlin, reply_kotlin,
    ruling_kotlin, settlement_kotlin, summary_kotlin, turn_kotlin, unparsed_kotlin,
};
use super::result_rows::{declares_a_parameter, expected, judged, rows};
use super::runtime::ran_kotlin;

const KOTLIN_IMPORTS: &str = "import kotlinx.serialization.*\n\
     import kotlinx.serialization.json.*\n\
     import kotlinx.serialization.descriptors.*\n\
     import kotlinx.serialization.encoding.*\n\
     import kotlinx.serialization.builtins.*";

/// What prints a row: the value read, written back, or `refused`, with each message of what
/// `validate()` refused where the type judges a model it holds, which the read of a type throws
/// as a `SerializationException` of its own.
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

/// `{"Ok": T}` and `{"Err": E}` read into the sealed interface of each `Result` and written back
/// as they were, in every position a `Result` is held, `null` under an arm of `()`; an object of
/// both keys, of neither and of another key refused; and each broken bound in an arm reported
/// under the arm's key.
#[test]
fn each_result_reads_and_writes_in_kotlin_as_serde_does() {
    let rows = rows();
    let expected = expected(&rows);
    assert!(
        expected.contains("    'frontmatter.Ok.slug': too long: maximum length is 8, got 12"),
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
        masthead_kotlin::kotlin_definition(),
        unparsed_kotlin::kotlin_definition(),
        ballot_kotlin::kotlin_definition(),
        mailbag_kotlin::kotlin_definition(),
        reply_kotlin::kotlin_definition(),
        missive_kotlin::kotlin_definition(),
        postbox_kotlin::kotlin_definition(),
        ruling_kotlin::kotlin_definition(),
        settlement_kotlin::kotlin_definition(),
        errand_kotlin::kotlin_definition(),
        attempted_kotlin::kotlin_definition(),
        turn_kotlin::kotlin_definition(),
        summary_kotlin::kotlin_definition(),
        bureau_kotlin::kotlin_definition(),
        jacket_kotlin::kotlin_definition(),
        notebook_kotlin::kotlin_definition(),
        errata_kotlin::kotlin_definition(),
        TOLD.to_owned(),
        main,
    ]
    .join("\n\n");
    let Some(printed) = ran_kotlin(&source) else {
        return;
    };
    assert_eq!(printed, expected);
}
