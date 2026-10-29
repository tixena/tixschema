//! An enum published under a name other than its Rust ident, plain or under each tagging and with
//! or without a parameter, read and written by the emitted Kotlin types compiled with `-Werror`
//! through both names, and held by a type declared above it and by one declared below. Each
//! payload the serde read reads prints what it prints.

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

const CALM: &str = r#"
fun calm(value: Temper): List<String> =
    if (value == Temper.Frantic) listOf("a mood is calm") else emptyList()
"#;

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

/// A holder that names each enum below by its Rust ident, declared above them.
#[model_schema(default_types(Leaf = String))]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Bookcase<Leaf> {
    pub act: Act,
    pub chapter: Chapter,
    pub cue: Cue<Leaf>,
    pub genre: Genre,
    pub mood: Mood,
    pub reel: Reel<Leaf>,
}

/// A plain enum renamed by `name`.
#[model_schema(name = "Category")]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum Genre {
    Poetry,
    Prose,
}

/// A plain enum renamed by `name`, which names a check.
#[model_schema(name = "Temper", check = ["calm"])]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum Mood {
    Calm,
    Frantic,
}

/// An enum renamed by `name` under a tag alone.
#[model_schema(name = "Excerpt")]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "kind")]
pub enum Chapter {
    Closing { line: String },
    Opening { line: String },
}

/// An enum renamed by `name` under a tag and a content key.
#[model_schema(name = "Scene")]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "kind", content = "held")]
pub enum Act {
    Lowered,
    Raised { line: String },
}

/// An enum renamed by `name` that declares a parameter, under no tag attribute.
#[model_schema(name = "Spool", default_types(Leaf = String))]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum Reel<Leaf> {
    Sealed(Leaf),
    Unrolled { leaf: Leaf },
}

/// An enum renamed by `name` that declares a parameter, untagged.
#[model_schema(name = "Prompt", default_types(Leaf = String))]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum Cue<Leaf> {
    Sealed(Leaf),
    Unrolled { leaf: Leaf },
}

/// A holder that names each enum above by its Rust ident, declared below them.
#[model_schema(default_types(Leaf = String))]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Archive<Leaf> {
    pub act: Act,
    pub chapter: Chapter,
    pub cue: Cue<Leaf>,
    pub genre: Genre,
    pub mood: Mood,
    pub reel: Reel<Leaf>,
}

struct Row {
    expected: String,
    kotlin: String,
}

fn calm(value: &Mood) -> Result<(), Vec<String>> {
    if matches!(value, Mood::Frantic) {
        return Err(vec!["a mood is calm".to_owned()]);
    }
    Ok(())
}

fn kotlin_string(text: &str) -> String {
    format!(
        "\"{}\"",
        text.replace('\\', "\\\\")
            .replace('"', "\\\"")
            .replace('$', "\\$")
    )
}

/// The payload serde writes for `held`, read as `read_as`, held to what the serde read prints.
fn read_row<Held>(name: &str, read_as: &str, held: &Held) -> Row
where
    Held: DeserializeOwned + Serialize,
{
    let payload = serde_json::to_string(held).unwrap();
    let expected = match serde_json::from_str::<Held>(&payload) {
        Ok(read) => format!("{name}: read {}\n", serde_json::to_string(&read).unwrap()),
        Err(refused) => {
            let message = refused.to_string();
            let lines = message
                .rsplit_once(" at line ")
                .map_or(message.as_str(), |(head, _)| head);
            let mut text = format!("{name}: refused\n");
            for line in lines.split("; ") {
                let _ = writeln!(text, "    {line}");
            }
            text
        }
    };
    Row {
        expected,
        kotlin: format!(
            "row({}, serializer<{read_as}>(), {})",
            kotlin_string(name),
            kotlin_string(&payload)
        ),
    }
}

