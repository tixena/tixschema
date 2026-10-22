//! The emitted Kotlin types, compiled with `-Werror`, handed each value of [`WIRE_FORMS`] and
//! written through their own serializer, which every Kotlin client writes a message through: each
//! prints the payload it sends, or `refused`, as the table's Kotlin column states, and reads every
//! payload any client sends.

#![cfg(all(feature = "kotlin", feature = "chrono"))]

use core::fmt::Write as _;

use super::runtime::ran_kotlin;
use super::wire_form_rows::{Held, constructed, handed, held, printed, sent_by_any};
use super::wire_forms_models::{
    WireFormRow, conveyed_day_kotlin, conveyed_f32_kotlin, conveyed_f64_kotlin,
    conveyed_flag_kotlin, conveyed_hour_kotlin, conveyed_i8_kotlin, conveyed_i16_kotlin,
    conveyed_i32_kotlin, conveyed_i64_kotlin, conveyed_instant_kotlin, conveyed_isize_kotlin,
    conveyed_letter_kotlin, conveyed_literal_count_kotlin, conveyed_literal_flag_kotlin,
    conveyed_literal_text_kotlin, conveyed_millis_kotlin, conveyed_slot_kotlin,
    conveyed_slotted_kotlin, conveyed_stamp_kotlin, conveyed_text_kotlin, conveyed_u8_kotlin,
    conveyed_u16_kotlin, conveyed_u32_kotlin,
};

const KOTLIN_IMPORTS: &str = "import kotlinx.serialization.*\n\
     import kotlinx.serialization.json.*\n\
     import kotlinx.serialization.descriptors.*\n\
     import kotlinx.serialization.encoding.*\n\
     import kotlinx.serialization.builtins.*";

/// What prints a value handed: the payload the serializer writes, or `refused` where it throws;
/// and what prints a payload read: `read`, or the refusal.
const PRINTERS: &str = r#"
fun <T> sent(serializer: KSerializer<T>, handed: () -> T): String =
    try { Json.encodeToString(serializer, handed()) } catch (refusal: SerializationException) { "refused" }

fun <T> read(serializer: KSerializer<T>, payload: String): String =
    try { Json.decodeFromString(serializer, payload); "read" } catch (refusal: Exception) { "refused $payload: ${refusal.message}" }
"#;

/// The Kotlin value of `json`, as the model `held_by` holds it.
fn kotlin_value(held_by: &str, json: &str) -> String {
    let text = json.trim_matches('"');
    match held(held_by) {
        Held::Flag => json.to_owned(),
        Held::Text | Held::Instant => json.replace('$', "\\$"),
        Held::Millis => format!("\"{json}\".toLong()"),
        Held::Slot => format!("ConveyedSlot.{text}"),
        Held::Number("f64") if json == "NaN" => "Double.NaN".to_owned(),
        Held::Number(number) => {
            let parsed = match number {
                "u8" => "toUByte",
                "u16" => "toUShort",
                "u32" => "toUInt",
                "i8" => "toByte",
                "i16" => "toShort",
                "i32" => "toInt",
                "f32" => "toFloat",
                "f64" => "toDouble",
                _ => "toLong",
            };
            format!("\"{json}\".{parsed}()")
        }
    }
}

/// The model of `row` holding its value as it was handed.
fn kotlin_handed(row: &WireFormRow) -> String {
    let value = kotlin_value(row.held_by, row.handed);
    constructed(
        row.held_by,
        "NAME = ",
        &|key| format!("mapOf({key} to true)"),
        &|listed| format!("listOf({listed})"),
        &value,
        &value,
    )
}

/// Every value handed as the table's Kotlin column states, and every payload any client sends read
/// back.
#[test]
fn each_value_is_sent_by_kotlin_in_the_form_every_target_reads() {
    let rows = handed(|row| row.kotlin);
    let mut expected = printed(&rows);
    let mut main = String::from(PRINTERS);
    main.push_str("fun main() {\n");
    for (row, _) in &rows {
        let _ = writeln!(
            main,
            "    println(sent({}.serializer()) {{ {} }})",
            row.held_by,
            kotlin_handed(row)
        );
    }
    for (held_by, payload) in sent_by_any(|row| row.kotlin) {
        let _ = writeln!(expected, "read");
        let _ = writeln!(
            main,
            "    println(read({held_by}.serializer(), \"\"\"{payload}\"\"\"))"
        );
    }
    main.push_str("}\n");
    let source = [
        KOTLIN_IMPORTS.to_owned(),
        conveyed_flag_kotlin::kotlin_definition(),
        conveyed_letter_kotlin::kotlin_definition(),
        conveyed_text_kotlin::kotlin_definition(),
        conveyed_u8_kotlin::kotlin_definition(),
        conveyed_u16_kotlin::kotlin_definition(),
        conveyed_u32_kotlin::kotlin_definition(),
        conveyed_i8_kotlin::kotlin_definition(),
        conveyed_i16_kotlin::kotlin_definition(),
        conveyed_i32_kotlin::kotlin_definition(),
        conveyed_i64_kotlin::kotlin_definition(),
        conveyed_isize_kotlin::kotlin_definition(),
        conveyed_f32_kotlin::kotlin_definition(),
        conveyed_f64_kotlin::kotlin_definition(),
        conveyed_literal_count_kotlin::kotlin_definition(),
        conveyed_literal_flag_kotlin::kotlin_definition(),
        conveyed_literal_text_kotlin::kotlin_definition(),
        conveyed_day_kotlin::kotlin_definition(),
        conveyed_hour_kotlin::kotlin_definition(),
        conveyed_stamp_kotlin::kotlin_definition(),
        conveyed_instant_kotlin::kotlin_definition(),
        conveyed_millis_kotlin::kotlin_definition(),
        conveyed_slot_kotlin::kotlin_definition(),
        conveyed_slotted_kotlin::kotlin_definition(),
        main,
    ];
    let Some(ran) = ran_kotlin(&source.join("\n\n")) else {
        return;
    };
    assert_eq!(
        ran, expected,
        "the Kotlin client does not send each value as the table states"
    );
}
