//! A member under an alias in each way the Swift read reads a member under its key: read as its
//! default where the payload holds none of its names, read from what a function answers, a model
//! `validate()` walks, optional too, and a map Swift reads through a wrapper. serde reads each
//! under each of its names, and refuses a member under two of them.

use alloc::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use tixschema::model_schema;

/// What [`Tholing`] is read from: each member under its own name, each under its alias, the
/// defaulted and the optional member under neither, a model under its alias that breaks its bound,
/// and a member under its name and its alias, which serde refuses.
pub const THOLING: [&str; 5] = [
    r#"{"by_tide":{"Ebb":1},"count":2,"mark":" m ","spare":{"label":"b"},"spud":{"label":"a"}}"#,
    r#"{"tides":{"Flood":1},"tally":2,"tag":" m ","spare_post":{"label":"b"},"post":{"label":"a"}}"#,
    r#"{"tides":{},"tag":"m","post":{"label":"a"}}"#,
    r#"{"by_tide":{},"tag":"m","post":{"label":""}}"#,
    r#"{"by_tide":{},"count":1,"tally":2,"mark":"m","spud":{"label":"a"}}"#,
];

/// A model with a bound, which the `validate()` of a type that holds it walks.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Spud {
    #[model_schema_prop(minLength = 1)]
    pub label: String,
}

/// A plain enum a map is keyed by, which Swift reads through a wrapper written for the map.
#[model_schema()]
#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub enum Tideway {
    Ebb,
    Flood,
}

/// A member under an alias in each way the Swift read reads a member under its key.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Tholing {
    #[serde(alias = "tides")]
    pub by_tide: BTreeMap<Tideway, u8>,
    #[serde(alias = "tally", default)]
    pub count: u32,
    #[model_schema_prop(preprocess = ["unpadded"])]
    #[serde(alias = "tag")]
    pub mark: String,
    #[serde(alias = "spare_post", default, skip_serializing_if = "Option::is_none")]
    pub spare: Option<Spud>,
    #[serde(alias = "post")]
    pub spud: Spud,
}

/// `payload`, a string, without the spaces around it.
fn unpadded(payload: Value) -> Value {
    if let Value::String(text) = payload {
        Value::String(text.trim().to_owned())
    } else {
        payload
    }
}

/// What serde reads of each payload: each member under each of its names, written back under its
/// own, the defaulted and the optional member read as their default where no name is held, and a
/// refusal of a model that breaks its bound and of a member under two of its names.
#[test]
fn serde_reads_each_member_under_each_alias() {
    let read = THOLING.map(|payload| {
        serde_json::from_str::<Tholing>(payload).map_or_else(
            |refused| refused.to_string(),
            |held| serde_json::to_string(&held).unwrap(),
        )
    });
    assert_eq!(
        read,
        [
            r#"{"by_tide":{"Ebb":1},"count":2,"mark":"m","spare":{"label":"b"},"spud":{"label":"a"}}"#,
            r#"{"by_tide":{"Flood":1},"count":2,"mark":"m","spare":{"label":"b"},"spud":{"label":"a"}}"#,
            r#"{"by_tide":{},"count":0,"mark":"m","spud":{"label":"a"}}"#,
            "'spud.label': too short: minimum length is 1, got 0",
            "duplicate field `count` at line 1 column 31",
        ],
        "what serde reads of each payload"
    );
}
