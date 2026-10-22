//! An `i64` held in each place a member holds one: the value itself, a list's item, a map's value,
//! an `Option`, a tuple's position, a brand's slot and a variant's member, each written as the
//! string of its digits. Each client reads each payload serde writes and writes it back as serde
//! does, and refuses each payload serde refuses: a digit string beyond an `i64`, a text of no
//! digits, and a value beyond its bound.

use alloc::collections::BTreeMap;

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use tixschema::model_schema;

/// Each payload serde writes, and the type it is read as.
pub const WRITTEN: [(&str, &str); 6] = [
    (
        "SixtyTally",
        r#"{"keyed":{"3":"-7"},"listed":["0","9223372036854775807"],"maybe":"-9223372036854775808","paired":["9","x"],"signed":"-1152921504606846976"}"#,
    ),
    (
        "SixtyTally",
        r#"{"keyed":{},"listed":[],"paired":["1","y"],"signed":"0"}"#,
    ),
    ("SixtyId", r#""1152921504606846976""#),
    ("SixtyEvent", r#"{"Counted":"5"}"#),
    ("SixtyEvent", r#"{"Moved":{"from":"-1","to":"2"}}"#),
    ("SixtyLevel", r#"{"level":"-5"}"#),
];

/// Each payload serde refuses, the type it is read as, and what serde answers, its position left
/// out.
pub const REFUSED: [(&str, &str, &str); 5] = [
    (
        "SixtyId",
        r#""9223372036854775808""#,
        r#"invalid value: string "9223372036854775808", expected an i64 as a string of its decimal digits"#,
    ),
    (
        "SixtyId",
        r#""seven""#,
        r#"invalid value: string "seven", expected an i64 as a string of its decimal digits"#,
    ),
    (
        "SixtyTally",
        r#"{"keyed":{},"listed":["1","-9223372036854775809"],"paired":["1","y"],"signed":"0"}"#,
        r#"invalid value: string "-9223372036854775809", expected an i64 as a string of its decimal digits"#,
    ),
    (
        "SixtyEvent",
        r#"{"Moved":{"from":"1.5","to":"2"}}"#,
        r#"invalid value: string "1.5", expected an i64 as a string of its decimal digits"#,
    ),
    (
        "SixtyLevel",
        r#"{"level":"6"}"#,
        "'level': too large: maximum is 5, got 6",
    ),
];

/// An `i64` in each place a member of a struct holds one.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SixtyTally {
    pub keyed: BTreeMap<u32, i64>,
    pub listed: Vec<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub maybe: Option<i64>,
    pub paired: (i64, String),
    pub signed: i64,
}

/// A brand over an `i64`.
#[model_schema()]
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(transparent)]
pub struct SixtyId(pub i64);

/// An `i64` as a variant's one value and as its members, with no tag attribute.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum SixtyEvent {
    Counted(i64),
    Moved { from: i64, to: i64 },
}

/// An `i64` under a bound, which `validate()` holds the integer to.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SixtyLevel {
    #[model_schema_prop(minimum = -5, maximum = 5)]
    pub level: i64,
}

/// What serde writes back of `json` read as the type `read_as` names.
pub fn serde_written(read_as: &str, json: &str) -> String {
    match read_as {
        "SixtyTally" => written_back::<SixtyTally>(json),
        "SixtyId" => written_back::<SixtyId>(json),
        "SixtyEvent" => written_back::<SixtyEvent>(json),
        _ => written_back::<SixtyLevel>(json),
    }
}

/// What serde answers of `json` read as the type `read_as` names, its position left out, or
/// `None` where it reads it.
pub fn serde_refusal(read_as: &str, json: &str) -> Option<String> {
    match read_as {
        "SixtyTally" => refusal_of::<SixtyTally>(json),
        "SixtyId" => refusal_of::<SixtyId>(json),
        "SixtyEvent" => refusal_of::<SixtyEvent>(json),
        _ => refusal_of::<SixtyLevel>(json),
    }
}

/// What serde writes back of `json` read as `T`.
fn written_back<T>(json: &str) -> String
where
    T: DeserializeOwned + Serialize,
{
    serde_json::to_string(&serde_json::from_str::<T>(json).unwrap()).unwrap()
}

/// What serde answers of `json` read as `T`, or `None` where it reads it.
fn refusal_of<T>(json: &str) -> Option<String>
where
    T: DeserializeOwned,
{
    serde_json::from_str::<T>(json).err().map(|refused| {
        let said = refused.to_string();
        said.split_once(" at line ")
            .map_or_else(|| said.clone(), |(stated, _)| stated.to_owned())
    })
}

/// serde reads each payload of [`WRITTEN`] and writes it back as it was, and refuses each of
/// [`REFUSED`] in the words beside it.
#[test]
fn serde_reads_and_refuses_each_payload_as_the_rows_state() {
    for (read_as, json) in WRITTEN {
        assert_eq!(serde_refusal(read_as, json), None, "serde refuses {json}");
        assert_eq!(
            serde_written(read_as, json),
            json,
            "serde writes {json} back"
        );
    }
    for (read_as, json, said) in REFUSED {
        assert_eq!(
            serde_refusal(read_as, json).as_deref(),
            Some(said),
            "serde's refusal of {json}"
        );
    }
}
