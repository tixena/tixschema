//! A `DateTime<Tz>` read and written back by the emitted Kotlin types compiled with `-Werror`
//! against serde: a date of whole seconds, of milliseconds, in a list, as a map's key and value, in
//! //! a tuple's slot and in a variant's slot, and an optional date under `as_number`.

#![cfg(all(feature = "kotlin", feature = "chrono"))]

use core::fmt::Write as _;

use super::dates_models::{
    Nudge, SAME_AS_SERDE, landing_kotlin, nudge_kotlin, positions_kotlin, serde_back,
    stamped_kotlin,
};
use super::runtime::ran_kotlin;

const KOTLIN_IMPORTS: &str = "import kotlinx.serialization.*\n\
     import kotlinx.serialization.json.*\n\
     import kotlinx.serialization.descriptors.*\n\
     import kotlinx.serialization.encoding.*\n\
     import kotlinx.serialization.builtins.*";

/// What prints a row: the value read and written back, or `refused`.
const TOLD: &str = r#"
fun told(name: String, read: () -> String) {
    try {
        println("$name: read ${read()}")
    } catch (refused: Exception) {
        println("$name: refused")
    }
}
"#;

/// What prints a row: the message of the refusal, or `read`.
const MESSAGE: &str = r#"
fun message(name: String, read: () -> Unit) {
    try {
        read()
        println("$name: read")
    } catch (refused: SerializationException) {
        println("$name: ${refused.message}")
    }
}
"#;

/// Each payload is read and written back by the Kotlin types as serde reads and writes it.
#[test]
fn a_date_in_every_position_is_read_and_written_back_by_kotlin_as_serde_writes_it() {
    let mut main = String::from("fun main() {\n");
    let mut expected = String::new();
    for (name, model, payload) in SAME_AS_SERDE {
        let _ = writeln!(
            main,
            "    told(\"{name}\") {{ Json.encodeToString({model}.serializer(), \
             Json.decodeFromString({model}.serializer(), \"\"\"{payload}\"\"\")) }}"
        );
        let _ = writeln!(expected, "{name}: read {}", serde_back(model, payload));
    }
    main.push_str("}\n");
    let source = [
        KOTLIN_IMPORTS.to_owned(),
        landing_kotlin::kotlin_definition(),
        nudge_kotlin::kotlin_definition(),
        positions_kotlin::kotlin_definition(),
        stamped_kotlin::kotlin_definition(),
        TOLD.to_owned(),
        main,
    ]
    .join("\n\n");
    let Some(printed) = ran_kotlin(&source) else {
        return;
    };
    assert_eq!(printed, expected, "the Kotlin write is not serde's");
}

/// A date under `as_number` that holds a float, or an integer a `u64` holds above an `i64`, is
/// refused by Kotlin in the words serde refuses it in, placed at the member.
#[test]
fn a_date_under_as_number_is_refused_by_kotlin_in_the_words_serde_refuses_it_in() {
    let rows = [
        ("a fraction", r#"{"due":1.5}"#),
        ("a whole float", r#"{"due":1e3}"#),
        ("an integer above an i64", r#"{"due":9223372036854775808}"#),
    ];
    let mut main = String::from("fun main() {\n");
    let mut expected = String::new();
    for (name, json) in rows {
        let _ = writeln!(
            main,
            "    message(\"{name}\") {{ Json.decodeFromString(Nudge.serializer(), \"\"\"{json}\"\"\") }}"
        );
        let refused = serde_json::from_str::<Nudge>(json).unwrap_err().to_string();
        let (words, _) = refused.rsplit_once(" at line ").unwrap();
        let _ = writeln!(expected, "{name}: {words} at path: $.due");
    }
    main.push_str("}\n");
    let source = [
        KOTLIN_IMPORTS.to_owned(),
        nudge_kotlin::kotlin_definition(),
        MESSAGE.to_owned(),
        main,
    ]
    .join("\n\n");
    let Some(printed) = ran_kotlin(&source) else {
        return;
    };
    assert_eq!(printed, expected, "the Kotlin refusal is not serde's");
}
