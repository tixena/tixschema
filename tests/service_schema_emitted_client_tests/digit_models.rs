//! An `i64` held in each place a member holds one: the value itself, a list's item, a map's value,
//! an `Option`, a tuple's position, a brand's slot, a variant's member, an alias, a range's bound,
//! a `Result`'s arm, a literal, a lock below a list, a map, an `Option` and a tuple, and beside the
//! type's own parameter, each written as the string of its digits. Each client reads each payload
//! serde writes and writes it back as serde does, and refuses each payload serde refuses: a digit
//! string beyond an `i64`, a text of no digits, and a value beyond its bound.

use alloc::collections::BTreeMap;
use core::cell::RefCell;
use core::ops::{Range, RangeFrom, RangeInclusive, RangeTo};
use std::sync::{Mutex, RwLock};

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use tixschema::model_schema;

/// Each payload serde writes, and the type it is read as.
pub const WRITTEN: [(&str, &str); 11] = [
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
    (
        "SixtyReached",
        r#"{"aliased":"-1152921504606846976","aliases":["1","9223372036854775807"],"fixed":"214","from":{"start":"-3"},"inclusive":{"start":"4","end":"5"},"ranged":{"start":"1","end":"1152921504606846976"},"resulted":{"Ok":"7"},"to":{"end":"6"},"wrong":{"Err":["8","-9"]}}"#,
    ),
    (
        "SixtyReached",
        r#"{"aliased":"0","fixed":"214","from":{"start":"0"},"inclusive":{"start":"0","end":"0"},"ranged":{"start":"0","end":"0"},"resulted":{"Err":"e"},"to":{"end":"0"},"wrong":{"Ok":"o"}}"#,
    ),
    (
        "SixtyLocked",
        r#"{"keyed":{"4":"-7"},"listed":["9223372036854775807"],"maybe":"-9223372036854775808","paired":["9","x"]}"#,
    ),
    (
        "SixtyLocked",
        r#"{"keyed":{},"listed":[],"paired":["0","y"]}"#,
    ),
    (
        "SixtyBeside<String>",
        r#"{"keyed":{"a":"-9223372036854775808"}}"#,
    ),
];

/// Each payload serde refuses, the type it is read as, and what serde answers, its position left
/// out.
pub const REFUSED: [(&str, &str, &str); 8] = [
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
    (
        "SixtyReached",
        r#"{"aliased":"0","fixed":"214","from":{"start":"0"},"inclusive":{"start":"0","end":"0"},"ranged":{"start":"0","end":"9223372036854775808"},"resulted":{"Ok":"7"},"to":{"end":"0"},"wrong":{"Ok":"o"}}"#,
        r#"invalid value: string "9223372036854775808", expected an i64 as a string of its decimal digits"#,
    ),
    (
        "SixtyReached",
        r#"{"aliased":"0","fixed":"215","from":{"start":"0"},"inclusive":{"start":"0","end":"0"},"ranged":{"start":"0","end":"0"},"resulted":{"Ok":"7"},"to":{"end":"0"},"wrong":{"Ok":"o"}}"#,
        "'fixed': not the literal 214, got 215",
    ),
    (
        "SixtyLocked",
        r#"{"keyed":{"4":"x"},"listed":[],"paired":["0","y"]}"#,
        r#"invalid value: string "x", expected an i64 as a string of its decimal digits"#,
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

/// An alias of an `i64`, declared above the types that hold it.
#[model_schema()]
pub type SixtyRevision = i64;

/// An `i64` through an alias, as a range's bound, as a `Result`'s arm and under a `literal`.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SixtyReached {
    pub aliased: SixtyRevision,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub aliases: Option<Vec<SixtyRevision>>,
    #[model_schema_prop(literal = 214)]
    pub fixed: i64,
    pub from: RangeFrom<i64>,
    pub inclusive: RangeInclusive<i64>,
    pub ranged: Range<i64>,
    pub resulted: Result<i64, String>,
    pub to: RangeTo<i64>,
    pub wrong: Result<String, Vec<i64>>,
}

/// An `i64` a lock guards below a map, a list, an `Option` and a tuple.
#[model_schema()]
#[derive(Debug, Deserialize, Serialize)]
pub struct SixtyLocked {
    pub keyed: BTreeMap<u32, Mutex<i64>>,
    pub listed: Vec<RefCell<i64>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub maybe: Option<RwLock<i64>>,
    pub paired: (RefCell<i64>, String),
}

/// An `i64` beside the type's own parameter, in a map keyed by it.
#[model_schema(default_types(Key = String))]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SixtyBeside<Key: Ord> {
    pub keyed: BTreeMap<Key, i64>,
}

/// What serde writes back of `json` read as the type `read_as` names.
pub fn serde_written(read_as: &str, json: &str) -> String {
    match read_as {
        "SixtyTally" => written_back::<SixtyTally>(json),
        "SixtyId" => written_back::<SixtyId>(json),
        "SixtyEvent" => written_back::<SixtyEvent>(json),
        "SixtyReached" => written_back::<SixtyReached>(json),
        "SixtyLocked" => written_back::<SixtyLocked>(json),
        "SixtyBeside<String>" => written_back::<SixtyBeside<String>>(json),
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
        "SixtyReached" => refusal_of::<SixtyReached>(json),
        "SixtyLocked" => refusal_of::<SixtyLocked>(json),
        "SixtyBeside<String>" => refusal_of::<SixtyBeside<String>>(json),
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
