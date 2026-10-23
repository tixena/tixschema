//! An alias of an `Option` held outside a field, where serde writes `None` as `null`: an item of a
//! list, a value of a map, a position of a tuple, a type argument, an arm of a `Result`, a slot of
//! a tuple struct and the slot of a variant. Beside it, an `Option` written as a type argument,
//! and a field typed with the alias, which keeps the alias's name: under `default` and
//! `skip_serializing_if`, under `skip_serializing_if` alone, whose absent key serde reads as
//! `None`, as a variant's member, and under `nullable`, written in place. An alias that declares a
//! type parameter is held in a field and in a list. `semantic_types_tests/option_aliases.rs` holds
//! serde, the text of each surface and the JSON Schema to [`LEADS`],
//! `generic_types_tests/declarations.rs` the Zod schema under `node` and the TypeScript type under
//! `tsc --strict`, and `service_schema_emitted_client_tests/run_option_aliases.rs` each client.

use alloc::collections::BTreeMap;

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use tixschema::model_schema;

/// Each payload serde writes, and the type it is read as. serde reads each back and writes it as
/// it was.
pub const LEADS: [(&str, &str); 19] = [
    (
        "Leadsman",
        r#"{"keyed":{"a":null,"b":3},"listed":[null,2],"nested":[[null]],"paired":[null,1],"relisted":[null,4],"resulted":{"Ok":null},"swung":{"held":null},"swung_option":{"held":null}}"#,
    ),
    (
        "Leadsman",
        r#"{"keyed":{},"listed":[],"nested":[],"paired":[6,1],"relisted":[],"resulted":{"Ok":7},"swung":{"held":8},"swung_option":{"held":9}}"#,
    ),
    ("Handlead", r#"{"listed":[null,2]}"#),
    ("Handlead", r#"{"kept":5,"listed":[]}"#),
    ("Chanced", r#"{"listed":[null,"h"]}"#),
    ("Chanced", r#"{"kept":"a","listed":[]}"#),
    ("Shoaled", r#"{"marked":[]}"#),
    ("Shoaled", r#"{"kept":5,"marked":[null,4]}"#),
    ("Lineman", r#"{"kept":null}"#),
    ("Lineman", r#"{"kept":5}"#),
    ("Plumbline", r#"{"Line":{}}"#),
    ("Plumbline", r#"{"Line":{"kept":5}}"#),
    ("Reckoned", "[null,1]"),
    ("Reckoned", "[2,3]"),
    ("Deepsea", "null"),
    ("Deepsea", "4"),
    ("Sinker", r#"{"Depth":null}"#),
    ("Sinker", r#"{"Pair":[null,5]}"#),
    ("Heaved", r#"{"kind":"Depth","body":null}"#),
];

/// An alias of an `Option`.
#[model_schema()]
pub type Plummet = Option<u32>;

/// An alias of an alias of an `Option`.
#[model_schema()]
pub type Relead = Plummet;

/// An alias of an `Option` that declares a parameter.
#[model_schema(default_types(T = u32))]
pub type Haply<T> = Option<T>;

/// A type that holds what fills its parameter under a key serde always writes.
#[model_schema(default_types(T = u32))]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Swung<T> {
    pub held: T,
}

/// A field that holds an alias of an `Option`, beside a list of it.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Handlead {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kept: Plummet,
    pub listed: Vec<Plummet>,
}

/// Every position outside a field that holds an alias of an `Option`, and an `Option` written as a
/// type argument.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Leadsman {
    pub keyed: BTreeMap<String, Plummet>,
    pub listed: Vec<Plummet>,
    pub nested: Vec<Vec<Plummet>>,
    pub paired: (Plummet, u32),
    pub relisted: Vec<Relead>,
    pub resulted: Result<Plummet, String>,
    pub swung: Swung<Plummet>,
    pub swung_option: Swung<Option<u32>>,
}

/// A field and a list of an alias of an `Option` that declares a parameter.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Chanced {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kept: Haply<String>,
    pub listed: Vec<Haply<String>>,
}

/// A field of an alias of an `Option` under `skip_serializing_if` alone: serde reads its absent
/// key as `None`, as it reads the key of an `Option` written as one. Beside it, a list of an
/// `Option` of the alias.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Shoaled {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kept: Plummet,
    pub marked: Vec<Option<Plummet>>,
}

/// A field of an alias of an `Option` under `nullable`, whose `None` serde writes as `null`.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Lineman {
    #[model_schema_prop(nullable)]
    pub kept: Plummet,
}

/// A variant's member of an alias of an `Option` under `default` and `skip_serializing_if`.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum Plumbline {
    Line {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        kept: Plummet,
    },
}

/// A tuple struct whose first slot holds an alias of an `Option`.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Reckoned(pub Plummet, pub u32);

/// A struct of one slot over an alias of an `Option`, which serde writes as the slot's value.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Deepsea(pub Plummet);

/// Variants whose slots hold an alias of an `Option`, under no tag attribute.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum Sinker {
    Depth(Plummet),
    Pair(Plummet, u32),
}

/// A variant whose slot holds an alias of an `Option`, under a tag and a content key.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", content = "body")]
pub enum Heaved {
    Depth(Plummet),
}

/// What serde writes back of `json` read as the type `read_as` names, or `None` where it refuses
/// it.
pub fn serde_wrote(read_as: &str, json: &str) -> Option<String> {
    match read_as {
        "Leadsman" => written::<Leadsman>(json),
        "Handlead" => written::<Handlead>(json),
        "Chanced" => written::<Chanced>(json),
        "Shoaled" => written::<Shoaled>(json),
        "Lineman" => written::<Lineman>(json),
        "Plumbline" => written::<Plumbline>(json),
        "Reckoned" => written::<Reckoned>(json),
        "Deepsea" => written::<Deepsea>(json),
        "Sinker" => written::<Sinker>(json),
        _ => written::<Heaved>(json),
    }
}

/// What serde writes back of `json` read as `T`.
fn written<T>(json: &str) -> Option<String>
where
    T: DeserializeOwned + Serialize,
{
    serde_json::from_str::<T>(json)
        .ok()
        .map(|read| serde_json::to_string(&read).unwrap())
}
