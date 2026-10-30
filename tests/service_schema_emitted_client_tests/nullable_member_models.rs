//! The models a nullable member is read on, in a struct with nothing to write a read for and in
//! one that flattens a member, and what serde reads of each payload.

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use tixschema::model_schema;

/// Each payload: its name, the type that reads it, and the payload.
pub const ROWS: [(&str, &str, &str); 9] = [
    ("Berth, no key", "Berth", "{}"),
    (
        "Berth, each a null",
        "Berth",
        r#"{"berthNote":null,"heldHawser":null}"#,
    ),
    (
        "Berth, each held",
        "Berth",
        r#"{"berthNote":"n","heldHawser":{"hawserCode":"h"}}"#,
    ),
    (
        "Slipway, no slipNote",
        "Slipway",
        r#"{"keelName":"k","heldHawser":null}"#,
    ),
    (
        "Slipway, no heldHawser",
        "Slipway",
        r#"{"keelName":"k","slipNote":null}"#,
    ),
    (
        "Slipway, no key but the flattened one",
        "Slipway",
        r#"{"keelName":"k"}"#,
    ),
    (
        "Slipway, each a null",
        "Slipway",
        r#"{"keelName":"k","slipNote":null,"heldHawser":null}"#,
    ),
    (
        "Slipway, each held",
        "Slipway",
        r#"{"keelName":"k","slipNote":"n","heldHawser":{"hawserCode":"h"}}"#,
    ),
    (
        "Slipway, no keelName",
        "Slipway",
        r#"{"slipNote":null,"heldHawser":null}"#,
    ),
];

/// A nullable member over a string and over a model.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Berth {
    #[model_schema_prop(nullable)]
    pub berth_note: Option<String>,
    #[model_schema_prop(nullable)]
    pub held_hawser: Option<Hawser>,
}

/// A model held by a nullable member.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Hawser {
    #[model_schema_prop(minLength = 1)]
    pub hawser_code: String,
}

#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Keel {
    pub keel_name: String,
}

/// The nullable members of [`Berth`] beside a flattened member.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Slipway {
    #[model_schema_prop(nullable)]
    pub held_hawser: Option<Hawser>,
    #[serde(flatten)]
    pub keel: Keel,
    #[model_schema_prop(nullable)]
    pub slip_note: Option<String>,
}

fn read_as<Read>(payload: &str) -> String
where
    Read: DeserializeOwned + Serialize,
{
    serde_json::from_str::<Read>(payload).map_or_else(
        |_| "refused".to_owned(),
        |held| format!("read {}", serde_json::to_string(&held).unwrap()),
    )
}

/// What serde reads of each row, as the runs print it.
pub fn expected() -> Vec<String> {
    ROWS.iter()
        .map(|&(name, read, payload)| {
            let answer = match read {
                "Berth" => read_as::<Berth>(payload),
                _ => read_as::<Slipway>(payload),
            };
            format!("{name}: {answer}")
        })
        .collect()
}

/// Whether `printed` holds the lines `expected` holds, a read compared as the JSON it holds.
pub fn agrees(printed: &str, expected: &[String]) -> bool {
    let lines: Vec<&str> = printed.lines().collect();
    lines.len() == expected.len()
        && lines.iter().zip(expected).all(|(line, wanted)| {
            match (line.split_once(": read "), wanted.split_once(": read ")) {
                (Some((name, held)), Some((wanted_name, wanted_held))) => {
                    name == wanted_name
                        && serde_json::from_str::<serde_json::Value>(held).ok()
                            == serde_json::from_str::<serde_json::Value>(wanted_held).ok()
                }
                _ => line == wanted,
            }
        })
}