/// [`read_row`] through the Rust ident, which a typealias republishes, and through the published
/// name.
fn both<Held>(name: &str, ident: &str, published: &str, held: &Held) -> [Row; 2]
where
    Held: DeserializeOwned + Serialize,
{
    [
        read_row(&format!("{name}, by its ident"), ident, held),
        read_row(&format!("{name}, by its published name"), published, held),
    ]
}

fn leaf(text: &str) -> String {
    text.to_owned()
}

fn rows() -> Vec<Row> {
    let line = || "a".to_owned();
    let mut rows: Vec<Row> = [
        both("a plain enum", "Genre", "Category", &Genre::Poetry),
        both("a plain enum with a check", "Mood", "Temper", &Mood::Calm),
        both(
            "a plain enum, its check refusing",
            "Mood",
            "Temper",
            &Mood::Frantic,
        ),
        both(
            "a tag alone",
            "Chapter",
            "Excerpt",
            &Chapter::Opening { line: line() },
        ),
        both("a tag and a content key", "Act", "Scene", &Act::Lowered),
        both(
            "a tag and a content key, a payload",
            "Act",
            "Scene",
            &Act::Raised { line: line() },
        ),
        both(
            "no tag attribute, a parameter",
            "Reel<String>",
            "Spool<String>",
            &Reel::Unrolled { leaf: leaf("b") },
        ),
        both(
            "no tag attribute, one slot",
            "Reel<String>",
            "Spool<String>",
            &Reel::Sealed(leaf("b")),
        ),
        both(
            "untagged, a parameter",
            "Cue<String>",
            "Prompt<String>",
            &Cue::Unrolled { leaf: leaf("b") },
        ),
    ]
    .into_iter()
    .flatten()
    .collect();
    rows.push(read_row(
        "a holder declared above",
        "Bookcase<String>",
        &Bookcase {
            act: Act::Lowered,
            chapter: Chapter::Closing { line: line() },
            cue: Cue::Sealed(leaf("c")),
            genre: Genre::Prose,
            mood: Mood::Calm,
            reel: Reel::Sealed(leaf("d")),
        },
    ));
    rows.push(read_row(
        "a holder declared below",
        "Archive<String>",
        &Archive {
            act: Act::Raised { line: line() },
            chapter: Chapter::Opening { line: line() },
            cue: Cue::Unrolled { leaf: leaf("c") },
            genre: Genre::Poetry,
            mood: Mood::Calm,
            reel: Reel::Unrolled { leaf: leaf("d") },
        },
    ));
    rows.push(read_row(
        "a holder declared below, its mood refused",
        "Archive<String>",
        &Archive {
            act: Act::Lowered,
            chapter: Chapter::Closing { line: line() },
            cue: Cue::Sealed(leaf("c")),
            genre: Genre::Prose,
            mood: Mood::Frantic,
            reel: Reel::Sealed(leaf("d")),
        },
    ));
    rows
}

/// The text of every enum compiles with `-Werror`, and every payload reads and writes in Kotlin
/// as the serde read reads and writes it, through the `typealias` that republishes the enum's
/// ident and through the name it publishes under.
#[test]
fn each_renamed_enum_reads_and_writes_in_kotlin_by_both_names() {
    let rows = rows();
    let expected: String = rows.iter().map(|row| row.expected.as_str()).collect();
    let mut main = String::from("fun main() {\n");
    for row in &rows {
        let _ = writeln!(main, "    {}", row.kotlin);
    }
    main.push_str("}\n");
    let source = [
        KOTLIN_IMPORTS.to_owned(),
        bookcase_kotlin::kotlin_definition(),
        genre_kotlin::kotlin_definition(),
        mood_kotlin::kotlin_definition(),
        chapter_kotlin::kotlin_definition(),
        act_kotlin::kotlin_definition(),
        reel_kotlin::kotlin_definition(),
        cue_kotlin::kotlin_definition(),
        archive_kotlin::kotlin_definition(),
        CALM.to_owned(),
        HARNESS.to_owned(),
        main,
    ]
    .join("\n\n");
    let Some(printed) = ran_kotlin(&source) else {
        return;
    };
    assert_eq!(printed, expected);
}
