//! The models a map keyed by a `bool` is held in, and the payloads each client is held to serde on:
//! a key that is neither `true` nor `false` is refused wherever the map sits.

use alloc::collections::BTreeMap;

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use tixschema::model_schema;

/// A variant that holds a map keyed by a `bool`, under a tag and a content key.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "kind", content = "content")]
pub enum AdjacentDrawer {
    Flagged(BTreeMap<bool, String>),
}

/// A map keyed by a `bool` in each place a member holds one.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Cubby {
    pub flag_list: Vec<BTreeMap<bool, String>>,
    pub flag_pair: (BTreeMap<bool, String>, u8),
    pub flags: BTreeMap<bool, String>,
    pub flags_by_number: BTreeMap<u32, BTreeMap<bool, String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub maybe_flags: Option<BTreeMap<bool, String>>,
}

/// A variant that holds a map keyed by a `bool`, under no tag attribute.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum Drawer {
    Flagged(BTreeMap<bool, String>),
}

/// One payload, the type a client run reads it as, and what serde answers of it.
pub struct Row {
    pub json: String,
    pub named: &'static str,
    pub serde: String,
}

/// What serde answers of `json` read as `Held`: the value it reads, written back, or `refused`.
fn serde_told<Held>(json: &str) -> String
where
    Held: DeserializeOwned + Serialize,
{
    serde_json::from_str::<Held>(json).map_or_else(
        |_| "refused".to_owned(),
        |read| format!("read {}", serde_json::to_string(&read).unwrap()),
    )
}

pub fn row<Held>(named: &'static str, json: &str) -> Row
where
    Held: DeserializeOwned + Serialize,
{
    Row {
        json: json.to_owned(),
        named,
        serde: serde_told::<Held>(json),
    }
}

/// A `Cubby` payload whose maps all read `flags`.
pub fn cubby_json(flags: &str) -> String {
    format!(
        r#"{{"flagList":[{flags}],"flagPair":[{flags},1],"flags":{flags},"flagsByNumber":{{"1":{flags}}},"maybeFlags":{flags}}}"#
    )
}

/// A `Cubby` payload whose map in `place` reads `flags` and whose others read `true` and `false`.
fn cubby_json_broken_at(place: &str, flags: &str) -> String {
    let sound = r#"{"false":"a","true":"b"}"#;
    let at = |name: &str| if name == place { flags } else { sound };
    format!(
        r#"{{"flagList":[{}],"flagPair":[{},1],"flags":{},"flagsByNumber":{{"1":{}}},"maybeFlags":{}}}"#,
        at("flagList"),
        at("flagPair"),
        at("flags"),
        at("flagsByNumber"),
        at("maybeFlags"),
    )
}

pub fn rows() -> Vec<Row> {
    let sound = r#"{"false":"a","true":"b"}"#;
    let broken = r#"{"maybe":"a"}"#;
    let mut rows = vec![
        row::<Cubby>("Cubby", &cubby_json(sound)),
        row::<Cubby>("Cubby", &cubby_json("{}")),
        row::<Cubby>("Cubby", &cubby_json(r#"{"true":"b"}"#)),
        row::<Cubby>("Cubby", &cubby_json(r#"{"True":"b"}"#)),
        row::<Cubby>("Cubby", &cubby_json(r#"{"":"b"}"#)),
    ];
    for place in [
        "flags",
        "flagsByNumber",
        "flagList",
        "maybeFlags",
        "flagPair",
    ] {
        rows.push(row::<Cubby>("Cubby", &cubby_json_broken_at(place, broken)));
    }
    rows.extend([
        row::<Drawer>("Drawer", &format!(r#"{{"Flagged":{sound}}}"#)),
        row::<Drawer>("Drawer", &format!(r#"{{"Flagged":{broken}}}"#)),
        row::<AdjacentDrawer>(
            "AdjacentDrawer",
            &format!(r#"{{"kind":"Flagged","content":{sound}}}"#),
        ),
        row::<AdjacentDrawer>(
            "AdjacentDrawer",
            &format!(r#"{{"kind":"Flagged","content":{broken}}}"#),
        ),
    ]);
    rows
}
