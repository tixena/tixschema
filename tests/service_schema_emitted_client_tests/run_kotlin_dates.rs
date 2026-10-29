//! A `DateTime<Tz>` read and written back by the emitted Kotlin types compiled with `-Werror`
//! against serde: a date of whole seconds, of milliseconds, in a list, as a map's key and value, in
//! a tuple's slot and in a variant's slot, and an optional date under `as_number`. Kotlin holds
//! the text as written, so what it writes is read by serde to the instant it was handed.

#![cfg(all(feature = "kotlin", feature = "chrono"))]

use core::fmt::Write as _;

use super::dates_models::{
    SAME_AS_SERDE, landing_kotlin, nudge_kotlin, positions_kotlin, serde_back, stamped_kotlin,
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
