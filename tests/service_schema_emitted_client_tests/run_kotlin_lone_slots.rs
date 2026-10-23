//! A tuple struct that declares one slot, which serde writes as the slot's value, read and written
//! by the emitted Kotlin types compiled with `-Werror`, over each payload of
//! [`lone_slot_rows`](super::lone_slot_rows), and a tuple that reaches a type parameter of its
//! holder.

#![cfg(feature = "kotlin")]

use core::fmt::Write as _;

use serde::{Deserialize, Serialize};
use tixschema::model_schema;

use super::lone_slot_rows::{Row, held_to_rows, rows};
use super::lone_slots_models::{
    Bale, adjacent_stow_kotlin, bale_kotlin, bale_pair_kotlin, baled_kotlin, bales_by_name_kotlin,
    bales_by_number_kotlin, bales_kotlin, clipped_kotlin, count_kotlin, girth_kotlin,
    girthed_kotlin, labels_by_number_kotlin, laden_kotlin, maybe_bale_kotlin, moniker_kotlin,
    rigger_kotlin, sack_kotlin, sacked_kotlin, single_kotlin, sling_kotlin, slung_kotlin,
    stow_kotlin, stowed_by_name_kotlin, stowed_by_number_kotlin, stowed_list_kotlin,
    stowed_maybe_kotlin, stowed_pair_kotlin, strop_kotlin, tagged_stow_kotlin, tote_kotlin,
    twine_kotlin, twined_kotlin, unskipped_kotlin, untagged_stow_kotlin,
};
#[cfg(feature = "chrono")]
use super::lone_slots_models::{stowed_stamp_kotlin, ticked_kotlin, ticked_millis_kotlin};
use super::runtime::ran_kotlin;

/// The imports the emitted Kotlin and its driver reach for.
const KOTLIN_IMPORTS: &str = "import kotlinx.serialization.*\n\
     import kotlinx.serialization.json.*\n\
     import kotlinx.serialization.descriptors.*\n\
     import kotlinx.serialization.encoding.*\n\
     import kotlinx.serialization.builtins.*";

/// What a run reads a payload with, and prints the answer with: the value written back, or each
/// message of a refusal.
const HARNESS: &str = r#"
fun <T> row(name: String, serializer: KSerializer<T>, payload: String) {
    try {
        val read = Json.decodeFromString(serializer, payload)
        println("$name: read ${Json.encodeToString(serializer, read)}")
    } catch (refused: SerializationException) {
        println("$name: refused")
        (refused.message ?: "").split("; ").forEach { println("    $it") }
    }
}
"#;

/// A type that declares a parameter and holds tuples that reach it, one under an `Option`.
#[model_schema(default_types(Stowed = String))]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Yoked<Stowed> {
    pub maybe: (Option<Stowed>, u8),
    pub pair: (u8, Stowed),
}

/// `text` as a Kotlin string writes it: a backslash, a quote and a `$` each behind a backslash.
fn kotlin_string(text: &str) -> String {
    format!(
        "\"{}\"",
        text.replace('\\', "\\\\")
            .replace('"', "\\\"")
            .replace('$', "\\$")
    )
}

