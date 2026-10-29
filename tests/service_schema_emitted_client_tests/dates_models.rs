//! The models of the date runs of the emitted clients (`run_{dart,kotlin,swift}_dates.rs`): a date
//! in a field and as a map key, in a list, as a map value, in a tuple slot and in a variant's
//! slot, and an optional date under `as_number`. `SAME_AS_SERDE` holds the payloads each client
//! writes back as serde writes them.

use alloc::collections::BTreeMap;

use chrono::{DateTime, Utc};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use tixschema::model_schema;

/// The payloads every client reads and writes back as serde writes them: a name, the model that
/// reads it and the payload.
pub const SAME_AS_SERDE: [(&str, &str, &str); 9] = [
    (
        "whole seconds, in a field and as a key",
        "Landing",
        r#"{"at":"2025-09-16T05:20:00Z","by_at":{"2025-09-16T05:20:00Z":1}}"#,
    ),
    (
        "milliseconds, in a field and as a key",
        "Landing",
        r#"{"at":"2025-09-16T05:20:00.500Z","by_at":{"2025-09-16T05:20:00.500Z":1}}"#,
    ),
    (
        "whole seconds, as a map value, in a list and in a tuple slot",
        "Positions",
        r#"{"by_name":{"a":"2025-09-16T05:20:00Z"},"listed":["2025-09-16T05:20:00Z"],"pair":["2025-09-16T05:20:00Z",1]}"#,
    ),
    (
        "milliseconds, as a map value, in a list and in a tuple slot",
        "Positions",
        r#"{"by_name":{"a":"2025-09-16T05:20:00.500Z"},"listed":["2025-09-16T05:20:00.500Z"],"pair":["2025-09-16T05:20:00.500Z",1]}"#,
    ),
    (
        "whole seconds, in a variant slot",
        "Stamped",
        r#"{"At":"2025-09-16T05:20:00Z"}"#,
    ),
    (
        "milliseconds, in a variant slot",
        "Stamped",
        r#"{"At":"2025-09-16T05:20:00.500Z"}"#,
    ),
    (
        "an optional date under as_number",
        "Nudge",
        r#"{"due":1758000000000}"#,
    ),
    (
        "an optional date under as_number, null",
        "Nudge",
        r#"{"due":null}"#,
    ),
    ("an optional date under as_number, absent", "Nudge", "{}"),
];

/// A date, and a map keyed by dates.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Landing {
    pub at: DateTime<Utc>,
    pub by_at: BTreeMap<DateTime<Utc>, u32>,
}

/// An optional date under `as_number`.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Nudge {
    #[model_schema_prop(as_number)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub due: Option<DateTime<Utc>>,
}

/// A date as a map value, in a list and in a tuple slot.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Positions {
    pub by_name: BTreeMap<String, DateTime<Utc>>,
    pub listed: Vec<DateTime<Utc>>,
    pub pair: (DateTime<Utc>, u32),
}

/// A date in a variant slot.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum Stamped {
    At(DateTime<Utc>),
    Note(String),
}

/// What serde writes back for `payload` read as `model`.
pub fn serde_back(model: &str, payload: &str) -> String {
    fn back<Held>(payload: &str) -> String
    where
        Held: DeserializeOwned + Serialize,
    {
        serde_json::to_string(&serde_json::from_str::<Held>(payload).unwrap()).unwrap()
    }
    match model {
        "Landing" => back::<Landing>(payload),
        "Nudge" => back::<Nudge>(payload),
        "Positions" => back::<Positions>(payload),
        "Stamped" => back::<Stamped>(payload),
        other => format!("no model named {other}"),
    }
}
