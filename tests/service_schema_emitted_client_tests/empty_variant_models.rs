//! Enums with a braced variant serde reads no member of, `Off`, whose every member serde neither
//! writes nor reads, beside a variant of one member, `On`, under each tagging and untagged, each
//! strict and `nonstrict`. A strict one refuses a key no member declares in either variant, and a
//! `nonstrict` one reads past it, on every surface. Each `Hidden` one holds, in place of `Off`, a
//! variant serde neither writes nor reads, `Closed`, which every surface refuses. `Off {}` itself
//! is refused in this package by `clippy::empty_enum_variants_with_brackets`, and takes the same
//! read.

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tixschema::model_schema;

/// The payloads of an externally tagged enum.
const EXTERNAL: Payloads = &[
    (r#"{"Off":{}}"#, r#"{"Off":{}}"#),
    (r#"{"On":{"level":1}}"#, r#"{"On":{"level":1}}"#),
    (r#"{"Off":{"stray":1}}"#, r#"{"Off":{}}"#),
    (r#"{"On":{"level":1,"stray":1}}"#, r#"{"On":{"level":1}}"#),
];

/// The payloads of an enum under the tag `kind` alone.
const INTERNAL: Payloads = &[
    (r#"{"kind":"Off"}"#, r#"{"kind":"Off"}"#),
    (r#"{"kind":"On","level":1}"#, r#"{"kind":"On","level":1}"#),
    (r#"{"kind":"Off","stray":1}"#, r#"{"kind":"Off"}"#),
    (
        r#"{"kind":"On","level":1,"stray":1}"#,
        r#"{"kind":"On","level":1}"#,
    ),
];

/// The payloads of an enum under the tag `kind` and the content key `body`.
const ADJACENT: Payloads = &[
    (r#"{"kind":"Off","body":{}}"#, r#"{"kind":"Off","body":{}}"#),
    (
        r#"{"kind":"On","body":{"level":1}}"#,
        r#"{"kind":"On","body":{"level":1}}"#,
    ),
    (
        r#"{"kind":"Off","body":{"stray":1}}"#,
        r#"{"kind":"Off","body":{}}"#,
    ),
    (
        r#"{"kind":"On","body":{"level":1,"stray":1}}"#,
        r#"{"kind":"On","body":{"level":1}}"#,
    ),
    (
        r#"{"kind":"Off","body":{},"stray":1}"#,
        r#"{"kind":"Off","body":{}}"#,
    ),
];

/// The payloads of an untagged enum.
const UNTAGGED: Payloads = &[
    ("{}", "{}"),
    (r#"{"level":1}"#, r#"{"level":1}"#),
    (r#"{"stray":1}"#, "{}"),
    (r#"{"level":1,"stray":1}"#, r#"{"level":1}"#),
];

/// The payloads of an externally tagged enum whose `Closed` serde neither writes nor reads.
const HIDDEN_EXTERNAL: Payloads = &[
    (r#"{"On":{"level":1}}"#, r#"{"On":{"level":1}}"#),
    (r#"{"On":{"level":1,"stray":1}}"#, r#"{"On":{"level":1}}"#),
    (r#"{"Closed":{}}"#, REFUSED),
];

/// [`HIDDEN_EXTERNAL`] under the tag `kind` alone.
const HIDDEN_INTERNAL: Payloads = &[
    (r#"{"kind":"On","level":1}"#, r#"{"kind":"On","level":1}"#),
    (
        r#"{"kind":"On","level":1,"stray":1}"#,
        r#"{"kind":"On","level":1}"#,
    ),
    (r#"{"kind":"Closed"}"#, REFUSED),
];

/// [`HIDDEN_EXTERNAL`] under the tag `kind` and the content key `body`.
const HIDDEN_ADJACENT: Payloads = &[
    (
        r#"{"kind":"On","body":{"level":1}}"#,
        r#"{"kind":"On","body":{"level":1}}"#,
    ),
    (
        r#"{"kind":"On","body":{"level":1,"stray":1}}"#,
        r#"{"kind":"On","body":{"level":1}}"#,
    ),
    (
        r#"{"kind":"On","body":{"level":1},"stray":1}"#,
        r#"{"kind":"On","body":{"level":1}}"#,
    ),
    (r#"{"kind":"Closed","body":{}}"#, REFUSED),
];

/// [`HIDDEN_EXTERNAL`] untagged.
const HIDDEN_UNTAGGED: Payloads = &[
    (r#"{"level":1}"#, r#"{"level":1}"#),
    (r#"{"level":1,"stray":1}"#, r#"{"level":1}"#),
    ("{}", REFUSED),
];

/// What a payload no type reads is answered with, strict or not.
const REFUSED: &str = "refused";

/// Each type's name, its payloads, and whether it is strict.
pub const TYPES: [(&str, Payloads, bool); 16] = [
    ("Signal", EXTERNAL, true),
    ("OpenSignal", EXTERNAL, false),
    ("TaggedSignal", INTERNAL, true),
    ("OpenTaggedSignal", INTERNAL, false),
    ("AdjacentSignal", ADJACENT, true),
    ("OpenAdjacentSignal", ADJACENT, false),
    ("UntaggedSignal", UNTAGGED, true),
    ("OpenUntaggedSignal", UNTAGGED, false),
    ("HiddenSignal", HIDDEN_EXTERNAL, true),
    ("OpenHiddenSignal", HIDDEN_EXTERNAL, false),
    ("HiddenTaggedSignal", HIDDEN_INTERNAL, true),
    ("OpenHiddenTaggedSignal", HIDDEN_INTERNAL, false),
    ("HiddenAdjacentSignal", HIDDEN_ADJACENT, true),
    ("OpenHiddenAdjacentSignal", HIDDEN_ADJACENT, false),
    ("HiddenUntaggedSignal", HIDDEN_UNTAGGED, true),
    ("OpenHiddenUntaggedSignal", HIDDEN_UNTAGGED, false),
];

/// Payloads of one tagging, each with what serde writes back of it.
type Payloads = &'static [(&'static str, &'static str)];

/// An externally tagged enum whose `Off` holds a member serde neither writes nor reads.
#[model_schema(enable = ["jsonschema"])]
#[derive(Debug, Deserialize, Serialize)]
pub enum Signal {
    Off {
        #[serde(skip)]
        since: u32,
    },
    On {
        level: u8,
    },
}

/// [`Signal`], reading past a key no member declares.
#[model_schema(enable = ["jsonschema"], nonstrict)]
#[derive(Debug, Deserialize, Serialize)]
pub enum OpenSignal {
    Off {
        #[serde(skip)]
        since: u32,
    },
    On {
        level: u8,
    },
}

/// [`Signal`] under a tag alone.
#[model_schema(enable = ["jsonschema"])]
#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "kind")]
pub enum TaggedSignal {
    Off {
        #[serde(skip)]
        since: u32,
    },
    On {
        level: u8,
    },
}

/// [`TaggedSignal`], reading past a key no member declares.
#[model_schema(enable = ["jsonschema"], nonstrict)]
#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "kind")]
pub enum OpenTaggedSignal {
    Off {
        #[serde(skip)]
        since: u32,
    },
    On {
        level: u8,
    },
}

/// [`Signal`] under a tag and a content key, its `Off` holding two members serde skips.
#[model_schema(enable = ["jsonschema"])]
#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "kind", content = "body")]
pub enum AdjacentSignal {
    Off {
        #[serde(skip)]
        reason: String,
        #[serde(skip)]
        since: u32,
    },
    On {
        level: u8,
    },
}

/// [`AdjacentSignal`], reading past a key no member declares.
#[model_schema(enable = ["jsonschema"], nonstrict)]
#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "kind", content = "body")]
pub enum OpenAdjacentSignal {
    Off {
        #[serde(skip)]
        reason: String,
        #[serde(skip)]
        since: u32,
    },
    On {
        level: u8,
    },
}

/// [`Signal`] untagged, the variant of no member last: a `nonstrict` one reads every object.
#[model_schema(enable = ["jsonschema"])]
#[derive(Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum UntaggedSignal {
    Lit {
        level: u8,
    },
    Unlit {
        #[serde(skip)]
        since: u32,
    },
}

/// [`UntaggedSignal`], reading past a key no member declares.
#[model_schema(enable = ["jsonschema"], nonstrict)]
#[derive(Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum OpenUntaggedSignal {
    Lit {
        level: u8,
    },
    Unlit {
        #[serde(skip)]
        since: u32,
    },
}

/// [`Signal`] with `Closed`, a variant serde neither writes nor reads, in place of `Off`.
#[model_schema(enable = ["jsonschema"])]
#[derive(Debug, Deserialize, Serialize)]
pub enum HiddenSignal {
    #[serde(skip)]
    Closed {
        #[serde(skip)]
        held: u32,
    },
    On {
        level: u8,
    },
}

/// [`HiddenSignal`], reading past a key no member declares.
#[model_schema(enable = ["jsonschema"], nonstrict)]
#[derive(Debug, Deserialize, Serialize)]
pub enum OpenHiddenSignal {
    #[serde(skip)]
    Closed {
        #[serde(skip)]
        held: u32,
    },
    On {
        level: u8,
    },
}

/// [`HiddenSignal`] under a tag alone.
#[model_schema(enable = ["jsonschema"])]
#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "kind")]
pub enum HiddenTaggedSignal {
    #[serde(skip)]
    Closed {
        #[serde(skip)]
        held: u32,
    },
    On {
        level: u8,
    },
}

/// [`HiddenTaggedSignal`], reading past a key no member declares.
#[model_schema(enable = ["jsonschema"], nonstrict)]
#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "kind")]
pub enum OpenHiddenTaggedSignal {
    #[serde(skip)]
    Closed {
        #[serde(skip)]
        held: u32,
    },
    On {
        level: u8,
    },
}

/// [`HiddenSignal`] under a tag and a content key.
#[model_schema(enable = ["jsonschema"])]
#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "kind", content = "body")]
pub enum HiddenAdjacentSignal {
    #[serde(skip)]
    Closed {
        #[serde(skip)]
        held: u32,
    },
    On {
        level: u8,
    },
}

/// [`HiddenAdjacentSignal`], reading past a key no member declares.
#[model_schema(enable = ["jsonschema"], nonstrict)]
#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "kind", content = "body")]
pub enum OpenHiddenAdjacentSignal {
    #[serde(skip)]
    Closed {
        #[serde(skip)]
        held: u32,
    },
    On {
        level: u8,
    },
}

/// [`HiddenSignal`] untagged.
#[model_schema(enable = ["jsonschema"])]
#[derive(Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum HiddenUntaggedSignal {
    #[serde(skip)]
    Closed {
        #[serde(skip)]
        held: u32,
    },
    On {
        level: u8,
    },
}

/// [`HiddenUntaggedSignal`], reading past a key no member declares.
#[model_schema(enable = ["jsonschema"], nonstrict)]
#[derive(Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum OpenHiddenUntaggedSignal {
    #[serde(skip)]
    Closed {
        #[serde(skip)]
        held: u32,
    },
    On {
        level: u8,
    },
}

/// Every row a surface is handed: each type with each payload of its tagging.
pub fn rows() -> Vec<(&'static str, &'static str)> {
    TYPES
        .iter()
        .flat_map(|(name, payloads, _)| payloads.iter().map(|(payload, _)| (*name, *payload)))
        .collect()
}

/// What every surface must answer of each row: the object serde writes back where it reads it,
/// `refused` where it does not. A strict type refuses a stray key, and a `nonstrict` one reads
/// past it.
pub fn expected() -> Vec<Value> {
    TYPES
        .iter()
        .flat_map(|(_, payloads, strict)| {
            payloads.iter().map(|(payload, written)| {
                if *written == REFUSED || (*strict && payload != written) {
                    Value::String(REFUSED.to_owned())
                } else {
                    serde_json::from_str(written).unwrap()
                }
            })
        })
        .collect()
}

/// Each line of `printed` after its position, its object read whatever the order of its keys.
pub fn objects(printed: &str) -> Vec<Value> {
    printed
        .lines()
        .enumerate()
        .map(|(at, line)| {
            let (position, written) = line.split_once(": ").unwrap();
            assert_eq!(position, at.to_string(), "{printed}");
            serde_json::from_str(written).unwrap_or_else(|_| Value::String(written.to_owned()))
        })
        .collect()
}

/// What serde writes back of `json` read as `T`, or `refused`.
fn serde_told<T>(json: &str) -> Value
where
    T: DeserializeOwned + Serialize,
{
    serde_json::from_str::<T>(json).map_or_else(
        |_| Value::String("refused".to_owned()),
        |read| serde_json::to_value(read).unwrap(),
    )
}

/// The JSON Schema of the type `name` names.
fn json_schema_of(name: &str) -> Value {
    match name {
        "Signal" => Signal::json_schema(),
        "OpenSignal" => OpenSignal::json_schema(),
        "TaggedSignal" => TaggedSignal::json_schema(),
        "OpenTaggedSignal" => OpenTaggedSignal::json_schema(),
        "AdjacentSignal" => AdjacentSignal::json_schema(),
        "OpenAdjacentSignal" => OpenAdjacentSignal::json_schema(),
        "UntaggedSignal" => UntaggedSignal::json_schema(),
        "OpenUntaggedSignal" => OpenUntaggedSignal::json_schema(),
        "HiddenSignal" => HiddenSignal::json_schema(),
        "OpenHiddenSignal" => OpenHiddenSignal::json_schema(),
        "HiddenTaggedSignal" => HiddenTaggedSignal::json_schema(),
        "OpenHiddenTaggedSignal" => OpenHiddenTaggedSignal::json_schema(),
        "HiddenAdjacentSignal" => HiddenAdjacentSignal::json_schema(),
        "OpenHiddenAdjacentSignal" => OpenHiddenAdjacentSignal::json_schema(),
        "HiddenUntaggedSignal" => HiddenUntaggedSignal::json_schema(),
        _ => OpenHiddenUntaggedSignal::json_schema(),
    }
}

/// The TypeScript type and the Zod schema of every type.
pub fn ts_definitions() -> Vec<String> {
    vec![
        Signal::ts_definition(),
        Signal::zod_schema(),
        OpenSignal::ts_definition(),
        OpenSignal::zod_schema(),
        TaggedSignal::ts_definition(),
        TaggedSignal::zod_schema(),
        OpenTaggedSignal::ts_definition(),
        OpenTaggedSignal::zod_schema(),
        AdjacentSignal::ts_definition(),
        AdjacentSignal::zod_schema(),
        OpenAdjacentSignal::ts_definition(),
        OpenAdjacentSignal::zod_schema(),
        UntaggedSignal::ts_definition(),
        UntaggedSignal::zod_schema(),
        OpenUntaggedSignal::ts_definition(),
        OpenUntaggedSignal::zod_schema(),
        HiddenSignal::ts_definition(),
        HiddenSignal::zod_schema(),
        OpenHiddenSignal::ts_definition(),
        OpenHiddenSignal::zod_schema(),
        HiddenTaggedSignal::ts_definition(),
        HiddenTaggedSignal::zod_schema(),
        OpenHiddenTaggedSignal::ts_definition(),
        OpenHiddenTaggedSignal::zod_schema(),
        HiddenAdjacentSignal::ts_definition(),
        HiddenAdjacentSignal::zod_schema(),
        OpenHiddenAdjacentSignal::ts_definition(),
        OpenHiddenAdjacentSignal::zod_schema(),
        HiddenUntaggedSignal::ts_definition(),
        HiddenUntaggedSignal::zod_schema(),
        OpenHiddenUntaggedSignal::ts_definition(),
        OpenHiddenUntaggedSignal::zod_schema(),
    ]
}

#[test]
fn serde_refuses_a_stray_key_in_either_variant_of_a_strict_enum_and_reads_past_it_otherwise() {
    let told: Vec<Value> = rows()
        .into_iter()
        .map(|(name, json)| match name {
            "Signal" => serde_told::<Signal>(json),
            "OpenSignal" => serde_told::<OpenSignal>(json),
            "TaggedSignal" => serde_told::<TaggedSignal>(json),
            "OpenTaggedSignal" => serde_told::<OpenTaggedSignal>(json),
            "AdjacentSignal" => serde_told::<AdjacentSignal>(json),
            "OpenAdjacentSignal" => serde_told::<OpenAdjacentSignal>(json),
            "UntaggedSignal" => serde_told::<UntaggedSignal>(json),
            "OpenUntaggedSignal" => serde_told::<OpenUntaggedSignal>(json),
            "HiddenSignal" => serde_told::<HiddenSignal>(json),
            "OpenHiddenSignal" => serde_told::<OpenHiddenSignal>(json),
            "HiddenTaggedSignal" => serde_told::<HiddenTaggedSignal>(json),
            "OpenHiddenTaggedSignal" => serde_told::<OpenHiddenTaggedSignal>(json),
            "HiddenAdjacentSignal" => serde_told::<HiddenAdjacentSignal>(json),
            "OpenHiddenAdjacentSignal" => serde_told::<OpenHiddenAdjacentSignal>(json),
            "HiddenUntaggedSignal" => serde_told::<HiddenUntaggedSignal>(json),
            _ => serde_told::<OpenHiddenUntaggedSignal>(json),
        })
        .collect();
    assert_eq!(told, expected());
    let read: AdjacentSignal = serde_json::from_str(ADJACENT[0].0).unwrap();
    assert!(
        matches!(&read, AdjacentSignal::Off { reason, since: 0 } if reason.is_empty()),
        "{read:?}"
    );
}

#[test]
fn serde_names_the_stray_key_it_refuses_in_the_variant_of_no_member() {
    for (refused, json) in [
        (
            serde_json::from_str::<Signal>(EXTERNAL[2].0).map(drop),
            EXTERNAL[2].0,
        ),
        (
            serde_json::from_str::<TaggedSignal>(INTERNAL[2].0).map(drop),
            INTERNAL[2].0,
        ),
        (
            serde_json::from_str::<AdjacentSignal>(ADJACENT[2].0).map(drop),
            ADJACENT[2].0,
        ),
    ] {
        let told = refused.unwrap_err().to_string();
        assert!(
            told.starts_with("unknown field `stray`, there are no fields"),
            "{json}: {told}"
        );
    }
}

#[test]
fn the_json_schema_of_each_enum_accepts_what_serde_reads() {
    let taken: Vec<bool> = rows()
        .into_iter()
        .map(|(name, json)| {
            jsonschema::validator_for(&json_schema_of(name))
                .unwrap()
                .is_valid(&serde_json::from_str(json).unwrap())
        })
        .collect();
    let stated: Vec<bool> = expected().iter().map(Value::is_object).collect();
    assert_eq!(taken, stated);
}

#[test]
fn serde_writes_no_variant_it_neither_writes_nor_reads() {
    let hidden_signal = HiddenSignal::Closed { held: 1 };
    assert!(matches!(hidden_signal, HiddenSignal::Closed { held: 1 }));
    assert!(refused_write(&hidden_signal).contains("Closed cannot be serialized"));
    let open_hidden_signal = OpenHiddenSignal::Closed { held: 1 };
    assert!(matches!(
        open_hidden_signal,
        OpenHiddenSignal::Closed { held: 1 }
    ));
    assert!(refused_write(&open_hidden_signal).contains("Closed cannot be serialized"));
    let hidden_tagged_signal = HiddenTaggedSignal::Closed { held: 1 };
    assert!(matches!(
        hidden_tagged_signal,
        HiddenTaggedSignal::Closed { held: 1 }
    ));
    assert!(refused_write(&hidden_tagged_signal).contains("Closed cannot be serialized"));
    let open_hidden_tagged_signal = OpenHiddenTaggedSignal::Closed { held: 1 };
    assert!(matches!(
        open_hidden_tagged_signal,
        OpenHiddenTaggedSignal::Closed { held: 1 }
    ));
    assert!(refused_write(&open_hidden_tagged_signal).contains("Closed cannot be serialized"));
    let hidden_adjacent_signal = HiddenAdjacentSignal::Closed { held: 1 };
    assert!(matches!(
        hidden_adjacent_signal,
        HiddenAdjacentSignal::Closed { held: 1 }
    ));
    assert!(refused_write(&hidden_adjacent_signal).contains("Closed cannot be serialized"));
    let open_hidden_adjacent_signal = OpenHiddenAdjacentSignal::Closed { held: 1 };
    assert!(matches!(
        open_hidden_adjacent_signal,
        OpenHiddenAdjacentSignal::Closed { held: 1 }
    ));
    assert!(refused_write(&open_hidden_adjacent_signal).contains("Closed cannot be serialized"));
    let hidden_untagged_signal = HiddenUntaggedSignal::Closed { held: 1 };
    assert!(matches!(
        hidden_untagged_signal,
        HiddenUntaggedSignal::Closed { held: 1 }
    ));
    assert!(refused_write(&hidden_untagged_signal).contains("Closed cannot be serialized"));
    let open_hidden_untagged_signal = OpenHiddenUntaggedSignal::Closed { held: 1 };
    assert!(matches!(
        open_hidden_untagged_signal,
        OpenHiddenUntaggedSignal::Closed { held: 1 }
    ));
    assert!(refused_write(&open_hidden_untagged_signal).contains("Closed cannot be serialized"));
}

/// What serde answers refusing to write `value`.
fn refused_write<T>(value: &T) -> String
where
    T: Serialize,
{
    serde_json::to_string(value).unwrap_err().to_string()
}
