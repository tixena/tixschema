//! A generic enum whose variants hold tuples that reach its type parameter, under each tagging, and
//! a generic struct that holds a nested tuple, read and written by the emitted Kotlin types
//! compiled with `-Werror`, filled at a text and at a number. Each payload the serde read reads
//! prints what it prints.

#![cfg(feature = "kotlin")]

use core::fmt::Write as _;

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use tixschema::model_schema;

use super::runtime::ran_kotlin;

const KOTLIN_IMPORTS: &str = "import kotlinx.serialization.*\n\
     import kotlinx.serialization.json.*\n\
     import kotlinx.serialization.descriptors.*\n\
     import kotlinx.serialization.encoding.*\n\
     import kotlinx.serialization.builtins.*";

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

/// An enum under no tag attribute, with a variant of several slots and a newtype variant over a
/// tuple, both reaching the parameter.
#[model_schema(default_types(Stowed = String))]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum Split<Stowed> {
    Halves(Stowed, u8),
    Paired((Stowed, u32)),
}

/// The same under a tag and a content key.
#[model_schema(default_types(Stowed = String))]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "t", content = "c")]
pub enum AdjacentSplit<Stowed> {
    Halves(Stowed, u8),
    Paired((Stowed, u32)),
}

/// An untagged enum with a newtype variant over a tuple and a struct variant.
#[model_schema(default_types(Stowed = String))]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum Loose<Stowed> {
    Named { held: Stowed },
    Paired((Stowed, u32)),
}

/// A struct that holds a tuple and a tuple nested in a tuple.
#[model_schema(default_types(Stowed = String))]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Stack<Stowed> {
    pub nested: ((Stowed, u8), u32),
    pub pair: (Stowed, u32),
}

/// A tuple struct of several slots, one of them the parameter.
#[model_schema(default_types(Stowed = String))]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Pairing<Stowed>(Stowed, u32);

struct Row {
    expected: String,
    kotlin: String,
}

fn kotlin_string(text: &str) -> String {
    format!(
        "\"{}\"",
        text.replace('\\', "\\\\")
            .replace('"', "\\\"")
            .replace('$', "\\$")
    )
}

/// The payload `payload` read as `read_as`, held to what the serde read of `Held` prints.
fn row_of<Held>(name: &str, read_as: &str, payload: &str) -> Row
where
    Held: DeserializeOwned + Serialize,
{
    let read = serde_json::from_str::<Held>(payload).unwrap();
    Row {
        expected: format!("{name}: read {}\n", serde_json::to_string(&read).unwrap()),
        kotlin: kotlin_call(name, read_as, payload),
    }
}

/// The payload `payload`, which serde refuses, read as `read_as`, held to the message the Kotlin
/// read refuses it with.
fn refused_row_of<Held>(name: &str, read_as: &str, payload: &str, message: &str) -> Row
where
    Held: DeserializeOwned,
{
    assert!(
        serde_json::from_str::<Held>(payload).is_err(),
        "serde reads {payload}"
    );
    Row {
        expected: format!("{name}: refused\n    {message}\n"),
        kotlin: kotlin_call(name, read_as, payload),
    }
}

fn kotlin_call(name: &str, read_as: &str, payload: &str) -> String {
    format!(
        "row({}, serializer<{read_as}>(), {})",
        kotlin_string(name),
        kotlin_string(payload)
    )
}

/// `draft` written as serde writes it.
fn written<Draft>(draft: &Draft) -> String
where
    Draft: Serialize,
{
    serde_json::to_string(draft).unwrap()
}

