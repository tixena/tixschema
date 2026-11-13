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

/// [`read_row_of`] over the valid type `$held` makes.
macro_rules! read_row {
    ($name:expr, $read_as:expr, $held:expr $(,)?) => {{
        let held = $held;
        let valid = held.clone().validate();
        read_row_of($name, $read_as, held, &valid)
    }};
}

/// [`read_row`] through the Rust ident, which a typealias republishes, and through the published
/// name.
macro_rules! both {
    ($name:expr, $ident:expr, $published:expr, $held:expr $(,)?) => {{
        let held = $held;
        [
            read_row!(&format!("{}, by its ident", $name), $ident, held),
            read_row!(
                &format!("{}, by its published name", $name),
                $published,
                held
            ),
        ]
    }};
}

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

fn calm(value: &MoodDraft) -> Result<(), Vec<String>> {
    if matches!(value, MoodDraft::Frantic) {
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
fn read_row_of<Draft, Held>(
    name: &str,
    read_as: &str,
    held: &Draft,
    _valid: &Result<Held, Vec<String>>,
) -> Row
where
    Draft: Serialize,
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

fn leaf(text: &str) -> String {
    text.to_owned()
}

fn rows() -> Vec<Row> {
    let line = || "a".to_owned();
    let mut rows: Vec<Row> = [
        both!("a plain enum", "Genre", "Category", &GenreDraft::Poetry),
        both!(
            "a plain enum with a check",
            "Mood",
            "Temper",
            &MoodDraft::Calm,
        ),
        both!(
            "a plain enum, its check refusing",
            "Mood",
            "Temper",
            &MoodDraft::Frantic,
        ),
        both!(
            "a tag alone",
            "Chapter",
            "Excerpt",
            &ChapterDraft::Opening { line: line() },
        ),
        both!(
            "a tag and a content key",
            "Act",
            "Scene",
            &ActDraft::Lowered,
        ),
        both!(
            "a tag and a content key, a payload",
            "Act",
            "Scene",
            &ActDraft::Raised { line: line() },
        ),
        both!(
            "no tag attribute, a parameter",
            "Reel<String>",
            "Spool<String>",
            &ReelDraft::Unrolled { leaf: leaf("b") },
        ),
        both!(
            "no tag attribute, one slot",
            "Reel<String>",
            "Spool<String>",
            &ReelDraft::Sealed(leaf("b")),
        ),
        both!(
            "untagged, a parameter",
            "Cue<String>",
            "Prompt<String>",
            &CueDraft::Unrolled { leaf: leaf("b") },
        ),
    ]
    .into_iter()
    .flatten()
    .collect();
    rows.push(read_row!(
        "a holder declared above",
        "Bookcase<String>",
        &BookcaseDraft {
            act: ActDraft::Lowered.validate().unwrap(),
            chapter: ChapterDraft::Closing { line: line() }.validate().unwrap(),
            cue: CueDraft::Sealed(leaf("c")).validate().unwrap(),
            genre: GenreDraft::Prose.validate().unwrap(),
            mood: MoodDraft::Calm.validate().unwrap(),
            reel: ReelDraft::Sealed(leaf("d")).validate().unwrap(),
        },
    ));
    rows.push(read_row!(
        "a holder declared below",
        "Archive<String>",
        &ArchiveDraft {
            act: ActDraft::Raised { line: line() }.validate().unwrap(),
            chapter: ChapterDraft::Opening { line: line() }.validate().unwrap(),
            cue: CueDraft::Unrolled { leaf: leaf("c") }.validate().unwrap(),
            genre: GenreDraft::Poetry.validate().unwrap(),
            mood: MoodDraft::Calm.validate().unwrap(),
            reel: ReelDraft::Unrolled { leaf: leaf("d") }.validate().unwrap(),
        },
    ));
    rows.push(read_row!(
        "a holder declared below, its mood refused",
        "Archive<String>",
        &ArchiveDraft {
            act: ActDraft::Lowered.validate().unwrap(),
            chapter: ChapterDraft::Closing { line: line() }.validate().unwrap(),
            cue: CueDraft::Sealed(leaf("c")).validate().unwrap(),
            genre: GenreDraft::Prose.validate().unwrap(),
            mood: Mood::model_schema_wrap_unjudged(MoodDraft::Frantic),
            reel: ReelDraft::Sealed(leaf("d")).validate().unwrap(),
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