/// Every payload reads and writes in Kotlin as the serde read reads and writes it: the slot's
/// value, with nothing around it. A value that breaks a rule is refused as the Rust `validate()`
/// reports it, the one slot under no step.
#[test]
fn a_struct_of_one_slot_reads_and_writes_in_kotlin_as_serde_does_wherever_it_is_held() {
    let rows = rows();
    let mut main = String::from("fun main() {\n");
    for row in &rows {
        let typed = row
            .read_as
            .replace("UInt32", "UInt")
            .replace("UInt8", "UByte");
        let _ = writeln!(
            main,
            "    row({}, serializer<{typed}>(), {})",
            kotlin_string(row.name),
            kotlin_string(&row.payload)
        );
    }
    main.push_str("}\n");
    let mut source = vec![
        KOTLIN_IMPORTS.to_owned(),
        bale_kotlin::kotlin_definition(),
        sack_kotlin::kotlin_definition(),
        count_kotlin::kotlin_definition(),
        moniker_kotlin::kotlin_definition(),
        baled_kotlin::kotlin_definition(),
        bales_kotlin::kotlin_definition(),
        maybe_bale_kotlin::kotlin_definition(),
        bales_by_name_kotlin::kotlin_definition(),
        bales_by_number_kotlin::kotlin_definition(),
        labels_by_number_kotlin::kotlin_definition(),
        bale_pair_kotlin::kotlin_definition(),
        unskipped_kotlin::kotlin_definition(),
        clipped_kotlin::kotlin_definition(),
        single_kotlin::kotlin_definition(),
        sacked_kotlin::kotlin_definition(),
        stowed_list_kotlin::kotlin_definition(),
        stowed_maybe_kotlin::kotlin_definition(),
        stowed_by_name_kotlin::kotlin_definition(),
        stowed_by_number_kotlin::kotlin_definition(),
        stowed_pair_kotlin::kotlin_definition(),
        tote_kotlin::kotlin_definition(),
        sling_kotlin::kotlin_definition(),
        slung_kotlin::kotlin_definition(),
        girth_kotlin::kotlin_definition(),
        strop_kotlin::kotlin_definition(),
        girthed_kotlin::kotlin_definition(),
        rigger_kotlin::kotlin_definition(),
        twine_kotlin::kotlin_definition(),
        twined_kotlin::kotlin_definition(),
        laden_kotlin::kotlin_definition(),
        stow_kotlin::kotlin_definition(),
        adjacent_stow_kotlin::kotlin_definition(),
        untagged_stow_kotlin::kotlin_definition(),
        tagged_stow_kotlin::kotlin_definition(),
    ];
    #[cfg(feature = "chrono")]
    source.extend([
        ticked_kotlin::kotlin_definition(),
        ticked_millis_kotlin::kotlin_definition(),
        stowed_stamp_kotlin::kotlin_definition(),
    ]);
    source.extend([HARNESS.to_owned(), main]);
    let Some(printed) = ran_kotlin(&source.join("\n\n")) else {
        return;
    };
    held_to_rows(&rows, &printed, "Kotlin");
}

/// A type that declares a parameter and holds a tuple that reaches it, filled at a text and at a
/// model, reads and writes back in Kotlin what serde writes, and refuses an array of another
/// length, as serde refuses it.
#[test]
fn a_tuple_that_reaches_a_type_parameter_reads_and_writes_in_kotlin_as_serde_does() {
    let label = |written: &str| Bale {
        label: written.to_owned(),
    };
    let text = serde_json::to_string(&Yoked {
        maybe: (None, 3),
        pair: (1, "a".to_owned()),
    })
    .unwrap();
    let model = serde_json::to_string(&Yoked {
        maybe: (Some(label("m")), 3),
        pair: (1, label("l")),
    })
    .unwrap();
    let short = r#"{"maybe":[null,3],"pair":[1]}"#;
    assert!(
        serde_json::from_str::<Yoked<String>>(short).is_err(),
        "serde refuses {short}"
    );
    let row = |name: &'static str, read_as: &'static str, payload: &str, expected: String| Row {
        expected,
        name,
        payload: payload.to_owned(),
        read_as,
    };
    let rows = [
        row(
            "a text",
            "Yoked<String>",
            &text,
            format!("a text: read {text}\n"),
        ),
        row(
            "a model",
            "Yoked<Bale>",
            &model,
            format!("a model: read {model}\n"),
        ),
        row(
            "an array of another length",
            "Yoked<String>",
            short,
            "an array of another length: refused\n    invalid length 1, expected 2\n".to_owned(),
        ),
    ];
    let mut main = String::from("fun main() {\n");
    for held in &rows {
        let _ = writeln!(
            main,
            "    row({}, serializer<{}>(), {})",
            kotlin_string(held.name),
            held.read_as,
            kotlin_string(&held.payload)
        );
    }
    main.push_str("}\n");
    let source = [
        KOTLIN_IMPORTS.to_owned(),
        bale_kotlin::kotlin_definition(),
        yoked_kotlin::kotlin_definition(),
        HARNESS.to_owned(),
        main,
    ];
    let Some(printed) = ran_kotlin(&source.join("\n\n")) else {
        return;
    };
    held_to_rows(&rows, &printed, "Kotlin");
}
