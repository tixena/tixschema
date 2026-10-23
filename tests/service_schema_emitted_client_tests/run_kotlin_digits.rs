//! An `i64` read from the string of its digits and written back as it by the emitted Kotlin types
//! compiled with `-Werror`, against serde, in each place a member holds one, and each payload
//! serde refuses refused with `SerializationException`.

#![cfg(all(feature = "kotlin", feature = "serde"))]

use core::fmt::Write as _;

use super::digit_models::{
    REFUSED, WRITTEN, serde_refusal, serde_written, sixty_event_kotlin, sixty_id_kotlin,
    sixty_level_kotlin, sixty_tally_kotlin,
};
use super::runtime::ran_kotlin;

const KOTLIN_IMPORTS: &str = "import kotlinx.serialization.*\n\
     import kotlinx.serialization.json.*\n\
     import kotlinx.serialization.descriptors.*\n\
     import kotlinx.serialization.encoding.*\n\
     import kotlinx.serialization.builtins.*";

/// What prints a row: the value read, written back, or the class of the `SerializationException`
/// the read throws and its message.
const TOLD: &str = r#"
fun <T> told(at: Int, serializer: KSerializer<T>, payload: String) {
    val printed = try {
        "read " + Json.encodeToString(serializer, Json.decodeFromString(serializer, payload))
    } catch (refused: SerializationException) {
        refused.javaClass.simpleName + ": " + refused.message
    }
    println("$at: $printed")
}
"#;

/// What the Kotlin read answers of each payload of [`REFUSED`], in its order.
const KOTLIN_REFUSED: [&str; 5] = [
    "SerializationException: expected an i64 as a string of its decimal digits, found \
     \"9223372036854775808\"",
    "SerializationException: expected an i64 as a string of its decimal digits, found \"seven\"",
    "SerializationException: expected an i64 as a string of its decimal digits, found \
     \"-9223372036854775809\"",
    "SerializationException: expected an i64 as a string of its decimal digits, found \"1.5\"",
    "SerializationException: 'level': too large: maximum is 5, got 6",
];

/// Each payload serde writes is read and written back as serde writes it back, and each payload
/// serde refuses is refused, under `-Werror`.
#[test]
fn an_i64_is_read_from_its_digits_and_written_as_them_in_kotlin() {
    let mut main = String::from("fun main() {\n");
    let mut expected = String::new();
    let mut at = 0_usize;
    for (read_as, json) in WRITTEN {
        let _ = writeln!(
            main,
            "    told({at}, serializer<{read_as}>(), \"\"\"{json}\"\"\")"
        );
        let _ = writeln!(expected, "{at}: read {}", serde_written(read_as, json));
        at += 1;
    }
    for ((read_as, json, said), refused) in REFUSED.into_iter().zip(KOTLIN_REFUSED) {
        assert_eq!(
            serde_refusal(read_as, json).as_deref(),
            Some(said),
            "serde no longer refuses {json}"
        );
        let _ = writeln!(
            main,
            "    told({at}, serializer<{read_as}>(), \"\"\"{json}\"\"\")"
        );
        let _ = writeln!(expected, "{at}: {refused}");
        at += 1;
    }
    main.push_str("}\n");
    let source = [
        KOTLIN_IMPORTS.to_owned(),
        sixty_tally_kotlin::kotlin_definition(),
        sixty_id_kotlin::kotlin_definition(),
        sixty_event_kotlin::kotlin_definition(),
        sixty_level_kotlin::kotlin_definition(),
        TOLD.to_owned(),
        main,
    ];
    let Some(printed) = ran_kotlin(&source.join("\n\n")) else {
        return;
    };
    assert_eq!(printed, expected, "the Kotlin read is not serde's");
}
