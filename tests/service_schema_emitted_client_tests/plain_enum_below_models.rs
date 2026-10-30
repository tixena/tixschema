//! A plain enum flattened into a holder that is declared above it. serde writes the enum as the
//! name of its variant over `null`, as an entry beside the holder's members, and reads it back.

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use tixschema::model_schema;

/// Each payload a model below is read from: the type it is read as, and the payload.
pub const ROWS: [(&str, &str); 8] = [
    ("Casement", r#"{"Sound":null,"breadth":1}"#),
    ("Casement", r#"{"breadth":1,"Worn":null}"#),
    ("Transom", r#"{"Worn":null,"width":2}"#),
    ("Mullion", r#"{"Sound":null,"breadth":3}"#),
    ("Mullion", r#"{"breadth":3,"Worn":null}"#),
    ("Mullion", r#"{"breadth":3}"#),
    ("Pane", r#"{"clear":null,"breadth":4}"#),
    ("Pane", r#"{"breadth":4,"Frosted":null}"#),
];

/// Each payload a model below refuses to read: the type, and a payload with no entry that names a
/// variant, or one that holds a value other than `null`.
pub const REFUSED: [(&str, &str); 4] = [
    ("Casement", r#"{"breadth":1}"#),
    ("Pane", r#"{"Clear":null,"breadth":4}"#),
    ("Casement", r#"{"Sound":1,"breadth":1}"#),
    ("Transom", r#"{"width":2}"#),
];

/// A payload of [`Reefing`], whose brand over a `String` is no object: serde refuses it, and so
/// does the Kotlin client.
pub const BRANDED: (&str, &str) = ("Reefing", r#"{"breadth":1,"abc":null}"#);

/// A struct that flattens a plain enum declared below it, beside a member of its own.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Casement {
    pub breadth: u32,
    #[serde(flatten)]
    pub weathering: Weathering,
}

/// A struct that flattens the enum ahead of a member of its own.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Transom {
    #[serde(flatten)]
    pub weathering: Weathering,
    pub width: u32,
}

/// A struct that flattens the enum optionally.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Mullion {
    pub breadth: u32,
    #[serde(default, flatten, skip_serializing_if = "Option::is_none")]
    pub weathering: Option<Weathering>,
}

/// A struct that flattens an enum declared below it that names its variants in another case and
/// reads one under an alias.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Pane {
    pub breadth: u32,
    #[serde(flatten)]
    pub glazing: Glazing,
}

/// A struct that flattens a brand over a `String` declared below it, which serde writes as no
/// object and refuses to read.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Reefing {
    #[serde(flatten)]
    pub bobstay: Bobstay,
    pub breadth: u32,
}

/// The brand [`Reefing`] flattens.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(transparent)]
pub struct Bobstay(pub String);

/// The plain enum the models above flatten.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum Weathering {
    Sound,
    Worn,
}

/// The plain enum [`Pane`] flattens.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Glazing {
    Clear,
    #[serde(alias = "Frosted")]
    Opaque,
}

/// What serde writes back of `payload` read as a `Read`, or `refused`.
fn written_back<Read>(payload: &str) -> String
where
    Read: DeserializeOwned + Serialize,
{
    serde_json::from_str::<Read>(payload).map_or_else(
        |_| "refused".to_owned(),
        |read| serde_json::to_string(&read).unwrap(),
    )
}

/// What serde writes back of the payload of each row of [`ROWS`], and whether it refuses each
/// payload of [`REFUSED`].
pub fn serde_wrote() -> (Vec<String>, Vec<String>) {
    let by_name = |name: &str, payload: &str| match name {
        "Casement" => written_back::<Casement>(payload),
        "Transom" => written_back::<Transom>(payload),
        "Pane" => written_back::<Pane>(payload),
        _ => written_back::<Mullion>(payload),
    };
    (
        ROWS.iter()
            .map(|(name, payload)| by_name(name, payload))
            .collect(),
        REFUSED
            .iter()
            .map(|(name, payload)| by_name(name, payload))
            .collect(),
    )
}

/// serde writes the enum as an entry of its variant's name over `null`, reads it back, and refuses
/// a payload with no such entry or with a value in it.
#[test]
fn serde_writes_a_flattened_plain_enum_as_an_entry_over_null() {
    let (wrote, refused) = serde_wrote();
    assert_eq!(
        wrote,
        [
            r#"{"breadth":1,"Sound":null}"#,
            r#"{"breadth":1,"Worn":null}"#,
            r#"{"Worn":null,"width":2}"#,
            r#"{"breadth":3,"Sound":null}"#,
            r#"{"breadth":3,"Worn":null}"#,
            r#"{"breadth":3}"#,
            r#"{"breadth":4,"clear":null}"#,
            r#"{"breadth":4,"opaque":null}"#,
        ]
    );
    assert_eq!(refused, ["refused"; 4]);
    assert_eq!(written_back::<Reefing>(BRANDED.1), "refused");
}

/// Checks the text a client printed against what serde writes back of each row of [`ROWS`], one
/// line `name: json` per row, and then one line `name refused: true` per payload of [`REFUSED`].
#[cfg(any(feature = "dart", feature = "swift", feature = "kotlin"))]
pub fn assert_printed_as_serde_writes(printed: &str) {
    let (written, _) = serde_wrote();
    let mut lines = printed.lines();
    for ((name, _), wrote) in ROWS.iter().zip(written) {
        let line = lines.next().unwrap();
        let (printed_name, json) = line.split_once(": ").unwrap();
        assert_eq!(printed_name, *name, "{printed}");
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(json).unwrap(),
            serde_json::from_str::<serde_json::Value>(&wrote).unwrap(),
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
