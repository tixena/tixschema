//! A value of the wrong kind where the emitted Kotlin reads a text, a number or a list itself,
//! compiled with `-Werror` and run: each read throws a `SerializationException` that names the
//! member, and a value of the right kind reads as serde reads it. Serde refuses each payload of
//! the wrong kind too.

#![cfg(feature = "kotlin")]

use core::fmt::Write as _;

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tixschema::model_schema;

use super::runtime::ran_kotlin;

const KOTLIN_IMPORTS: &str = "import kotlinx.serialization.*\n\
     import kotlinx.serialization.json.*\n\
     import kotlinx.serialization.descriptors.*\n\
     import kotlinx.serialization.encoding.*\n\
     import kotlinx.serialization.builtins.*";

/// The function a member names, and what a row prints: the value written back, or what the read
/// threw, as a `SerializationException` or as anything else.
const HARNESS: &str = r#"
fun trimmed(payload: JsonElement): JsonElement =
    if (payload is JsonPrimitive && payload.isString) JsonPrimitive(payload.content.trim()) else payload

fun <T> row(name: String, serializer: KSerializer<T>, payload: String) {
    try {
        val read = Json.decodeFromString(serializer, payload)
        println("$name: read ${Json.encodeToString(serializer, read)}")
    } catch (refused: SerializationException) {
        println("$name: ${refused::class.simpleName}: ${(refused.message ?: "").lines().first()}")
    } catch (refused: Throwable) {
        println("$name: ${refused::class.simpleName}")
    }
}
"#;

