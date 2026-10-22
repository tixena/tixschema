//! `()` read and written back by the emitted Kotlin types compiled with `-Werror`, against serde:
//! a field, a brand's slot, a list's item, a tuple's position, an `Option` and a variant's one
//! value, each read from `null` and written as `null`, and each value serde refuses in its place
//! refused with `SerializationException`, an absent key among them. A tuple struct of no slot,
//! which kotlinc refused as a data class of no property, keeps `[]`.

#![cfg(all(feature = "kotlin", feature = "serde"))]

use core::fmt::Write as _;

use super::runtime::ran_kotlin;
use super::unit_models::{
    REFUSED, WRITTEN, probe_maybe_unit_kotlin, probe_nothing_kotlin, probe_slotless_kotlin,
    probe_unit_adjacent_kotlin, probe_unit_choice_kotlin, probe_unit_either_holder_kotlin,
    probe_unit_either_kotlin, probe_units_kotlin, serde_refusal, serde_written,
};

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

/// What the Kotlin read answers of each payload of [`REFUSED`], in its order. An absent key is
/// refused by the plugin's serializer.
const KOTLIN_REFUSED: [&str; 14] = [
    "SerializationException: invalid type: [], expected unit",
    "SerializationException: invalid type: [], expected unit",
    "SerializationException: invalid type: [], expected unit",
    "SerializationException: invalid type: [], expected unit",
    "SerializationException: invalid type: {}, expected unit",
    "SerializationException: invalid type: \"x\", expected unit",
    "MissingFieldException: Field 'unit' is required for type with serial name 'ProbeUnits', but \
     it was missing at path: $",
    "MissingFieldException: Field 'nothing' is required for type with serial name 'ProbeUnits', \
     but it was missing at path: $",
    "SerializationException: invalid type: [], expected unit",
    "SerializationException: invalid type: [], expected unit",
    "SerializationException: invalid type: [], expected unit",
    "SerializationException: missing field `body`",
    "MissingFieldException: Field 'either' is required for type with serial name \
     'ProbeUnitEitherHolder', but it was missing at path: $",
    "SerializationException: expected an array, found null",
];

/// Each payload serde writes is read and written back as serde writes it back, and each payload
/// serde refuses is refused, under `-Werror`.
#[test]
fn unit_is_read_from_null_and_written_as_null_in_kotlin() {
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
        probe_nothing_kotlin::kotlin_definition(),
        probe_units_kotlin::kotlin_definition(),
        probe_maybe_unit_kotlin::kotlin_definition(),
        probe_unit_choice_kotlin::kotlin_definition(),
        probe_unit_adjacent_kotlin::kotlin_definition(),
        probe_unit_either_kotlin::kotlin_definition(),
        probe_unit_either_holder_kotlin::kotlin_definition(),
        probe_slotless_kotlin::kotlin_definition(),
        TOLD.to_owned(),
        main,
    ];
    let Some(printed) = ran_kotlin(&source.join("\n\n")) else {
        return;
    };
    assert_eq!(printed, expected, "the Kotlin read is not serde's");
}
