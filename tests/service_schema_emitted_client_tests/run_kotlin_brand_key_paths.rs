//! A map keyed by a brand over a text, a number and a date, read by the emitted Kotlin types
//! compiled with `-Werror`: a value refused under a key is refused at the path serde names, the
//! key's text among its steps, and a map that reads is written back as serde writes it.

#![cfg(feature = "kotlin")]

use alloc::collections::BTreeMap;
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

/// What prints a row: the value read and written back, or the path its refusal states.
const TOLD: &str = r#"
fun <T> told(at: String, serializer: KSerializer<T>, payload: String) {
    val printed = try {
        "read ${Json.encodeToString(serializer, Json.decodeFromString(serializer, payload))}"
    } catch (refused: SerializationException) {
        val path = Regex(""" at path: \$\.([^\n]*)""").find(refused.message.orEmpty())?.groupValues?.get(1)
        "refused at ${path ?: "no path"}"
    }
    println("$at: $printed")
}
"#;

/// A brand over a text a map is keyed by.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
pub struct KeyedTone(pub String);

/// A brand over a number a map is keyed by.
#[model_schema()]
#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
pub struct KeyedPitch(pub u32);

/// A brand over a date a map is keyed by.
#[cfg(feature = "chrono")]
#[model_schema()]
#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
pub struct KeyedDay(pub chrono::NaiveDate);

/// A map keyed by the brand over a number, and one keyed by the brand over a text.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Keyring {
    pub pitches: BTreeMap<KeyedPitch, u32>,
    pub tones: BTreeMap<KeyedTone, u32>,
}

/// A map keyed by the brand over a date.
#[cfg(feature = "chrono")]
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Calendar {
    pub days: BTreeMap<KeyedDay, u32>,
}

/// One payload, the type it is read as, and what serde answers of it.
struct Row {
    json: &'static str,
    named: &'static str,
    serde: String,
}

/// What serde answers of `json` read as `Held`: the value it reads, written back, or the path its
/// refusal names.
fn serde_told<Held>(json: &str) -> String
where
    Held: DeserializeOwned + Serialize,
{
    let mut read = serde_json::Deserializer::from_str(json);
    serde_path_to_error::deserialize::<_, Held>(&mut read).map_or_else(
        |refused| format!("refused at {}", refused.path()),
        |held| format!("read {}", serde_json::to_string(&held).unwrap()),
    )
}

fn row<Held>(named: &'static str, json: &'static str) -> Row
where
    Held: DeserializeOwned + Serialize,
{
    Row {
        json,
        named,
        serde: serde_told::<Held>(json),
    }
}

/// Each payload: the maps read, a negative number under a number key and under a text key, a text
/// under a text key, and under the `chrono` feature the same under a date key.
fn rows() -> Vec<Row> {
    let mut rows = vec![
        row::<Keyring>("Keyring", r#"{"pitches":{"3":1},"tones":{"low":2}}"#),
        row::<Keyring>("Keyring", r#"{"pitches":{"3":-1},"tones":{"low":2}}"#),
        row::<Keyring>("Keyring", r#"{"pitches":{"3":1},"tones":{"low":-1}}"#),
        row::<Keyring>("Keyring", r#"{"pitches":{"3":1},"tones":{"low":"x"}}"#),
    ];
    #[cfg(feature = "chrono")]
    rows.extend([
        row::<Calendar>("Calendar", r#"{"days":{"2024-01-02":1}}"#),
        row::<Calendar>("Calendar", r#"{"days":{"2024-01-02":-1}}"#),
    ]);
    rows
}

#[test]
fn a_value_under_a_brand_key_is_refused_at_the_path_serde_names() {
    let rows = rows();
    let mut serde_said: Vec<&str> = vec![
        r#"read {"pitches":{"3":1},"tones":{"low":2}}"#,
        "refused at pitches.3",
        "refused at tones.low",
        "refused at tones.low",
    ];
    #[cfg(feature = "chrono")]
    serde_said.extend([
        r#"read {"days":{"2024-01-02":1}}"#,
        "refused at days.2024-01-02",
    ]);
    assert_eq!(
        rows.iter()
            .map(|row| row.serde.as_str())
            .collect::<Vec<_>>(),
        serde_said,
        "serde no longer answers each payload as it did"
    );
    let mut main = String::from("fun main() {\n");
    let mut expected = String::new();
    for (at, row) in rows.iter().enumerate() {
        let _ = writeln!(
            main,
            "    told(\"{at}\", serializer<{}>(), \"\"\"{}\"\"\")",
            row.named, row.json
        );
        let _ = writeln!(expected, "{at}: {}", row.serde);
    }
    main.push_str("}\n");
    let mut source = vec![
        KOTLIN_IMPORTS.to_owned(),
        keyed_tone_kotlin::kotlin_definition(),
        keyed_pitch_kotlin::kotlin_definition(),
        keyring_kotlin::kotlin_definition(),
    ];
    #[cfg(feature = "chrono")]
    source.extend([
        keyed_day_kotlin::kotlin_definition(),
        calendar_kotlin::kotlin_definition(),
    ]);
    source.extend([TOLD.to_owned(), main]);
    let Some(printed) = ran_kotlin(&source.join("\n\n")) else {
        return;
    };
    assert_eq!(printed, expected, "the Kotlin read is not serde's");
}
