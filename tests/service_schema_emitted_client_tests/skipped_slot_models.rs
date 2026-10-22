//! Tuple variants with a slot serde neither writes nor reads: serde writes the array of the slots
//! that remain, of one slot where one is left and empty where none is, and a variant that declares
//! one slot and drops it as a unit variant. TypeScript and Zod describe that array, and each client
//! reads and writes it.

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use tixschema::model_schema;

/// Each payload, the type it is read as, and what serde, Zod and each client answer of it: the
/// payload written back, or `None` where it is refused.
pub const ROWS: [Row; 14] = [
    Row::read("Bowsed", r#"{"Pair":["a",3]}"#),
    Row::refused("Bowsed", r#"{"Pair":["a","b",3]}"#),
    Row::read("Bowsed", r#"{"Duo":["a"]}"#),
    Row::refused("Bowsed", r#"{"Duo":["a",9]}"#),
    Row::refused("Bowsed", r#"{"Duo":"a"}"#),
    Row::read("Bowsed", r#""Lone""#),
    Row::refused("Bowsed", r#"{"Lone":"x"}"#),
    Row::read("Bowsed", r#"{"Gone":[]}"#),
    Row::refused("Bowsed", r#"{"Gone":null}"#),
    Row::refused("Bowsed", r#"{"Gone":[1]}"#),
    Row::read("Bowsed", r#"{"Tallied":["a",3]}"#),
    Row::read("Clewed", r#"{"t":"Pair","c":["a",3]}"#),
    Row::refused("Clewed", r#"{"t":"Pair","c":["a",true,3]}"#),
    Row::read("Clewed", r#"{"t":"Duo","c":[4]}"#),
];

/// One payload, and whether every side reads it, in which case each writes it back as it was.
pub struct Row {
    pub json: &'static str,
    pub read: bool,
    pub read_as: &'static str,
}

impl Row {
    /// A payload every side reads and writes back as it was.
    const fn read(read_as: &'static str, json: &'static str) -> Self {
        Self {
            json,
            read: true,
            read_as,
        }
    }

    /// A payload every side refuses.
    const fn refused(read_as: &'static str, json: &'static str) -> Self {
        Self {
            json,
            read: false,
            read_as,
        }
    }

    /// What a side that agrees with serde answers of the payload: `read` and the payload written
    /// back, or `refused`.
    pub fn told(&self) -> String {
        if self.read {
            format!("read {}", self.json)
        } else {
            "refused".to_owned()
        }
    }
}

/// An enum with no tag attribute whose tuple variants drop a slot: in the middle, at the end, every
/// one, the only one, and a slot of a type no client declares.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum Bowsed {
    Duo(String, #[serde(skip)] u64),
    Gone(#[serde(skip)] u8, #[serde(skip)] u8),
    Lone(#[serde(skip)] String),
    Open { note: String },
    Pair(String, #[serde(skip)] String, u8),
    Tallied(String, #[serde(skip)] serde_json::Value, u8),
}

/// An enum under a tag and a content key whose tuple variants drop a slot, the first or the one
/// in the middle.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "t", content = "c")]
pub enum Clewed {
    Duo(#[serde(skip)] String, u32),
    Pair(String, #[serde(skip)] bool, u8),
}

/// What serde writes back of `json` read as the type `read_as` names, or `refused`.
pub fn serde_told(read_as: &str, json: &str) -> String {
    if read_as == "Bowsed" {
        written_back::<Bowsed>(json)
    } else {
        written_back::<Clewed>(json)
    }
}

/// What serde writes back of `json` read as `T`, or `refused`.
fn written_back<T>(json: &str) -> String
where
    T: DeserializeOwned + Serialize,
{
    serde_json::from_str::<T>(json).map_or_else(
        |_| "refused".to_owned(),
        |read| format!("read {}", serde_json::to_string(&read).unwrap()),
    )
}
