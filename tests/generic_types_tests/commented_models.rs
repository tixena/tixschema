//! Models whose names on the wire and doc comments hold the end of a comment: a struct's members,
//! with and without a doc comment and a bound, an item's own doc, a plain enum's values and the
//! doc of one, an enum's variants and their members under a tag and under none, a brand and an
//! alias.

use serde::{Deserialize, Serialize};
use tixschema::model_schema;

/// A struct whose members are written under names that hold the end of a comment.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Glossed {
    /// A member doc that holds */ the end of a comment, beside a bound.
    #[serde(rename = "c*/d")]
    #[model_schema_prop(minLength = 1)]
    pub bounded: String,
    #[serde(rename = "a*/b")]
    pub closing: String,
}

/// An item doc that holds */ the end of a comment.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Remarked {
    /// A member doc that holds */ the end of a comment.
    pub note: String,
}

/// A plain enum doc that holds */ the end of a comment.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum Footnoted {
    #[serde(rename = "*/")]
    Bare,
    /// A variant doc that holds */ the end of a comment.
    #[serde(rename = "x*/y")]
    Closing,
}

/// An enum under a tag whose name, variant and variant's member hold the end of a comment.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "k*/ind")]
pub enum Marginal {
    #[serde(rename = "p*/q")]
    Noted {
        #[serde(rename = "r*/s")]
        remark: String,
    },
    /// A variant doc that holds */ the end of a comment.
    Plain { line: String },
}

/// An externally tagged enum whose variants and variant's member hold the end of a comment.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum Rubric {
    #[serde(rename = "*/x")]
    Held(String),
    #[serde(rename = "t*/u")]
    Ruled {
        #[serde(rename = "v*/w")]
        mark: String,
    },
}

/// A brand whose doc holds */ the end of a comment.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(transparent)]
pub struct Scholion(pub String);

/// An alias whose doc holds */ the end of a comment.
#[model_schema()]
pub type Marginalia = Remarked;

/// What serde writes of a value of each model, beside the name of the schema it is read through.
pub fn written() -> Vec<(&'static str, String)> {
    vec![
        (
            "Glossed",
            json(&Glossed {
                bounded: "b".to_owned(),
                closing: "c".to_owned(),
            }),
        ),
        (
            "Remarked",
            json(&Remarked {
                note: "n".to_owned(),
            }),
        ),
        ("Footnoted", json(&Footnoted::Bare)),
        ("Footnoted", json(&Footnoted::Closing)),
        (
            "Marginal",
            json(&Marginal::Noted {
                remark: "r".to_owned(),
            }),
        ),
        (
            "Marginal",
            json(&Marginal::Plain {
                line: "l".to_owned(),
            }),
        ),
        ("Rubric", json(&Rubric::Held("h".to_owned()))),
        (
            "Rubric",
            json(&Rubric::Ruled {
                mark: "m".to_owned(),
            }),
        ),
        ("Scholion", json(&Scholion("s".to_owned()))),
        (
            "MarginaliaType",
            json::<Marginalia>(&Remarked {
                note: "a".to_owned(),
            }),
        ),
    ]
}

/// What serde writes of `value`.
fn json<T>(value: &T) -> String
where
    T: Serialize,
{
    serde_json::to_string(value).unwrap()
}
