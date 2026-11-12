//! A map keyed by a brand over a text, a number and a date, and by a brand over the brand over a
//! text, each with a check, read by the emitted Kotlin types compiled with `-Werror`: a value
//! refused under a key, and a key a brand's check refuses, are refused at the field serde names,
//! read off the refusal as the `ws_rpc` attachment and client read it, and a map that reads is
//! written back as serde writes it.

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

/// The checks the brands name, written by hand as a consumer writes them.
const CHECKS: &str = r#"
fun audible(value: KeyedTone): List<String> =
    if (value.value == "mute") listOf("mute is silent") else emptyList()

fun inScale(value: KeyedPitch): List<String> =
    if (value.value <= 12u) emptyList() else listOf("${value.value} is outside of the scale")

fun loud(value: OuterTone): List<String> =
    if (value.value.value == "soft") listOf("soft is quiet") else emptyList()
"#;

/// The check the brand over a date names.
#[cfg(feature = "chrono")]
const DAY_CHECK: &str = r#"
fun workday(value: KeyedDay): List<String> =
    if (value.value == "2024-01-06") listOf("2024-01-06 is a day of rest") else emptyList()
"#;

/// What prints a row: the value read and written back, or the field its refusal names, the steps
/// of the path it states and the place its message opens with; and what prints a refusal whole.
const TOLD: &str = r#"
fun <T> told(at: String, serializer: KSerializer<T>, payload: String) {
    val printed = try {
        "read ${Json.encodeToString(serializer, Json.decodeFromString(serializer, payload))}"
    } catch (refused: SerializationException) {
        val said = refused.message.orEmpty()
        val path = Regex(""" at path: \$\.([^\n]*)""").find(said)?.groupValues?.get(1)
        val place = Regex("""^'([^']*)': """).find(said)?.groupValues?.get(1)
        val opening = place?.takeIf { path != it && path?.endsWith(".$it") != true }
        val field = listOfNotNull(path, opening).joinToString(".")
        "refused at ${field.ifEmpty { "no path" }}"
    }
    println("$at: $printed")
}

fun <T> said(at: String, serializer: KSerializer<T>, payload: String) {
    val printed = try {
        Json.decodeFromString(serializer, payload)
        "read"
    } catch (refused: SerializationException) {
        refused.message.orEmpty()
    }
    println("$at said: $printed")
}
"#;

/// A brand over a text a map is keyed by.
#[model_schema(check = ["audible"])]
#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
pub struct KeyedTone(String);

/// A brand over the brand over a text a map is keyed by.
#[model_schema(check = ["loud"])]
#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
pub struct OuterTone(KeyedTone);

/// A brand over a number a map is keyed by.
#[model_schema(check = ["inScale"])]
#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
pub struct KeyedPitch(u32);

/// A brand over a date a map is keyed by.
#[cfg(feature = "chrono")]
#[model_schema(check = ["workday"])]
#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
pub struct KeyedDay(chrono::NaiveDate);

/// A map keyed by the brand over a number, and one keyed by the brand over a text.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Keyring {
    pub pitches: BTreeMap<KeyedPitch, u32>,
    pub tones: BTreeMap<KeyedTone, u32>,
}

/// A map keyed by the brand over the brand over a text.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Nested {
    pub outer: BTreeMap<OuterTone, u32>,
}

/// A map keyed by the brand over a date.
#[cfg(feature = "chrono")]
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Calendar {
    pub days: BTreeMap<KeyedDay, u32>,
}

/// One payload, the type it is read as, what serde answers of it, and the whole refusal the Kotlin
/// read throws where the row holds one.
struct Row {
    json: &'static str,
    named: &'static str,
    said: Option<&'static str>,
    serde: String,
}

/// The check of [`KeyedTone`]: a tone that sounds.
fn audible(value: &KeyedTone) -> Result<(), Vec<String>> {
    if value.as_inner() == "mute" {
        Err(vec!["mute is silent".to_owned()])
    } else {
        Ok(())
    }
}

/// The check of [`OuterTone`]: a tone louder than soft.
fn loud(value: &OuterTone) -> Result<(), Vec<String>> {
    if value.as_inner().as_inner() == "soft" {
        Err(vec!["soft is quiet".to_owned()])
    } else {
        Ok(())
    }
}

/// The check of [`KeyedPitch`]: a pitch within the twelve tones of the scale.
fn in_scale(value: &KeyedPitch) -> Result<(), Vec<String>> {
    if *value.as_inner() <= 12 {
        Ok(())
    } else {
        Err(vec![format!(
            "{} is outside of the scale",
            value.as_inner()
        )])
    }
}

/// The check of [`KeyedDay`]: a day that is not the one day of rest.
#[cfg(feature = "chrono")]
fn workday(value: &KeyedDay) -> Result<(), Vec<String>> {
    if *value.as_inner() == chrono::NaiveDate::from_ymd_opt(2024, 1, 6).unwrap() {
        Err(vec!["2024-01-06 is a day of rest".to_owned()])
    } else {
        Ok(())
    }
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
        said: None,
        serde: serde_told::<Held>(json),
    }
}

