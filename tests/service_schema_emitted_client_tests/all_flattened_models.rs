//! A struct whose every member is flattened, a struct or a map, and an enum whose
//! variant holds only a flattened member under each tagging. Each client reads the payloads below
//! and writes them back as serde does.

use alloc::collections::BTreeMap;

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use tixschema::model_schema;

/// Each payload a model below is read from: the type it is read as, and the payload.
pub const ROWS: [(&str, &str); 7] = [
    ("Keelson", r#"{"label":"n"}"#),
    ("Gimbal", r#"{"height":2,"label":"n"}"#),
    ("Berth", r#"{"north":2}"#),
    ("Rigging", r#"{"kind":"Lit","label":"n"}"#),
    ("Rigged", r#"{"Lit":{"label":"n"}}"#),
    ("Rigs", r#"{"kind":"Lit","load":{"label":"n"}}"#),
    ("Hoist", r#"{"label":"n"}"#),
];

/// Each payload a model below refuses to read: the type, and a payload that lacks the keys of the
/// flattened struct or breaks its bound.
pub const REFUSED: [(&str, &str); 5] = [
    ("Keelson", "{}"),
    ("Keelson", r#"{"label":""}"#),
    ("Rigging", r#"{"kind":"Lit","label":""}"#),
    ("Gimbal", r#"{"label":"n"}"#),
    ("Berth", r#"{"north":"x"}"#),
];

/// The struct every model below flattens.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Lantern {
    #[model_schema_prop(minLength = 1)]
    pub label: String,
}

/// A struct whose one member is flattened.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Keelson {
    #[serde(flatten)]
    pub lantern: Lantern,
}

/// A second struct the models below flatten.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Mast {
    pub height: u8,
}

/// A struct whose members are two flattened structs.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Gimbal {
    #[serde(flatten)]
    pub lantern: Lantern,
    #[serde(flatten)]
    pub mast: Mast,
}

/// A struct whose one member is a flattened map.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Berth {
    #[serde(flatten)]
    pub by_dock: BTreeMap<String, u8>,
}

/// A variant under a tag alone whose one member is flattened.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind")]
pub enum Rigging {
    Dark,
    Lit {
        #[serde(flatten)]
        lantern: Lantern,
    },
}

/// A variant under no tag attribute whose one member is flattened.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum Rigged {
    Dark,
    Lit {
        #[serde(flatten)]
        lantern: Lantern,
    },
}

/// A variant under a tag and a content key whose one member is flattened.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", content = "load")]
pub enum Rigs {
    Dark,
    Lit {
        #[serde(flatten)]
        lantern: Lantern,
    },
}

/// A variant of an untagged enum whose one member is flattened.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(untagged)]
pub enum Hoist {
    Dark(u8),
    Lit {
        #[serde(flatten)]
        lantern: Lantern,
    },
}

/// The refusal serde answers `payload` read as a `Read`, or `read` where it reads.
fn refusal<Read>(payload: &str) -> String
where
    Read: DeserializeOwned,
{
    serde_json::from_str::<Read>(payload)
        .map_or_else(|refused| refused.to_string(), |_| "read".to_owned())
}

/// What serde writes back of `payload` read as a `Read`.
fn written_back<Read>(payload: &str) -> serde_json::Value
where
    Read: DeserializeOwned + Serialize,
{
    serde_json::to_value(serde_json::from_str::<Read>(payload).unwrap()).unwrap()
}

/// What serde writes back of the payload of each row of [`ROWS`], as JSON.
pub fn serde_wrote() -> Vec<serde_json::Value> {
    vec![
        written_back::<Keelson>(ROWS[0].1),
        written_back::<Gimbal>(ROWS[1].1),
        written_back::<Berth>(ROWS[2].1),
        written_back::<Rigging>(ROWS[3].1),
        written_back::<Rigged>(ROWS[4].1),
        written_back::<Rigs>(ROWS[5].1),
        written_back::<Hoist>(ROWS[6].1),
    ]
}

/// Each row of [`ROWS`] is written back as it is read, and each payload of [`REFUSED`] is
/// refused.
#[test]
fn serde_reads_a_model_of_flattened_members_alone_and_writes_it_back() {
    for ((name, payload), wrote) in ROWS.iter().zip(serde_wrote()) {
        assert_eq!(
            wrote,
            serde_json::from_str::<serde_json::Value>(payload).unwrap(),
            "{name}"
        );
    }
    assert_eq!(
        [
            refusal::<Keelson>(REFUSED[0].1),
            refusal::<Keelson>(REFUSED[1].1),
            refusal::<Rigging>(REFUSED[2].1),
            refusal::<Gimbal>(REFUSED[3].1),
            refusal::<Berth>(REFUSED[4].1),
        ],
        [
            "missing field `label` at line 1 column 2",
            "'label': too short: minimum length is 1, got 0",
            "'label': too short: minimum length is 1, got 0",
            "missing field `height` at line 1 column 13",
            "invalid type: string \"x\", expected u8 at line 1 column 13",
        ]
    );
}

/// Holds the text a client printed to what serde writes back of each row of [`ROWS`], and to the
/// refusal of each payload of [`REFUSED`]: a line `name: json` per row, then a line
/// `name refused: true` per refusal.
#[cfg(any(feature = "dart", feature = "swift", feature = "kotlin"))]
pub fn assert_printed_as_serde_writes(printed: &str) {
    let mut lines = printed.lines();
    for ((name, _), wrote) in ROWS.iter().zip(serde_wrote()) {
        let line = lines.next().unwrap();
        let (printed_name, json) = line.split_once(": ").unwrap();
        assert_eq!(printed_name, *name, "{printed}");
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(json).unwrap(),
            wrote,
            "{name} is not written back as serde writes it: {printed}"
        );
    }
    for (name, _) in REFUSED {
        assert_eq!(
            lines.next(),
            Some(format!("{name} refused: true").as_str()),
            "{printed}"
        );
    }
    assert_eq!(lines.next(), None, "{printed}");
}
