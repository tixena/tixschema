//! A map flattened as the one member of its struct, and a flattened struct beside a member of its
//! own, read and written by the emitted Kotlin types compiled with `-Werror`, and an optional
//! member beside a flattened one, beside its twin under `nullable`: each payload is written back
//! as the serde read writes it back.

#![cfg(feature = "kotlin")]

use core::fmt::Write as _;

use serde::Serialize;
use serde::de::DeserializeOwned;

use super::flattened_models::{
    Knotting, Netting, Seizing, Splicing, knotting_kotlin, netting_kotlin, seizing_kotlin,
    splicing_kotlin,
};
use super::runtime::ran_kotlin;
use super::tuple_variants_models::labelling_kotlin;

const KOTLIN_IMPORTS: &str = "import kotlinx.serialization.*\n\
     import kotlinx.serialization.json.*\n\
     import kotlinx.serialization.descriptors.*\n\
     import kotlinx.serialization.encoding.*\n\
     import kotlinx.serialization.builtins.*";

/// What prints a row: the value read, written back as serde writes it.
const TOLD: &str = r#"
fun <T> row(name: String, serializer: KSerializer<T>, payload: String) {
    val read = Json.decodeFromString(serializer, payload)
    println("$name: ${Json.encodeToString(serializer, read)}")
}
"#;

/// Each struct's payload, the second's keys in an order serde does not write them in.
const ROWS: [Row; 2] = [
    Row {
        json: r#"{"north":2}"#,
        name: "a map, the one member",
        read_as: "Netting",
        written: written_back::<Netting>,
    },
    Row {
        json: r#"{"note":"a","label":"n"}"#,
        name: "a member of its own beside a flattened struct",
        read_as: "Knotting",
        written: written_back::<Knotting>,
    },
];

/// An optional member beside a flattened one, absent and present, and its twin under `nullable`
/// with the key absent.
const OPTIONAL_ROWS: [Row; 3] = [
    Row {
        json: r#"{"label":"north"}"#,
        name: "an absent optional member beside a flattened one",
        read_as: "Splicing",
        written: written_back::<Splicing>,
    },
    Row {
        json: r#"{"label":"north","spare":3}"#,
        name: "a present optional member beside a flattened one",
        read_as: "Splicing",
        written: written_back::<Splicing>,
    },
    Row {
        json: r#"{"label":"north"}"#,
        name: "an absent nullable member beside a flattened one",
        read_as: "Seizing",
        written: written_back::<Seizing>,
    },
];

/// A payload, the type it is read as, and what serde writes back of it.
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

/// What serde writes back of each row, one line each, and the `main` that prints the same of each
/// in Kotlin.
fn expected_and_main(rows: &[Row]) -> (String, String) {
    let mut expected = String::new();
    let mut main = String::from("fun main() {\n");
    for row in rows {
        let _ = writeln!(expected, "{}: {}", row.name, (row.written)(row.json));
        let _ = writeln!(
            main,
            "    row(\"{}\", {}.serializer(), \"\"\"{}\"\"\")",
            row.name, row.read_as, row.json
        );
    }
    main.push_str("}\n");
    (expected, main)
}

/// Each struct read from its payload and written back as serde writes it back.
#[test]
fn a_struct_of_flattened_members_reads_and_writes_in_kotlin_as_serde_does() {
    let (expected, main) = expected_and_main(&ROWS);
    assert_eq!(
        expected,
        "a map, the one member: {\"north\":2}\n\
         a member of its own beside a flattened struct: {\"label\":\"n\",\"note\":\"a\"}\n"
    );
    let source = [
        KOTLIN_IMPORTS.to_owned(),
        labelling_kotlin::kotlin_definition(),
        netting_kotlin::kotlin_definition(),
        knotting_kotlin::kotlin_definition(),
        TOLD.to_owned(),
        main,
    ];
    let Some(printed) = ran_kotlin(&source.join("\n\n")) else {
        return;
    };
    assert_eq!(printed, expected);
}

/// An optional member beside a flattened one is left out where it holds no value, as serde leaves
/// it out, and one under `nullable` is written as `null`.
#[test]
fn an_optional_member_beside_a_flattened_one_is_written_in_kotlin_as_serde_writes_it() {
    let (expected, main) = expected_and_main(&OPTIONAL_ROWS);
    assert_eq!(
        expected,
        "an absent optional member beside a flattened one: {\"label\":\"north\"}\n\
         a present optional member beside a flattened one: {\"label\":\"north\",\"spare\":3}\n\
         an absent nullable member beside a flattened one: {\"label\":\"north\",\"spare\":null}\n"
    );
    let source = [
        KOTLIN_IMPORTS.to_owned(),
        labelling_kotlin::kotlin_definition(),
        splicing_kotlin::kotlin_definition(),
        seizing_kotlin::kotlin_definition(),
        TOLD.to_owned(),
        main,
    ];
    let Some(printed) = ran_kotlin(&source.join("\n\n")) else {
        return;
    };
    assert_eq!(printed, expected);
}
