//! Each integer width Kotlin reads below 64 bits, read by the emitted Kotlin type compiled with
//! `-Werror` against serde: the values at the ends of a range read and written back as serde
//! writes them, and a value outside the range refused in serde's sentence, at the path of the
//! member.

#![cfg(all(feature = "kotlin", feature = "serde"))]

use core::fmt::Write as _;

use super::narrow_integer_models::{
    ENDS, OUTSIDE, payload, serde_refusal, serde_written, spans_kotlin,
};
use super::runtime::ran_kotlin;

const KOTLIN_IMPORTS: &str = "import kotlinx.serialization.*\n\
     import kotlinx.serialization.json.*\n\
     import kotlinx.serialization.descriptors.*\n\
     import kotlinx.serialization.encoding.*\n\
     import kotlinx.serialization.builtins.*";

/// What prints a row: the value read and written back, or the class of what the read throws and
/// its message.
const TOLD: &str = r#"
fun told(at: Int, payload: String) {
    val printed = try {
        "read " + Json.encodeToString(serializer<Spans>(), Json.decodeFromString(serializer<Spans>(), payload))
    } catch (refused: Exception) {
        refused.javaClass.simpleName + ": " + refused.message
    }
    println("$at: $printed")
}
"#;

/// Each end of a range is read and written back as serde writes it, and each value outside a
/// range is refused in the sentence serde refuses it in, under `-Werror`.
#[test]
fn a_value_outside_its_width_is_refused_in_serdes_words_in_kotlin() {
    let mut main = String::from("fun main() {\n");
    let mut expected = String::new();
    let mut at = 0_usize;
    for json in ENDS {
        assert_eq!(serde_refusal(json), None, "serde refuses {json}");
        let _ = writeln!(main, "    told({at}, \"\"\"{json}\"\"\")");
        let _ = writeln!(expected, "{at}: read {}", serde_written(json));
        at += 1;
    }
    for (member, literal) in OUTSIDE {
        let json = payload(member, literal);
        let refusal = serde_refusal(&json);
        assert!(refusal.is_some(), "serde reads {json}");
        let said = refusal.unwrap_or_default();
        let _ = writeln!(main, "    told({at}, \"\"\"{json}\"\"\")");
        let _ = writeln!(
            expected,
            "{at}: SerializationException: {said} at path: $.{member}"
        );
        at += 1;
    }
    main.push_str("}\n");
    let source = [
        KOTLIN_IMPORTS.to_owned(),
        spans_kotlin::kotlin_definition(),
        TOLD.to_owned(),
        main,
    ];
    let Some(printed) = ran_kotlin(&source.join("\n\n")) else {
        return;
    };
    assert_eq!(printed, expected, "the Kotlin read is not serde's");
}
