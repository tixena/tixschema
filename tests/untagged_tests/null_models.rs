//! Untagged enums with a newtype variant over an `Option`, whose `null` serde reads as the
//! variant holding `None`: an `Option` of a number, of a number with a bound, of a model and of a
//! list, and an alias of an `Option`. serde takes the first variant whose payload reads, so `null`
//! is read as the first such variant. `untagged_tests/nulls.rs` holds serde and the JSON Schema
//! to [`NULL_ROWS`], and `service_schema_emitted_client_tests/run_untagged_nulls.rs` the Zod
//! schema and each client.

use core::fmt::Debug;

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use tixschema::model_schema;

/// Each payload, the enum it is read as, and the variant serde takes for it, `None` where serde
/// refuses it. serde writes back each payload it reads as it was.
pub const NULL_ROWS: [NullRow; 19] = [
    NullRow::taken("Fathom", "null", "Count"),
    NullRow::taken("Fathom", "3", "Count"),
    NullRow::taken("Fathom", "true", "Flag"),
    NullRow::refused("Fathom", r#""x""#),
    NullRow::refused("Fathom", "1.5"),
    NullRow::taken("Kedged", "null", "Above"),
    NullRow::taken("Kedged", "2", "Above"),
    NullRow::refused("Kedged", "0"),
    NullRow::taken("Kedged", "false", "Flag"),
    NullRow::taken("Kedged", r#""t""#, "Text"),
    NullRow::taken("Drogued", "null", "Held"),
    NullRow::taken("Drogued", r#"{"label":"a"}"#, "Held"),
    NullRow::refused("Drogued", r#"{"label":""}"#),
    NullRow::taken("Drogued", r#"[{"label":"a"}]"#, "Listed"),
    NullRow::taken("Drogued", "[]", "Listed"),
    NullRow::taken("Drogued", r#"["a","b"]"#, "Texts"),
    NullRow::taken("Plumb", "null", "Depth"),
    NullRow::taken("Plumb", "4", "Depth"),
    NullRow::taken("Plumb", "false", "Flag"),
];

/// One payload, the enum it is read as, and the variant serde takes for it.
pub struct NullRow {
    pub json: &'static str,
    pub read_as: &'static str,
    pub taken: Option<&'static str>,
}

impl NullRow {
    /// A payload serde refuses.
    const fn refused(read_as: &'static str, json: &'static str) -> Self {
        Self {
            json,
            read_as,
            taken: None,
        }
    }

    /// A payload serde reads as the variant `taken`.
    const fn taken(read_as: &'static str, json: &'static str, taken: &'static str) -> Self {
        Self {
            json,
            read_as,
            taken: Some(taken),
        }
    }

    /// What a side that agrees with serde answers of the payload: `read`, the variant taken as
    /// `named` names it, and the payload written back, or `refused`.
    pub fn told<F>(&self, named: F) -> String
    where
        F: Fn(&str) -> String,
    {
        self.taken.map_or_else(
            || "refused".to_owned(),
            |taken| format!("read {} {}", named(taken), self.json),
        )
    }
}

/// A newtype variant over an `Option` of a number, beside one over a `bool`.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(untagged)]
pub enum Fathom {
    Count(Option<u32>),
    Flag(bool),
}

/// Two variants over an `Option`, the first with a bound of its own, ahead of a text: both read
/// `null`, and the first takes it.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(untagged)]
pub enum Kedged {
    Above(#[model_schema_prop(minimum = 1)] Option<u32>),
    Flag(Option<bool>),
    Text(String),
}

/// A model with a rule.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Drogue {
    #[model_schema_prop(minLength = 1)]
    pub label: String,
}

/// Variants over an `Option` of a model, of a list of models and of a list of texts.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(untagged)]
pub enum Drogued {
    Flag(bool),
    Held(Option<Drogue>),
    Listed(Option<Vec<Drogue>>),
    Texts(Option<Vec<String>>),
}

/// An alias of an `Option`.
#[model_schema()]
pub type Sounded = Option<u32>;

/// A newtype variant over an alias of an `Option`.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(untagged)]
pub enum Plumb {
    Depth(Sounded),
    Flag(bool),
}

/// What serde answers of `json` read as the enum `read_as` names: `read`, the variant it takes and
/// the payload it writes back, or `refused`.
pub fn serde_told(read_as: &str, json: &str) -> String {
    match read_as {
        "Fathom" => told::<Fathom>(json),
        "Kedged" => told::<Kedged>(json),
        "Drogued" => told::<Drogued>(json),
        _ => told::<Plumb>(json),
    }
}

/// What serde answers of `json` read as `T`, the variant read off its `Debug`.
fn told<T>(json: &str) -> String
where
    T: Debug + DeserializeOwned + Serialize,
{
    serde_json::from_str::<T>(json).map_or_else(
        |_| "refused".to_owned(),
        |read| {
            let debugged = format!("{read:?}");
            let variant = debugged.split('(').next().unwrap_or_default();
            format!("read {variant} {}", serde_json::to_string(&read).unwrap())
        },
    )
}
