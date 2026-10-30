//! A struct whose every member is flattened, and an enum whose variant holds only a flattened
//! member, read and written back by the emitted Kotlin types compiled with `-Werror`.

#![cfg(feature = "kotlin")]

use core::fmt::Write as _;

use super::all_flattened_models::{
    REFUSED, ROWS, assert_printed_as_serde_writes, berth_kotlin, gimbal_kotlin, hoist_kotlin,
    keelson_kotlin, lantern_kotlin, mast_kotlin, rigged_kotlin, rigging_kotlin, rigs_kotlin,
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
    main.push_str("}\n");
    main
}

/// Each model reads its payload and writes back what serde writes, and refuses a payload serde
/// refuses.
#[test]
fn a_model_of_flattened_members_alone_is_read_and_written_back_in_kotlin() {
    let source = [
        KOTLIN_IMPORTS.to_owned(),
        lantern_kotlin::kotlin_definition(),
        mast_kotlin::kotlin_definition(),
        berth_kotlin::kotlin_definition(),
        keelson_kotlin::kotlin_definition(),
        gimbal_kotlin::kotlin_definition(),
        rigging_kotlin::kotlin_definition(),
        rigged_kotlin::kotlin_definition(),
        rigs_kotlin::kotlin_definition(),
        hoist_kotlin::kotlin_definition(),
        main_of_the_rows(),
    ];
    let Some(printed) = ran_kotlin(&source.join("\n\n")) else {
        return;
    };
    assert_printed_as_serde_writes(&printed);
}