fn rows() -> Vec<Row> {
    let text = || "a".to_owned();
    vec![
        row_of::<Split<String>>(
            "several slots, external, a text",
            "Split<String>",
            &written(&SplitDraft::Halves(text(), 1)),
        ),
        row_of::<Split<i64>>(
            "several slots, external, a number",
            "Split<Long>",
            &written(&SplitDraft::Halves(-7_i64, 1)),
        ),
        row_of::<Split<String>>(
            "a newtype over a tuple, external, a text",
            "Split<String>",
            &written(&SplitDraft::Paired((text(), 2))),
        ),
        row_of::<Split<i64>>(
            "a newtype over a tuple, external, a number",
            "Split<Long>",
            &written(&SplitDraft::Paired((-7_i64, 2))),
        ),
        refused_row_of::<Split<String>>(
            "several slots, external, too few",
            "Split<String>",
            r#"{"Halves":["a"]}"#,
            "invalid length 1, expected 2 at path: $.Halves",
        ),
        row_of::<AdjacentSplit<String>>(
            "several slots, adjacent, a text",
            "AdjacentSplit<String>",
            &written(&AdjacentSplitDraft::Halves(text(), 1)),
        ),
        row_of::<AdjacentSplit<i64>>(
            "several slots, adjacent, a number",
            "AdjacentSplit<Long>",
            &written(&AdjacentSplitDraft::Halves(-7_i64, 1)),
        ),
        row_of::<AdjacentSplit<String>>(
            "a newtype over a tuple, adjacent, a text",
            "AdjacentSplit<String>",
            &written(&AdjacentSplitDraft::Paired((text(), 2))),
        ),
        row_of::<AdjacentSplit<i64>>(
            "a newtype over a tuple, adjacent, a number",
            "AdjacentSplit<Long>",
            &written(&AdjacentSplitDraft::Paired((-7_i64, 2))),
        ),
        row_of::<Loose<String>>(
            "untagged, a tuple, a text",
            "Loose<String>",
            &written(&LooseDraft::Paired((text(), 2))),
        ),
        row_of::<Loose<i64>>(
            "untagged, a tuple, a number",
            "Loose<Long>",
            &written(&LooseDraft::Paired((-7_i64, 2))),
        ),
        row_of::<Loose<String>>(
            "untagged, a struct, a text",
            "Loose<String>",
            &written(&LooseDraft::Named { held: text() }),
        ),
        row_of::<Stack<String>>(
            "a nested tuple, a text",
            "Stack<String>",
            &written(&StackDraft {
                pair: (text(), 1),
                nested: ((text(), 2), 3),
            }),
        ),
        row_of::<Stack<i64>>(
            "a nested tuple, a number",
            "Stack<Long>",
            &written(&StackDraft {
                pair: (-7_i64, 1),
                nested: ((-8_i64, 2), 3),
            }),
        ),
        refused_row_of::<Stack<String>>(
            "a nested tuple, too short",
            "Stack<String>",
            r#"{"pair":["a",1],"nested":[["a"],3]}"#,
            "invalid length 1, expected 2 at path: $.nested[0]",
        ),
        row_of::<Pairing<String>>(
            "a tuple struct, a text",
            "Pairing<String>",
            &written(&PairingDraft(text(), 1)),
        ),
        row_of::<Pairing<i64>>(
            "a tuple struct, a number",
            "Pairing<Long>",
            &written(&PairingDraft(-7_i64, 1)),
        ),
    ]
}

/// The text of every type compiles with `-Werror`, and every payload reads and writes in Kotlin as
/// the serde read reads and writes it.
#[test]
fn a_tuple_that_reaches_an_enums_parameter_reads_and_writes_in_kotlin_as_serde_does() {
    let rows = rows();
    let expected: String = rows.iter().map(|row| row.expected.as_str()).collect();
    let mut main = String::from("fun main() {\n");
    for row in &rows {
        let _ = writeln!(main, "    {}", row.kotlin);
    }
    main.push_str("}\n");
    let source = [
        KOTLIN_IMPORTS.to_owned(),
        split_kotlin::kotlin_definition(),
        adjacent_split_kotlin::kotlin_definition(),
        loose_kotlin::kotlin_definition(),
        stack_kotlin::kotlin_definition(),
        pairing_kotlin::kotlin_definition(),
        HARNESS.to_owned(),
        main,
    ]
    .join("\n\n");
    let Some(printed) = ran_kotlin(&source) else {
        return;
    };
    assert_eq!(printed, expected);
}
