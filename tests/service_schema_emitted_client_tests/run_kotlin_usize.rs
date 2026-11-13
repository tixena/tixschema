//! A `usize` read as a `ULong` and written back as a JSON number by the emitted Kotlin types
//! compiled with `-Werror`, against serde, in each place a member holds one: the values above
//! `Long.MAX_VALUE` included, each payload serde refuses refused, a default read as serde reads it,
//! and `validate()` answering what the Rust `validate()` answers.

#![cfg(all(feature = "kotlin", feature = "serde"))]

use core::fmt::Write as _;

use super::runtime::ran_kotlin;
use super::usize_models::{
    DEFAULTED, OUT_OF_BOUNDS, REFUSED, WRITTEN, offsets_kotlin, rust_violations, serde_defaulted,
    serde_refusal, serde_written,
};
use super::usize_positions_models::{holding_kotlin, placed_kotlin, slot_index_kotlin};

const KOTLIN_IMPORTS: &str = "import kotlinx.serialization.*\n\
     import kotlinx.serialization.json.*\n\
     import kotlinx.serialization.descriptors.*\n\
     import kotlinx.serialization.encoding.*\n\
     import kotlinx.serialization.builtins.*";

/// What prints a row: the value read, written back, or the class of what the read throws and its
/// message; the default a read takes for an absent key; and what `validate()` answers of a value
/// read with no judgement.
const TOLD: &str = r#"
fun <T> told(at: Int, serializer: KSerializer<T>, payload: String) {
    val printed = try {
        "read " + Json.encodeToString(serializer, Json.decodeFromString(serializer, payload))
    } catch (refused: Exception) {
        refused.javaClass.simpleName + ": " + refused.message
    }
    println("$at: $printed")
}

fun defaulted(at: Int, payload: String) {
    println("$at: defaulted " + Json.decodeFromString(serializer<Placed>(), payload).defaulted)
}

fun validated(at: Int, bounded: ULong) {
    val payload = "{\"at\":0,\"bounded\":$bounded,\"by_index\":{},\"span\":{\"start\":0,\"end\":0}}"
    println("$at: validate: " + Json.decodeFromString(OffsetsUnjudged, payload).validationIssues().joinToString("; "))
}
"#;

/// The Kotlin read of each payload of [`REFUSED`], in its order: each refusal in serde's words, at
/// the path of the number.
const KOTLIN_REFUSED: [&str; 6] = [
    "SerializationException: invalid value: integer `-1`, expected usize at path: $.at",
    "SerializationException: invalid type: floating point `1.8446744073709552e+19`, expected \
     usize at path: $.at",
    "SerializationException: invalid type: floating point `1.5`, expected usize at path: $.at",
    "SerializationException: 'bounded': too small: minimum is 1, got 0",
    "SerializationException: 'bounded': too large: maximum is 4096, got 4097",
    "SerializationException: 'pinned': not the literal 18446744073709551615, got \
     9223372036854775808",
];

/// Each payload serde writes is read and written back as serde writes it back, each payload serde
/// refuses is refused, an absent key is read as its default, and `validate()` answers the Rust
/// sentence, under `-Werror`.
#[test]
fn a_usize_is_read_and_written_as_a_ulong_in_kotlin() {
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
        assert!(
            refused.contains(said),
            "the row does not hold serde's {said}"
        );
        let _ = writeln!(
            main,
            "    told({at}, serializer<{read_as}>(), \"\"\"{json}\"\"\")"
        );
        let _ = writeln!(expected, "{at}: {refused}");
        at += 1;
    }
    let _ = writeln!(main, "    defaulted({at}, \"\"\"{DEFAULTED}\"\"\")");
    let _ = writeln!(expected, "{at}: defaulted {}", serde_defaulted());
    at += 1;
    for bounded in OUT_OF_BOUNDS {
        let _ = writeln!(main, "    validated({at}, {bounded}uL)");
        let _ = writeln!(expected, "{at}: validate: {}", rust_violations(bounded));
        at += 1;
    }
    main.push_str("}\n");
    let source = [
        KOTLIN_IMPORTS.to_owned(),
        offsets_kotlin::kotlin_definition(),
        slot_index_kotlin::kotlin_definition(),
        holding_kotlin::kotlin_definition(),
        placed_kotlin::kotlin_definition(),
        TOLD.to_owned(),
        main,
    ];
    let Some(printed) = ran_kotlin(&source.join("\n\n")) else {
        return;
    };
    assert_eq!(printed, expected, "the Kotlin read is not serde's");
}
