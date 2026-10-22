//! `()` held in each place a value is: a field, a brand's slot, a list's item, a tuple's position,
//! an `Option`, and a variant's one value with no tag attribute, under a tag and a content key and
//! in an untagged enum, beside a tuple struct of no slot, which is no `()` and which serde writes as `[]`. Each client
//! reads each payload serde writes and writes it back as serde does, and refuses each payload
//! serde refuses.

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use tixschema::model_schema;

/// Each payload serde writes, and the type it is read as.
pub const WRITTEN: [(&str, &str); 9] = [
    (
        "ProbeUnits",
        r#"{"listed":[null],"nothing":null,"paired":[1,null],"unit":null}"#,
    ),
    ("ProbeNothing", "null"),
    ("ProbeMaybeUnit", r#"{"maybe":null}"#),
    ("ProbeMaybeUnit", "{}"),
    ("ProbeUnitChoice", r#"{"Nothing":null}"#),
    ("ProbeUnitAdjacent", r#"{"kind":"Nothing","body":null}"#),
    ("ProbeUnitEitherHolder", r#"{"either":null}"#),
    ("ProbeUnitEitherHolder", r#"{"either":1}"#),
    ("ProbeSlotless", "[]"),
];

/// Each payload serde refuses, the type it is read as, and what serde answers, its position left
/// out: `[]` and every other value in place of `()`, an absent key `()` is read from, and a tuple
/// struct of no slot read from `null`.
pub const REFUSED: [(&str, &str, &str); 14] = [
    (
        "ProbeUnits",
        r#"{"listed":[[]],"nothing":null,"paired":[1,null],"unit":null}"#,
        "invalid type: sequence, expected unit",
    ),
    (
        "ProbeUnits",
        r#"{"listed":[null],"nothing":[],"paired":[1,null],"unit":null}"#,
        "invalid type: sequence, expected unit",
    ),
    (
        "ProbeUnits",
        r#"{"listed":[null],"nothing":null,"paired":[1,[]],"unit":null}"#,
        "invalid type: sequence, expected unit",
    ),
    (
        "ProbeUnits",
        r#"{"listed":[null],"nothing":null,"paired":[1,null],"unit":[]}"#,
        "invalid type: sequence, expected unit",
    ),
    (
        "ProbeUnits",
        r#"{"listed":[null],"nothing":null,"paired":[1,null],"unit":{}}"#,
        "invalid type: map, expected unit",
    ),
    (
        "ProbeUnits",
        r#"{"listed":[null],"nothing":null,"paired":[1,null],"unit":"x"}"#,
        "invalid type: string \"x\", expected unit",
    ),
    (
        "ProbeUnits",
        r#"{"listed":[null],"nothing":null,"paired":[1,null]}"#,
        "missing field `unit`",
    ),
    (
        "ProbeUnits",
        r#"{"listed":[null],"paired":[1,null],"unit":null}"#,
        "missing field `nothing`",
    ),
    (
        "ProbeNothing",
        "[]",
        "invalid type: sequence, expected unit",
    ),
    (
        "ProbeMaybeUnit",
        r#"{"maybe":[]}"#,
        "invalid type: sequence, expected unit",
    ),
    (
        "ProbeUnitChoice",
        r#"{"Nothing":[]}"#,
        "invalid type: sequence, expected unit",
    ),
    (
        "ProbeUnitAdjacent",
        r#"{"kind":"Nothing"}"#,
        "missing field `body`",
    ),
    ("ProbeUnitEitherHolder", "{}", "missing field `either`"),
    (
        "ProbeSlotless",
        "null",
        "invalid type: null, expected tuple struct ProbeSlotless",
    ),
];

/// A brand over `()`, which serde writes as `null`.
#[model_schema(no_display)]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(transparent)]
pub struct ProbeNothing(());

/// `()` as a field, a list's item and a tuple's position, beside a brand over it.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ProbeUnits {
    pub listed: Vec<()>,
    pub nothing: ProbeNothing,
    pub paired: (u32, ()),
    pub unit: (),
}

/// An `Option` of `()`: serde writes `Some(())` as `null`, and reads `null` back as `None`.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ProbeMaybeUnit {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub maybe: Option<()>,
}

/// `()` as a variant's one value, with no tag attribute.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum ProbeUnitChoice {
    Count(u32),
    Nothing(()),
}

/// `()` as a variant's one value under a tag and a content key, which serde requires.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", content = "body")]
pub enum ProbeUnitAdjacent {
    Count(u32),
    Nothing(()),
}

/// `()` as a member of an untagged enum, which serde writes as `null`.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(untagged)]
pub enum ProbeUnitEither {
    Count(u32),
    Nothing(()),
}

/// An untagged enum with a member over `()`, under a key serde requires.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ProbeUnitEitherHolder {
    pub either: ProbeUnitEither,
}

/// A tuple struct whose every slot serde skips, which it writes as `[]`.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ProbeSlotless(#[serde(skip)] pub u8, #[serde(skip)] pub u8);

/// What serde writes back of `json` read as the type `read_as` names.
pub fn serde_written(read_as: &str, json: &str) -> String {
    match read_as {
        "ProbeUnits" => written_back::<ProbeUnits>(json),
        "ProbeNothing" => written_back::<ProbeNothing>(json),
        "ProbeMaybeUnit" => written_back::<ProbeMaybeUnit>(json),
        "ProbeUnitChoice" => written_back::<ProbeUnitChoice>(json),
        "ProbeUnitAdjacent" => written_back::<ProbeUnitAdjacent>(json),
        "ProbeUnitEitherHolder" => written_back::<ProbeUnitEitherHolder>(json),
        _ => written_back::<ProbeSlotless>(json),
    }
}

/// What serde answers of `json` read as the type `read_as` names, its position left out, or
/// `None` where it reads it.
pub fn serde_refusal(read_as: &str, json: &str) -> Option<String> {
    match read_as {
        "ProbeUnits" => refusal_of::<ProbeUnits>(json),
        "ProbeNothing" => refusal_of::<ProbeNothing>(json),
        "ProbeMaybeUnit" => refusal_of::<ProbeMaybeUnit>(json),
        "ProbeUnitChoice" => refusal_of::<ProbeUnitChoice>(json),
        "ProbeUnitAdjacent" => refusal_of::<ProbeUnitAdjacent>(json),
        "ProbeUnitEitherHolder" => refusal_of::<ProbeUnitEitherHolder>(json),
        _ => refusal_of::<ProbeSlotless>(json),
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

/// serde reads each payload of [`WRITTEN`] and writes it back, and refuses each of [`REFUSED`] in
/// the words beside it.
#[test]
fn serde_reads_and_refuses_each_payload_as_the_rows_state() {
    for (read_as, json) in WRITTEN {
        assert_eq!(serde_refusal(read_as, json), None, "serde refuses {json}");
        assert!(
            !serde_written(read_as, json).is_empty(),
            "serde writes nothing back of {json}"
        );
    }
    for (read_as, json, said) in REFUSED {
        assert_eq!(
            serde_refusal(read_as, json).as_deref(),
            Some(said),
            "serde no longer refuses {json} as the row states"
        );
    }
}
