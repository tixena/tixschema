//! A `literal` read by the emitted Kotlin types compiled with `-Werror`, against serde: each
//! payload of the literal table is read and refused as serde reads and refuses it, each violation
//! in the sentence Rust's `validate()` reports.

#![cfg(all(feature = "kotlin", feature = "serde"))]

use core::fmt::Write as _;

use super::literal_models::{
    Answered, LITERAL_READS, adjacent_assay_kotlin, assay_kotlin, bullion_kotlin, hallmark_kotlin,
    ingot_kotlin, literal_report, names_each_violation, serde_read, tagged_assay_kotlin,
    touchmark_kotlin,
};
use super::runtime::ran_kotlin;

const KOTLIN_IMPORTS: &str = "import kotlinx.serialization.*\n\
     import kotlinx.serialization.json.*\n\
     import kotlinx.serialization.descriptors.*\n\
     import kotlinx.serialization.encoding.*\n\
     import kotlinx.serialization.builtins.*";

/// What prints a row: its name and whether it is read, then each violation the read refused it
/// with, where `named` asks for them.
const TOLD: &str = r#"
fun <T> told(name: String, named: Boolean, serializer: KSerializer<T>, payload: String) {
    try {
        Json.decodeFromString(serializer, payload)
        println("$name: read")
    } catch (refused: SerializationException) {
        println("$name: refused")
        if (named) {
            refused.message.orEmpty().split("; ").forEach { println("    $it") }
        }
    }
}
"#;

/// Each payload is read and refused by the Kotlin read as serde reads and refuses it, under
/// `-Werror`.
#[test]
fn a_literal_refuses_in_kotlin_what_serde_refuses() {
    let mut main = String::from("fun main() {\n");
    for row in &LITERAL_READS {
        assert_eq!(
            serde_read(row.held_by, row.payload).is_ok(),
            matches!(row.answered, Answered::Read),
            "serde no longer reads {} as the table says",
            row.name
        );
        let _ = writeln!(
            main,
            "    told(\"{}\", {}, serializer<{}>(), \"\"\"{}\"\"\")",
            row.name,
            names_each_violation(row),
            row.held_by,
            row.payload
        );
    }
    main.push_str("}\n");
    let source = [
        KOTLIN_IMPORTS.to_owned(),
        adjacent_assay_kotlin::kotlin_definition(),
        assay_kotlin::kotlin_definition(),
        bullion_kotlin::kotlin_definition(),
        hallmark_kotlin::kotlin_definition(),
        ingot_kotlin::kotlin_definition(),
        tagged_assay_kotlin::kotlin_definition(),
        touchmark_kotlin::kotlin_definition(),
        TOLD.to_owned(),
        main,
    ];
    let Some(printed) = ran_kotlin(&source.join("\n\n")) else {
        return;
    };
    assert_eq!(
        printed,
        literal_report(names_each_violation),
        "the Kotlin read is not serde's"
    );
}
