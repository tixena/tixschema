//! Models whose names on the wire hold a `$`, a quote, a double quote and a backslash, each a
//! character a Dart or a Swift string literal reads apart from text: a struct's members, its own
//! tag key and its name, an alias of a member and of a variant, an enum's variants under each
//! tagging, its tag key and its content key, a plain enum's values, and a struct and an
//! externally tagged enum flattened into a holder beside a member of its own.

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use tixschema::model_schema;

/// What serde writes of each model, in the order the payloads below are read.
pub const WRITTEN: [(&str, &str); 14] = [
    (
        "Tariffed",
        r#"{"ki$nd":"Price's$Tag\\","back\\slash":"b","cost$usd":"c","say \"hi\"":"s"}"#,
    ),
    ("Tolled", r#"{"Bid$Ask":{"lo$t":1}}"#),
    ("Tolled", r#"{"C:\\Path":"p"}"#),
    ("Tolled", r#""Calm\"""#),
    ("Levied", r#"{"ki'nd":"Bid$Ask","lo$t":1}"#),
    ("Levied", r#"{"ki'nd":"He\"ld","cost$usd":"c"}"#),
    ("Levied", r#"{"ki'nd":"Calm\\"}"#),
    ("Dutied", r#"{"ki$nd":"Bid$Ask","bo'dy":{"lot":1}}"#),
    ("Dutied", r#"{"ki$nd":"He\"ld","bo'dy":"h"}"#),
    ("Dutied", r#"{"ki$nd":"Calm\\"}"#),
    ("Tithed", r#"{"cost$usd":"c","no'te":"n"}"#),
    ("Mulcted", r#"{"no'te":"n","C:\\Path":"p"}"#),
    ("Excised", r#""hi'gh""#),
    ("Excised", r#""lo$w""#),
];

/// A payload under an alias serde reads, the type it is read as, and what serde writes back.
pub const UNDER_AN_ALIAS: [(&str, &str, &str); 4] = [
    (
        "Tariffed",
        r#"{"ki$nd":"Price's$Tag\\","back\\slash":"b","cost's":"c","say \"hi\"":"s"}"#,
        r#"{"ki$nd":"Price's$Tag\\","back\\slash":"b","cost$usd":"c","say \"hi\"":"s"}"#,
    ),
    (
        "Tolled",
        r#"{"O'Neil":{"lo$t":1}}"#,
        r#"{"Bid$Ask":{"lo$t":1}}"#,
    ),
    (
        "Mulcted",
        r#"{"no'te":"n","O'Neil":{"lo$t":1}}"#,
        r#"{"no'te":"n","Bid$Ask":{"lo$t":1}}"#,
    ),
    ("Excised", r#""to\"p""#, r#""hi'gh""#),
];

/// A struct under its own tag, its tag key and its name, and the names of its members, one of them
/// read under an alias too, holding a `$`, a quote, a double quote and a backslash.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "ki$nd", rename = "Price's$Tag\\")]
pub struct Tariffed {
    #[serde(rename = "back\\slash")]
    pub backslash: String,
    #[serde(rename = "cost$usd", alias = "cost's")]
    pub cost: String,
    #[serde(rename = "say \"hi\"")]
    pub quoted: String,
}

/// An externally tagged enum whose variants, one of them read under an alias too, and a variant's
/// member are written under such names.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum Tolled {
    #[serde(rename = "Bid$Ask", alias = "O'Neil")]
    Bid {
        #[serde(rename = "lo$t")]
        lot: u8,
    },
    #[serde(rename = "C:\\Path")]
    Held(String),
    #[serde(rename = "Calm\"")]
    Still,
}

/// A struct whose member is written under such a name.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Leveed {
    #[serde(rename = "cost$usd")]
    pub cost: String,
}

/// An enum under a tag alone, its tag key, its variants and a variant's member written under such
/// names.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "ki'nd")]
pub enum Levied {
    #[serde(rename = "Bid$Ask")]
    Bid {
        #[serde(rename = "lo$t")]
        lot: u8,
    },
    #[serde(rename = "He\"ld")]
    Held(Leveed),
    #[serde(rename = "Calm\\")]
    Still,
}

/// An enum under a tag and a content key, both keys and its variants written under such names.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "ki$nd", content = "bo'dy")]
pub enum Dutied {
    #[serde(rename = "Bid$Ask")]
    Bid { lot: u8 },
    #[serde(rename = "He\"ld")]
    Held(String),
    #[serde(rename = "Calm\\")]
    Still,
}

/// A plain enum whose values, one of them read under an alias too, are such names.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum Excised {
    #[serde(rename = "hi'gh", alias = "to\"p")]
    High,
    #[serde(rename = "lo$w")]
    Low,
}

/// A struct flattened beside a member of its holder's own written under such a name.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Tithed {
    #[serde(flatten)]
    pub leveed: Leveed,
    #[serde(rename = "no'te")]
    pub note: String,
}

/// An externally tagged enum flattened beside a member of its holder's own written under such a
/// name.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Mulcted {
    #[serde(rename = "no'te")]
    pub note: String,
    #[serde(flatten)]
    pub tolled: Tolled,
}

/// What serde writes back of `json` read as the model `read_as` names.
pub fn serde_written(read_as: &str, json: &str) -> String {
    match read_as {
        "Tariffed" => written_back::<Tariffed>(json),
        "Tolled" => written_back::<Tolled>(json),
        "Leveed" => written_back::<Leveed>(json),
        "Levied" => written_back::<Levied>(json),
        "Dutied" => written_back::<Dutied>(json),
        "Excised" => written_back::<Excised>(json),
        "Tithed" => written_back::<Tithed>(json),
        _ => written_back::<Mulcted>(json),
    }
}

/// What serde writes back of `json` read as `T`.
fn written_back<T>(json: &str) -> String
where
    T: DeserializeOwned + Serialize,
{
    serde_json::to_string(&serde_json::from_str::<T>(json).unwrap()).unwrap()
}