/// What the Kotlin read prints of each row. A text where a number is read, and a number where a
/// text is, are the library's own refusals.
const ANSWERS: &str = "\
flat, right kinds: read {\"dock\":\"d\",\"count\":1,\"label\":\"A\",\"tags\":[\"t\"]}
flat, a list for a text: SerializationException: 'label': STRING cannot be read from an array
flat, an object for a text: SerializationException: 'label': STRING cannot be read from an object
flat, a list for a number: SerializationException: 'count': INT cannot be read from an array
flat, an object for a number: SerializationException: 'count': INT cannot be read from an object
flat, a text for a list: SerializationException: 'tags': LIST cannot be read from a primitive
flat, a number for a text: JsonDecodingException: String literal for value of key 'primitive' should be quoted at path: $.primitive
tally, right kinds: read [\"a\",1]
tally, a list for a text: SerializationException: '0': STRING cannot be read from an array
tally, an object for a number: SerializationException: '1': INT cannot be read from an object
haul, right kinds: read {\"kind\":\"Split\",\"held\":[\"a\",1]}
haul, a list for a text: SerializationException: '0': STRING cannot be read from an array
haul, an object for a number: SerializationException: '1': INT cannot be read from an object
haul, a list for the one slot: SerializationException: 'held': STRING cannot be read from an array
freight, right kinds: read {\"Split\":[\"a\",1]}
freight, a list for a text: SerializationException: '0': STRING cannot be read from an array
freight, an object for a number: SerializationException: '1': INT cannot be read from an object
freight, a list for the one slot: SerializationException: 'Sealed': STRING cannot be read from an array
pinned, right kinds: read {\"count\":1,\"label\":\"A\",\"tags\":[\"t\"]}
pinned, a list for a text: SerializationException: 'label': STRING cannot be read from an array
pinned, an object for a text: SerializationException: 'label': STRING cannot be read from an object
pinned, a list for a number: SerializationException: 'count': INT cannot be read from an array
pinned, a text for a list: SerializationException: 'tags': LIST cannot be read from a primitive
pinned, a number for a text: JsonDecodingException: String literal for value of key 'primitive' should be quoted at path: $.primitive
held, right kinds: read {\"pair\":[\"a\",1]}
held, a list for a text: SerializationException: '0': STRING cannot be read from an array
held, an object for a number: SerializationException: '1': INT cannot be read from an object
berth, a list for a text: JsonDecodingException: Unexpected JSON token at offset 8: Expected beginning of the string, but got [ at path: $.dock
";

const ROWS: [Row; 28] = [
    Row {
        name: "flat, right kinds",
        payload: r#"{"dock":"d","label":"A","count":1,"tags":["t"]}"#,
        serde: serde_written::<Flat>,
        typed: "Flat",
    },
    Row {
        name: "flat, a list for a text",
        payload: r#"{"dock":"d","label":["A"],"count":1,"tags":["t"]}"#,
        serde: serde_written::<Flat>,
        typed: "Flat",
    },
    Row {
        name: "flat, an object for a text",
        payload: r#"{"dock":"d","label":{},"count":1,"tags":["t"]}"#,
        serde: serde_written::<Flat>,
        typed: "Flat",
    },
    Row {
        name: "flat, a list for a number",
        payload: r#"{"dock":"d","label":"A","count":[1],"tags":["t"]}"#,
        serde: serde_written::<Flat>,
        typed: "Flat",
    },
    Row {
        name: "flat, an object for a number",
        payload: r#"{"dock":"d","label":"A","count":{},"tags":["t"]}"#,
        serde: serde_written::<Flat>,
        typed: "Flat",
    },
    Row {
        name: "flat, a text for a list",
        payload: r#"{"dock":"d","label":"A","count":1,"tags":"t"}"#,
        serde: serde_written::<Flat>,
        typed: "Flat",
    },
    Row {
        name: "flat, a number for a text",
        payload: r#"{"dock":"d","label":5,"count":1,"tags":["t"]}"#,
        serde: serde_written::<Flat>,
        typed: "Flat",
    },
    Row {
        name: "tally, right kinds",
        payload: r#"["a",1]"#,
        serde: serde_written::<Tally>,
        typed: "Tally",
    },
    Row {
        name: "tally, a list for a text",
        payload: r#"[["a"],1]"#,
        serde: serde_written::<Tally>,
        typed: "Tally",
    },
    Row {
        name: "tally, an object for a number",
        payload: r#"["a",{}]"#,
        serde: serde_written::<Tally>,
        typed: "Tally",
    },
    Row {
        name: "haul, right kinds",
        payload: r#"{"kind":"Split","held":["a",1]}"#,
        serde: serde_written::<Haul>,
        typed: "Haul",
    },
    Row {
        name: "haul, a list for a text",
        payload: r#"{"kind":"Split","held":[["a"],1]}"#,
        serde: serde_written::<Haul>,
        typed: "Haul",
    },
    Row {
        name: "haul, an object for a number",
        payload: r#"{"kind":"Split","held":["a",{}]}"#,
        serde: serde_written::<Haul>,
        typed: "Haul",
    },
    Row {
        name: "haul, a list for the one slot",
        payload: r#"{"kind":"Sealed","held":["a"]}"#,
        serde: serde_written::<Haul>,
        typed: "Haul",
    },
    Row {
        name: "freight, right kinds",
        payload: r#"{"Split":["a",1]}"#,
        serde: serde_written::<Freight>,
        typed: "Freight",
    },
    Row {
        name: "freight, a list for a text",
        payload: r#"{"Split":[["a"],1]}"#,
        serde: serde_written::<Freight>,
        typed: "Freight",
    },
    Row {
        name: "freight, an object for a number",
        payload: r#"{"Split":["a",{}]}"#,
        serde: serde_written::<Freight>,
        typed: "Freight",
    },
    Row {
        name: "freight, a list for the one slot",
        payload: r#"{"Sealed":["a"]}"#,
        serde: serde_written::<Freight>,
        typed: "Freight",
    },
    Row {
        name: "pinned, right kinds",
        payload: r#"{"label":"A","count":1,"tags":["t"]}"#,
        serde: serde_written::<Pinned>,
        typed: "Pinned",
    },
    Row {
        name: "pinned, a list for a text",
        payload: r#"{"label":["A"],"count":1,"tags":["t"]}"#,
        serde: serde_written::<Pinned>,
        typed: "Pinned",
    },
    Row {
        name: "pinned, an object for a text",
        payload: r#"{"label":{},"count":1,"tags":["t"]}"#,
        serde: serde_written::<Pinned>,
        typed: "Pinned",
    },
    Row {
        name: "pinned, a list for a number",
        payload: r#"{"label":"A","count":[1],"tags":["t"]}"#,
        serde: serde_written::<Pinned>,
        typed: "Pinned",
    },
    Row {
        name: "pinned, a text for a list",
        payload: r#"{"label":"A","count":1,"tags":"t"}"#,
        serde: serde_written::<Pinned>,
        typed: "Pinned",
    },
    Row {
        name: "pinned, a number for a text",
        payload: r#"{"label":5,"count":1,"tags":["t"]}"#,
        serde: serde_written::<Pinned>,
        typed: "Pinned",
    },
    Row {
        name: "held, right kinds",
        payload: r#"{"pair":["a",1]}"#,
        serde: serde_written::<Held>,
        typed: "Held",
    },
    Row {
        name: "held, a list for a text",
        payload: r#"{"pair":[["a"],1]}"#,
        serde: serde_written::<Held>,
        typed: "Held",
    },
    Row {
        name: "held, an object for a number",
        payload: r#"{"pair":["a",{}]}"#,
        serde: serde_written::<Held>,
        typed: "Held",
    },
    Row {
        name: "berth, a list for a text",
        payload: r#"{"dock":["d"]}"#,
        serde: serde_written::<Berth>,
        typed: "Berth",
    },
];

/// One payload: its name, the Kotlin type that reads it, and the serde read of the same type, which
/// answers what it writes back, or `None` where it refuses the payload.
struct Row {
    name: &'static str,
    payload: &'static str,
    serde: fn(&str) -> Option<String>,
    typed: &'static str,
}

#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Berth {
    pub dock: String,
}

#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Flat {
    #[serde(flatten)]
    pub berth: Berth,
    pub count: u32,
    pub label: String,
    pub tags: Vec<String>,
}

#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum Freight {
    Sealed(String),
    Split(String, u32),
}

#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "kind", content = "held")]
pub enum Haul {
    Sealed(String),
    Split(String, u32),
}

#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Held {
    pub pair: (String, u32),
}

#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Pinned {
    #[model_schema_prop(preprocess = ["trimmed"])]
    pub count: u32,
    #[model_schema_prop(preprocess = ["trimmed"])]
    pub label: String,
    #[model_schema_prop(preprocess = ["trimmed"])]
    pub tags: Vec<String>,
}

#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Tally(pub String, pub u32);

fn trimmed(payload: Value) -> Value {
    if let Value::String(text) = payload {
        Value::String(text.trim().to_owned())
    } else {
        payload
    }
}

fn kotlin_string(text: &str) -> String {
    format!(
        "\"{}\"",
        text.replace('\\', "\\\\")
            .replace('"', "\\\"")
            .replace('$', "\\$")
    )
}

/// What serde writes back of `payload` read as `T`, or `None` where it refuses it.
fn serde_written<T>(payload: &str) -> Option<String>
where
    T: DeserializeOwned + Serialize,
{
    let read: T = serde_json::from_str(payload).ok()?;
    serde_json::to_string(&read).ok()
}

/// A payload of the wrong kind is refused by the Kotlin read with a `SerializationException`
/// that names the member where the emitter reads the value itself, and by serde; a payload of
/// the right kind is written back as serde writes it.
#[test]
fn a_value_of_the_wrong_kind_is_refused_with_a_serialization_exception_naming_the_member() {
    let mut main = String::from("fun main() {\n");
    for row in &ROWS {
        let _ = writeln!(
            main,
            "    row({}, serializer<{}>(), {})",
            kotlin_string(row.name),
            row.typed,
            kotlin_string(row.payload)
        );
    }
    main.push_str("}\n");
    let source = [
        KOTLIN_IMPORTS.to_owned(),
        berth_kotlin::kotlin_definition(),
        flat_kotlin::kotlin_definition(),
        tally_kotlin::kotlin_definition(),
        haul_kotlin::kotlin_definition(),
        freight_kotlin::kotlin_definition(),
        pinned_kotlin::kotlin_definition(),
        held_kotlin::kotlin_definition(),
        HARNESS.to_owned(),
        main,
    ];
    let Some(printed) = ran_kotlin(&source.join("\n\n")) else {
        return;
    };
    assert_eq!(printed.trim_end(), ANSWERS.trim_end());
    for row in &ROWS {
        let written = (row.serde)(row.payload);
        if row.name.ends_with("right kinds") {
            let prefix = format!("{}: read ", row.name);
            let line = printed
                .lines()
                .find_map(|line| line.strip_prefix(prefix.as_str()))
                .unwrap_or_default();
            assert_eq!(Some(line), written.as_deref(), "{}", row.name);
        } else {
            assert_eq!(written, None, "serde reads {}", row.name);
        }
    }
}
