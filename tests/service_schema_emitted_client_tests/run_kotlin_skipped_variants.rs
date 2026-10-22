//! An enum's variant serde neither writes nor reads is written by no emitted Kotlin type: each
//! enum reads the payload of every variant serde writes and writes it back, and refuses the
//! payload of one it skips, as serde does, under every tagging, compiled with `-Werror`.

#![cfg(feature = "kotlin")]

use super::runtime::ran_kotlin;
use super::skipped_variant_models::{
    ROWS, berthage_kotlin, cartage_kotlin, dockage_kotlin, keelage_kotlin, lighterage_kotlin,
    pilotage_kotlin, tonnage_kotlin, unread_writes, wharfage_kotlin,
};

const KOTLIN_IMPORTS: &str = "import kotlinx.serialization.*\n\
     import kotlinx.serialization.json.*\n\
     import kotlinx.serialization.descriptors.*\n\
     import kotlinx.serialization.encoding.*\n\
     import kotlinx.serialization.builtins.*";

/// What reads a payload and answers it written back, or `refused`.
const SAID: &str = r#"
fun <T> said(serializer: KSerializer<T>, json: String): String =
    runCatching { Json.decodeFromString(serializer, json) }
        .fold({ "read " + Json.encodeToString(serializer, it) }, { "refused" })
"#;

/// What prints each row serde and Zod agree on, read as its type and written back.
fn main_of_rows() -> String {
    let lines: Vec<String> = ROWS
        .iter()
        .filter(|row| row.serde == row.zod)
        .map(|row| {
            format!(
                "    println(said({}.serializer(), \"\"\"{}\"\"\"))",
                row.read_as, row.json
            )
        })
        .collect();
    format!("fun main() {{\n{}\n}}\n", lines.join("\n"))
}

/// Each row serde reads is read and written back as it was, and each row serde refuses, of a
/// variant it neither writes nor reads, is refused.
#[test]
fn a_variant_serde_neither_writes_nor_reads_is_refused_in_kotlin() {
    assert!(
        unread_writes()
            .iter()
            .rev()
            .skip(1)
            .all(|said| said.ends_with("cannot be serialized")),
        "serde writes a variant it skips"
    );
    let source = [
        KOTLIN_IMPORTS.to_owned(),
        berthage_kotlin::kotlin_definition(),
        wharfage_kotlin::kotlin_definition(),
        pilotage_kotlin::kotlin_definition(),
        lighterage_kotlin::kotlin_definition(),
        keelage_kotlin::kotlin_definition(),
        cartage_kotlin::kotlin_definition(),
        tonnage_kotlin::kotlin_definition(),
        dockage_kotlin::kotlin_definition(),
        SAID.to_owned(),
        main_of_rows(),
    ];
    let Some(printed) = ran_kotlin(&source.join("\n\n")) else {
        return;
    };
    let rows: Vec<_> = ROWS.iter().filter(|row| row.serde == row.zod).collect();
    let lines: Vec<&str> = printed.lines().collect();
    assert_eq!(lines.len(), rows.len(), "{printed}");
    for (row, line) in rows.iter().zip(lines) {
        let read = (row.read)(row.json);
        assert_eq!(read, row.serde, "serde's read of {}", row.json);
        if read {
            let written: serde_json::Value =
                serde_json::from_str(line.strip_prefix("read ").unwrap_or("null")).unwrap();
            assert_eq!(
                written,
                serde_json::from_str::<serde_json::Value>(row.json).unwrap(),
                "{} as {}: {line}",
                row.json,
                row.read_as
            );
        } else {
            assert_eq!(line, "refused", "{} as {}", row.json, row.read_as);
        }
    }
}
