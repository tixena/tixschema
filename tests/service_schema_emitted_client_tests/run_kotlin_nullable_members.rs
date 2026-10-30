//! An absent key of a nullable member, read by the emitted Kotlin types compiled with `-Werror`:
//! each payload prints what the serde read prints of it.

#![cfg(feature = "kotlin")]

use core::fmt::Write as _;

use super::nullable_member_models::{
    Berth, ROWS, agrees, berth_kotlin, expected, hawser_kotlin, keel_kotlin, slipway_kotlin,
};
use super::runtime::ran_kotlin;

const KOTLIN_IMPORTS: &str = "import kotlinx.serialization.*\n\
     import kotlinx.serialization.json.*\n\
     import kotlinx.serialization.descriptors.*\n\
     import kotlinx.serialization.encoding.*\n\
     import kotlinx.serialization.builtins.*";

/// What prints a row: the value read, written back with every member, or that the read threw.
const TOLD: &str = r#"
val everyMember = Json { encodeDefaults = true }

fun told(name: String, read: () -> String) {
    try {
        println("$name: read ${read()}")
    } catch (refused: SerializationException) {
        println("$name: refused")
    }
}
"#;

#[test]
fn an_absent_key_of_a_nullable_member_is_read_in_kotlin_as_serde_reads_it() {
    let mut main = String::from("fun main() {\n");
    for &(name, read_as, payload) in &ROWS {
        let _ = writeln!(
            main,
            "    told(\"{name}\") {{ everyMember.encodeToString(serializer<{read_as}>(), \
             Json.decodeFromString(serializer<{read_as}>(), \"\"\"{payload}\"\"\")) }}"
        );
    }
    main.push_str(
        "    println(\"written: ${Json.encodeToString(serializer<Berth>(), Berth())}\")\n}\n",
    );
    let source = [
        KOTLIN_IMPORTS.to_owned(),
        hawser_kotlin::kotlin_definition(),
        berth_kotlin::kotlin_definition(),
        keel_kotlin::kotlin_definition(),
        slipway_kotlin::kotlin_definition(),
        TOLD.to_owned(),
        main,
    ]
    .join("\n\n");
    let Some(printed) = ran_kotlin(&source) else {
        return;
    };
    let (read, written) = printed.rsplit_once("written: ").unwrap();
    assert!(agrees(read, &expected()), "{printed}\n{:#?}", expected());
    assert_eq!(
        written.trim_end(),
        serde_json::to_string(&serde_json::from_str::<Berth>("{}").unwrap()).unwrap(),
        "a nullable member's key is written with null under the default Json"
    );
}