/// [`row`] of a key the brand's check refuses, which the Kotlin read refuses with `said`.
fn refused_key<Held>(named: &'static str, json: &'static str, said: &'static str) -> Row
where
    Held: DeserializeOwned + Serialize,
{
    Row {
        said: Some(said),
        ..row::<Held>(named, json)
    }
}

/// Each payload: the maps read, a negative number under a number key and under a text key, a text
/// under a text key, a key each check refuses and one it reads, the same under a key of the brand
/// over the brand over a text, refused by either brand's check, and under the `chrono` feature
/// the same under a date key.
fn rows() -> Vec<Row> {
    let mut rows = vec![
        row::<Keyring>("Keyring", r#"{"pitches":{"3":1},"tones":{"low":2}}"#),
        row::<Keyring>("Keyring", r#"{"pitches":{"3":-1},"tones":{"low":2}}"#),
        row::<Keyring>("Keyring", r#"{"pitches":{"3":1},"tones":{"low":-1}}"#),
        row::<Keyring>("Keyring", r#"{"pitches":{"3":1},"tones":{"low":"x"}}"#),
        refused_key::<Keyring>(
            "Keyring",
            r#"{"pitches":{"3":1},"tones":{"mute":1}}"#,
            "'mute': mute is silent at path: $.tones",
        ),
        row::<Keyring>("Keyring", r#"{"pitches":{"3":1},"tones":{"ring":1}}"#),
        refused_key::<Keyring>(
            "Keyring",
            r#"{"pitches":{"13":1},"tones":{"low":2}}"#,
            "'13': 13 is outside of the scale at path: $.pitches",
        ),
        row::<Nested>("Nested", r#"{"outer":{"ring":1}}"#),
        row::<Nested>("Nested", r#"{"outer":{"ring":-1}}"#),
        refused_key::<Nested>(
            "Nested",
            r#"{"outer":{"soft":1}}"#,
            "'soft': soft is quiet at path: $.outer",
        ),
        refused_key::<Nested>(
            "Nested",
            r#"{"outer":{"mute":1}}"#,
            "'mute': mute is silent at path: $.outer",
        ),
    ];
    #[cfg(feature = "chrono")]
    rows.extend([
        row::<Calendar>("Calendar", r#"{"days":{"2024-01-02":1}}"#),
        row::<Calendar>("Calendar", r#"{"days":{"2024-01-02":-1}}"#),
        refused_key::<Calendar>(
            "Calendar",
            r#"{"days":{"2024-01-06":1}}"#,
            "'2024-01-06': 2024-01-06 is a day of rest at path: $.days",
        ),
    ]);
    rows
}

#[test]
fn a_value_and_a_key_under_a_brand_key_are_refused_at_the_field_serde_names() {
    let rows = rows();
    let mut serde_said: Vec<&str> = vec![
        r#"read {"pitches":{"3":1},"tones":{"low":2}}"#,
        "refused at pitches.3",
        "refused at tones.low",
        "refused at tones.low",
        "refused at tones.mute",
        r#"read {"pitches":{"3":1},"tones":{"ring":1}}"#,
        "refused at pitches.13",
        r#"read {"outer":{"ring":1}}"#,
        "refused at outer.ring",
        "refused at outer.soft",
        "refused at outer.mute",
    ];
    #[cfg(feature = "chrono")]
    serde_said.extend([
        r#"read {"days":{"2024-01-02":1}}"#,
        "refused at days.2024-01-02",
        "refused at days.2024-01-06",
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
        if let Some(said) = row.said {
            let _ = writeln!(
                main,
                "    said(\"{at}\", serializer<{}>(), \"\"\"{}\"\"\")",
                row.named, row.json
            );
            let _ = writeln!(expected, "{at} said: {said}");
        }
    }
    main.push_str("}\n");
    let mut source = vec![
        KOTLIN_IMPORTS.to_owned(),
        keyed_tone_kotlin::kotlin_definition(),
        keyed_pitch_kotlin::kotlin_definition(),
        keyring_kotlin::kotlin_definition(),
        outer_tone_kotlin::kotlin_definition(),
        nested_kotlin::kotlin_definition(),
        CHECKS.to_owned(),
    ];
    #[cfg(feature = "chrono")]
    source.extend([
        keyed_day_kotlin::kotlin_definition(),
        calendar_kotlin::kotlin_definition(),
        DAY_CHECK.to_owned(),
    ]);
    source.extend([TOLD.to_owned(), main]);
    let Some(printed) = ran_kotlin(&source.join("\n\n")) else {
        return;
    };
    assert_eq!(printed, expected, "the Kotlin read is not serde's");
}
