//! An enum with a variant of several slots, read and written by the emitted Kotlin types compiled
//! with `-Werror` under each tagging serde accepts for one, and the untagged enum that writes the
//! same slots from a tuple: each payload is written back as the serde read writes it back.

#![cfg(feature = "kotlin")]

use core::fmt::Write as _;

use serde::Serialize;
use serde::de::DeserializeOwned;

use super::runtime::ran_kotlin;
use super::tuple_variants_models::{
    Piling, Racking, Stacking, piling_kotlin, racking_kotlin, stacking_kotlin,
};

const KOTLIN_IMPORTS: &str = "import kotlinx.serialization.*\n\
     import kotlinx.serialization.json.*\n\
     import kotlinx.serialization.descriptors.*\n\
     import kotlinx.serialization.encoding.*\n\
     import kotlinx.serialization.builtins.*";

const ROWS: [Row; 6] = [
    Row {
        json: r#"{"kind":"Split","held":[[1,2],3]}"#,
        name: "slots under a tag and a content key",
        read_as: "Racking",
        written: written_back::<Racking>,
    },
    Row {
        json: r#"{"kind":"Loose","held":{"members":["a"]}}"#,
        name: "members under a tag and a content key",
        read_as: "Racking",
        written: written_back::<Racking>,
    },
    Row {
        json: r#"{"Split":[[1,2],3]}"#,
        name: "slots under the variant's name",
        read_as: "Stacking",
        written: written_back::<Stacking>,
    },
    Row {
        json: r#"{"Loose":{"members":["a"]}}"#,
        name: "members under the variant's name",
        read_as: "Stacking",
        written: written_back::<Stacking>,
    },
    Row {
        json: "[[1,2],3]",
        name: "slots untagged",
        read_as: "Piling",
        written: written_back::<Piling>,
    },
    Row {
        json: r#"{"members":["a"]}"#,
        name: "members untagged",
        read_as: "Piling",
        written: written_back::<Piling>,
    },
];

/// What prints a row: the value read, written back.
const TOLD: &str = r#"
fun <T> row(name: String, serializer: KSerializer<T>, payload: String) {
    val read = Json.decodeFromString(serializer, payload)
    println("$name: ${Json.encodeToString(serializer, read)}")
}
"#;

/// A payload, the type it is read as, and what the serde read of it writes back.
struct Row {
    json: &'static str,
    name: &'static str,
    read_as: &'static str,
    written: fn(&str) -> String,
}

/// What serde writes back of `json`, read as `T`.
fn written_back<T>(json: &str) -> String
where
    T: DeserializeOwned + Serialize,
{
    let read: T = serde_json::from_str(json).unwrap();
    serde_json::to_string(&read).unwrap()
}

/// Each variant of several slots, and the variant beside it, read from the payload serde writes
/// for it and written back as serde writes it back.
#[test]
fn a_variant_of_several_slots_reads_and_writes_in_kotlin_as_serde_does() {
    let mut main = String::from("fun main() {\n");
    let mut expected = String::new();
    for row in &ROWS {
        let _ = writeln!(
            main,
            "    row(\"{}\", {}.serializer(), \"\"\"{}\"\"\")",
            row.name, row.read_as, row.json
        );
        let _ = writeln!(expected, "{}: {}", row.name, (row.written)(row.json));
    }
    main.push_str("}\n");
    assert!(
        expected
            .contains(r#"slots under a tag and a content key: {"kind":"Split","held":[[1,2],3]}"#),
        "{expected}"
    );
    let source = [
        KOTLIN_IMPORTS.to_owned(),
        racking_kotlin::kotlin_definition(),
        stacking_kotlin::kotlin_definition(),
        piling_kotlin::kotlin_definition(),
        TOLD.to_owned(),
        main,
    ]
    .join("\n\n");
    let Some(printed) = ran_kotlin(&source) else {
        return;
    };
    assert_eq!(printed, expected);
}
