//! The emitted Kotlin types compiled with `-Werror`, named as Rust names them: a field written
//! `Self` and a variant whose own `rename_all` names its members under every tagging. A model
//! declared below its holder is pinned in `run_kotlin_read.rs`. Each payload prints what the
//! serde read prints of it.

#![cfg(feature = "kotlin")]

use core::fmt::Write as _;

use super::named_by_rust_models::{
    bough_kotlin, carriage_adjacent_kotlin, carriage_external_kotlin, carriage_kotlin,
    carriage_untagged_kotlin, fork_kotlin, rope_kotlin, spur_kotlin, tip_kotlin,
};
use super::named_by_rust_rows::{
    CARRIAGE_ROWS, Row, SELF_ROWS, TIP_ROWS, declares_a_parameter, expected, normalized,
};
use super::runtime::ran_kotlin;

const KOTLIN_IMPORTS: &str = "import kotlinx.serialization.*\n\
     import kotlinx.serialization.json.*\n\
     import kotlinx.serialization.descriptors.*\n\
     import kotlinx.serialization.encoding.*\n\
     import kotlinx.serialization.builtins.*";

/// What prints a row: the value read and written back, or `refused`, with each message of what
/// the read refused where the row compares them.
const TOLD: &str = r#"
fun <T> row(name: String, serializer: KSerializer<T>, payload: String, judged: Boolean) {
    try {
        val read = Json.decodeFromString(serializer, payload)
        println("$name: read ${Json.encodeToString(serializer, read)}")
    } catch (refused: Exception) {
        println("$name: refused")
        if (judged && refused::class == SerializationException::class) {
            (refused.message ?: "").split("; ").forEach { println("    $it") }
        }
    }
}
"#;

/// The emitted types of the models a field written `Self` is declared on.
fn self_definitions() -> Vec<String> {
    vec![
        bough_kotlin::kotlin_definition(),
        spur_kotlin::kotlin_definition(),
        fork_kotlin::kotlin_definition(),
        rope_kotlin::kotlin_definition(),
    ]
}

/// The emitted types of the enums whose variant writes its own `rename_all`.
fn carriage_definitions() -> Vec<String> {
    vec![
        carriage_kotlin::kotlin_definition(),
        carriage_external_kotlin::kotlin_definition(),
        carriage_adjacent_kotlin::kotlin_definition(),
        carriage_untagged_kotlin::kotlin_definition(),
    ]
}

/// What the emitted Kotlin prints of `rows`, or `None` where no toolchain is reachable.
fn printed(rows: &[Row], definitions: Vec<String>) -> Option<String> {
    let mut main = String::from("fun main() {\n");
    for row in rows {
        let read_as = if declares_a_parameter(row.read_as) {
            format!("{}<Int>", row.read_as)
        } else {
            row.read_as.to_owned()
        };
        let _ = writeln!(
            main,
            "    row(\"{}\", serializer<{read_as}>(), \"\"\"{}\"\"\", {})",
            row.name, row.json, row.judged
        );
    }
    main.push_str("}\n");
    let mut source = vec![KOTLIN_IMPORTS.to_owned()];
    source.extend(definitions);
    source.extend([TOLD.to_owned(), main]);
    ran_kotlin(&source.join("\n\n")).map(|told| normalized(&told))
}

/// A field written `Self` names the enclosing type, in an `Option`, a `Box`, a map's value, a
/// variant's slot and a generic type, and `validate()` and the read name a child's place.
#[test]
fn a_field_written_self_names_its_type_in_kotlin() {
    let expected = expected(&SELF_ROWS);
    assert!(
        expected.contains("    'twigs.0.label': too short: minimum length is 1, got 0"),
        "{expected}"
    );
    let Some(printed) = printed(&SELF_ROWS, self_definitions()) else {
        return;
    };
    assert_eq!(printed, expected);
}

/// A field written `Self` in an `Option` over a `Box` names the enclosing type.
#[test]
fn an_optional_box_of_self_names_its_type_in_kotlin() {
    let expected = expected(&TIP_ROWS);
    assert!(
        expected.contains("    'next.label': too short: minimum length is 1, got 0"),
        "{expected}"
    );
    let Some(printed) = printed(&TIP_ROWS, vec![tip_kotlin::kotlin_definition()]) else {
        return;
    };
    assert_eq!(printed, expected);
}

/// A variant's members cross under its own `rename_all` in Kotlin, as serde names them.
#[test]
fn a_variants_own_rename_all_names_its_members_in_kotlin() {
    let expected = expected(&CARRIAGE_ROWS);
    assert!(
        expected.contains(r#"internal, the variant rename_all: read {"axle_total":3,"mode":"OverLand","roadName":"DR-1"}"#),
        "{expected}"
    );
    let Some(printed) = printed(&CARRIAGE_ROWS, carriage_definitions()) else {
        return;
    };
    assert_eq!(printed, expected);
}
