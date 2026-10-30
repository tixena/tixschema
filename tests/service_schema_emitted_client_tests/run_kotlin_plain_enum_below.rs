//! A plain enum flattened into a holder declared above it, read and written back by the emitted
//! Kotlin types compiled with `-Werror`.

#![cfg(feature = "kotlin")]

use core::fmt::Write as _;

use super::plain_enum_below_models::{
    BRANDED, REFUSED, ROWS, assert_printed_as_serde_writes, casement_kotlin, cringle_kotlin,
    glazing_kotlin, mullion_kotlin, pane_kotlin, reefing_kotlin, transom_kotlin, weathering_kotlin,
};
use super::runtime::ran_kotlin;

const KOTLIN_IMPORTS: &str = "import kotlinx.serialization.*\n\
     import kotlinx.serialization.json.*\n\
     import kotlinx.serialization.descriptors.*\n\
     import kotlinx.serialization.encoding.*\n\
     import kotlinx.serialization.builtins.*";

/// What prints each row read and written back, then whether each refused payload is refused.
fn main_of_the_rows() -> String {
    let mut main = String::from("fun main() {\n");
    for (name, payload) in ROWS {
        let _ = writeln!(
            main,
            "    println(\"{name}: ${{Json.encodeToString({name}.serializer(), \
             Json.decodeFromString({name}.serializer(), \"\"\"{payload}\"\"\"))}}\")"
        );
    }
    for (name, payload) in REFUSED {
        let _ = writeln!(
            main,
            "    println(\"{name} refused: ${{runCatching {{ Json.decodeFromString({name}.serializer(), \
             \"\"\"{payload}\"\"\") }}.isFailure}}\")"
        );
    }
    let (name, payload) = BRANDED;
    let _ = writeln!(
        main,
        "    println(\"{name} refused: ${{runCatching {{ Json.decodeFromString({name}.serializer(), \
         \"\"\"{payload}\"\"\") }}.isFailure}}\")"
    );
    main.push_str("}\n");
    main
}

/// Each model reads its payload and writes back what serde writes, and refuses a payload serde
/// refuses.
#[test]
fn a_plain_enum_flattened_below_its_holder_is_read_and_written_back_in_kotlin() {
    let source = [
        KOTLIN_IMPORTS.to_owned(),
        casement_kotlin::kotlin_definition(),
        transom_kotlin::kotlin_definition(),
        mullion_kotlin::kotlin_definition(),
        weathering_kotlin::kotlin_definition(),
        reefing_kotlin::kotlin_definition(),
        cringle_kotlin::kotlin_definition(),
        pane_kotlin::kotlin_definition(),
        glazing_kotlin::kotlin_definition(),
        main_of_the_rows(),
    ];
    let Some(printed) = ran_kotlin(&source.join("\n\n")) else {
        return;
    };
    let (rows, branded) = printed.trim_end().rsplit_once('\n').unwrap();
    assert_eq!(branded, "Reefing refused: true", "{printed}");
    assert_printed_as_serde_writes(&format!("{rows}\n"));
}
