//! Enums with a variant serde neither writes nor reads, under `#[serde(skip)]` or under
//! `skip_serializing` and `skip_deserializing` together, under every tagging, and enums with a
//! variant serde drops in one direction alone. No surface describes the first kind, and no client
//! reads or writes it; the second kind is described as every variant is.

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use tixschema::model_schema;

/// Each payload of a variant serde writes, and of a variant it neither writes nor reads, of each
/// enum below. serde and Zod take the first and refuse the second; a variant serde drops in one
/// direction alone is described, so Zod takes it in both directions.
pub const ROWS: [Row; 16] = [
    Row {
        json: r#"{"kind":"Open","note":"a"}"#,
        read: reads::<Berthage>,
        read_as: "Berthage",
        serde: true,
        zod: true,
    },
    Row {
        json: r#"{"kind":"Closed","reason":"r"}"#,
        read: reads::<Berthage>,
        read_as: "Berthage",
        serde: false,
        zod: false,
    },
    Row {
        json: r#"{"kind":"Hidden"}"#,
        read: reads::<Berthage>,
        read_as: "Berthage",
        serde: false,
        zod: false,
    },
    Row {
        json: r#"{"Open":{"note":"a"}}"#,
        read: reads::<Wharfage>,
        read_as: "Wharfage",
        serde: true,
        zod: true,
    },
    Row {
        json: r#"{"Closed":{"reason":"r"}}"#,
        read: reads::<Wharfage>,
        read_as: "Wharfage",
        serde: false,
        zod: false,
    },
    Row {
        json: r#""Gone""#,
        read: reads::<Wharfage>,
        read_as: "Wharfage",
        serde: false,
        zod: false,
    },
    Row {
        json: r#"{"kind":"Open","body":{"note":"a"}}"#,
        read: reads::<Pilotage>,
        read_as: "Pilotage",
        serde: true,
        zod: true,
    },
    Row {
        json: r#"{"kind":"Closed","body":{"reason":"r"}}"#,
        read: reads::<Pilotage>,
        read_as: "Pilotage",
        serde: false,
        zod: false,
    },
    Row {
        json: r#"{"note":"a"}"#,
        read: reads::<Lighterage>,
        read_as: "Lighterage",
        serde: true,
        zod: true,
    },
    Row {
        json: r#"{"reason":"r"}"#,
        read: reads::<Lighterage>,
        read_as: "Lighterage",
        serde: false,
        zod: false,
    },
    Row {
        json: r#""Red""#,
        read: reads::<Keelage>,
        read_as: "Keelage",
        serde: true,
        zod: true,
    },
    Row {
        json: r#""Green""#,
        read: reads::<Keelage>,
        read_as: "Keelage",
        serde: false,
        zod: false,
    },
    Row {
        json: r#""Light""#,
        read: reads::<Cartage>,
        read_as: "Cartage",
        serde: true,
        zod: true,
    },
    Row {
        json: r#"{"Bulk":"b"}"#,
        read: reads::<Cartage>,
        read_as: "Cartage",
        serde: false,
        zod: false,
    },
    Row {
        json: r#"{"kind":"Closed","reason":"r"}"#,
        read: reads::<Tonnage>,
        read_as: "Tonnage",
        serde: true,
        zod: true,
    },
    Row {
        json: r#"{"kind":"Closed","reason":"r"}"#,
        read: reads::<Dockage>,
        read_as: "Dockage",
        serde: false,
        zod: true,
    },
];

/// One payload, the type it is read as, serde's read of that type, and whether serde and the Zod
/// schema of the type each take it.
pub struct Row {
    pub json: &'static str,
    pub read: fn(&str) -> bool,
    pub read_as: &'static str,
    pub serde: bool,
    pub zod: bool,
}

/// Under a tag alone: a variant of members and a newtype variant over text, neither written nor
/// read.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(tag = "kind")]
pub enum Berthage {
    #[serde(skip)]
    Closed {
        reason: String,
    },
    #[serde(skip)]
    Hidden(String),
    Open {
        note: String,
    },
}

/// No tag attribute: a variant of members, among them an `Option` serde would write as `null`, and
/// a unit variant, neither written nor read.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub enum Wharfage {
    #[serde(skip)]
    Closed {
        reason: Option<String>,
    },
    #[serde(skip)]
    Gone,
    Open {
        note: String,
    },
}

/// Under a tag and a content key: a variant of members, and one that declares one slot and takes
/// it off the wire, neither written nor read.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", content = "body")]
pub enum Pilotage {
    #[serde(skip)]
    Closed {
        reason: String,
    },
    #[serde(skip)]
    Lone(#[serde(skip)] u8),
    Open {
        note: String,
    },
}

/// Untagged: a variant of members neither written nor read.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(untagged)]
pub enum Lighterage {
    #[serde(skip)]
    Closed {
        reason: String,
    },
    Open {
        note: String,
    },
}

/// A plain enum: a unit variant neither written nor read.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub enum Keelage {
    #[serde(skip)]
    Green,
    Red,
}

/// Unit variants beside a variant of one slot under both one-way skips, neither written nor read:
/// serde writes the unit variants' names alone.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub enum Cartage {
    #[serde(skip_serializing, skip_deserializing)]
    Bulk(String),
    Heavy,
    Light,
}

/// Under a tag alone: a variant serde reads and never writes.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(tag = "kind")]
pub enum Tonnage {
    #[serde(skip_serializing)]
    Closed {
        reason: String,
    },
    Open {
        note: String,
    },
}

/// Under a tag alone: a variant serde writes and never reads.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(tag = "kind")]
pub enum Dockage {
    #[serde(skip_deserializing)]
    Closed {
        reason: String,
    },
    Open {
        note: String,
    },
}

/// What serde answers for a write of each variant it does not read, of each enum above: a refusal
/// for each variant it neither writes nor reads, then what it writes of [`Dockage`]'s.
pub fn unread_writes() -> Vec<String> {
    let said = |written: Result<String, serde_json::Error>| {
        written.unwrap_or_else(|refused| refused.to_string())
    };
    vec![
        said(serde_json::to_string(&Berthage::Closed {
            reason: "r".to_owned(),
        })),
        said(serde_json::to_string(&Berthage::Hidden("h".to_owned()))),
        said(serde_json::to_string(&Wharfage::Closed { reason: None })),
        said(serde_json::to_string(&Wharfage::Gone)),
        said(serde_json::to_string(&Pilotage::Closed {
            reason: "r".to_owned(),
        })),
        said(serde_json::to_string(&Pilotage::Lone(1))),
        said(serde_json::to_string(&Lighterage::Closed {
            reason: "r".to_owned(),
        })),
        said(serde_json::to_string(&Keelage::Green)),
        said(serde_json::to_string(&Cartage::Bulk("b".to_owned()))),
        said(serde_json::to_string(&Dockage::Closed {
            reason: "r".to_owned(),
        })),
    ]
}

/// Whether serde reads `json` as `T`.
fn reads<T>(json: &str) -> bool
where
    T: DeserializeOwned,
{
    serde_json::from_str::<T>(json).is_ok()
}
