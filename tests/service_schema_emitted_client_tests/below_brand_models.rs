//! An untagged enum whose variants hold a plain enum, a bounded brand, a list of it, an optional
//! one, a model with a bounded member and a plain string, with the plain enum, the brand and the
//! model declared below the enum,
//! and what serde answers of each payload: the first variant whose payload reads, a bound that
//! refuses the payload taking the variant out of the running.

use serde::{Deserialize, Serialize};
use tixschema::model_schema;

/// The payloads read as [`Fastening`]: the texts of the plain enum's two cases and one it lacks, a
/// text and a list within the bound and past it, a null, a model within its bound and past it, and
/// a number.
pub const PAYLOADS: [&str; 11] = [
    r#""High""#,
    r#""Low""#,
    r#""Mid""#,
    r#""abc""#,
    r#""abcdef""#,
    r#"["ab","abc"]"#,
    r#"["ab","abcdef"]"#,
    "null",
    r#"{"label":"ab"}"#,
    r#"{"label":"abcdef"}"#,
    "7",
];

/// An untagged enum over a plain enum and a brand declared below it.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum Fastening {
    Anodized(Finish),
    Coded(Rivet),
    Listed(Vec<Rivet>),
    Maybe(Option<Rivet>),
    Whole(Gasket),
    Wordy(String),
}

/// A plain enum.
#[model_schema()]
#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
pub enum Finish {
    High,
    Low,
}

/// A brand under a bound of its own.
#[model_schema(maxLength = 4)]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(transparent)]
pub struct Rivet(pub String);

/// A model with a bounded member.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Gasket {
    #[model_schema_prop(maxLength = 4)]
    pub label: String,
}

/// What the Rust read of `payload` as [`Fastening`] takes: the name of its variant, or `refused`.
pub fn rust_takes(payload: &str) -> String {
    serde_json::from_str::<Fastening>(payload).map_or_else(
        |_| "refused".to_owned(),
        |held| {
            format!("{held:?}")
                .split(|written: char| !written.is_alphanumeric())
                .next()
                .unwrap_or_default()
                .to_owned()
        },
    )
}
