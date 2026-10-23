//! An `i64` read and written back as a JSON number by the emitted Kotlin types compiled with
//! `-Werror`, against serde, in each place a member holds one, and each payload serde refuses
//! refused, the string of an `i64`'s digits included.

#![cfg(all(feature = "kotlin", feature = "serde"))]

use core::fmt::Write as _;

use super::runtime::ran_kotlin;
use super::wide_integer_models::{
    REFUSED, WRITTEN, serde_refusal, serde_written, sixty_beside_kotlin, sixty_event_kotlin,
    sixty_id_kotlin, sixty_level_kotlin, sixty_locked_kotlin, sixty_reached_kotlin,
    sixty_revision_kotlin, sixty_tally_kotlin,
};

const KOTLIN_IMPORTS: &str = "import kotlinx.serialization.*\n\
     import kotlinx.serialization.json.*\n\
     import kotlinx.serialization.descriptors.*\n\
     import kotlinx.serialization.encoding.*\n\
     import kotlinx.serialization.builtins.*";

/// What prints a row: the value read, written back, or the class of what the read throws and its
/// message.
const TOLD: &str = r#"
fun <T> told(at: Int, serializer: KSerializer<T>, payload: String) {
    val printed = try {
        "read " + Json.encodeToString(serializer, Json.decodeFromString(serializer, payload))
    } catch (refused: Exception) {
        refused.javaClass.simpleName + ": " + refused.message
    }
    println("$at: $printed")
}
"#;

/// What the Kotlin read answers of each payload of [`REFUSED`], in its order. The read refuses the
/// string of an `i64`'s digits in serde's words, and reads a number through the tree of the
/// payload, so the path of an out-of-range number starts at the number.
const KOTLIN_REFUSED: [&str; 8] = [
    "JsonDecodingException: Failed to parse literal '9223372036854775808' as a long value at \
     path: $.primitive\nJSON input: 9223372036854775808",
    "SerializationException: invalid type: string \"7\", expected i64",
    "JsonDecodingException: Failed to parse literal '-9223372036854775809' as a long value at \
     path: $.primitive\nJSON input: -9223372036854775809",
    "JsonDecodingException: Failed to parse literal '1.5' as a long value at path: \
     $.primitive\nJSON input: 1.5",
    "SerializationException: 'level': too large: maximum is 5, got 6",
    "JsonDecodingException: Failed to parse literal '9223372036854775808' as a long value at \
     path: $.primitive\nJSON input: 9223372036854775808",
    "SerializationException: 'fixed': not the literal 214, got 215",
    "SerializationException: invalid type: string \"x\", expected i64",
];

/// Each payload serde writes is read and written back as serde writes it back, and each payload
/// serde refuses is refused, under `-Werror`.
#[test]
fn an_i64_is_read_and_written_as_a_number_in_kotlin() {
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
        sixty_revision_kotlin::kotlin_definition(),
        sixty_reached_kotlin::kotlin_definition(),
        sixty_locked_kotlin::kotlin_definition(),
        sixty_beside_kotlin::kotlin_definition(),
        TOLD.to_owned(),
        main,
    ];
    let Some(printed) = ran_kotlin(&source.join("\n\n")) else {
        return;
    };
    assert_eq!(printed, expected, "the Kotlin read is not serde's");
}
