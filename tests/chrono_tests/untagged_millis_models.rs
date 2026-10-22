//! A date under `as_number` in a member of an untagged enum: in a newtype member and a named
//! member, under an `Option` too. serde writes and reads it as the integer of its epoch
//! milliseconds, through the hook `as_number` hangs on the member, which every surface describes.
//! `chrono_tests/tests.rs` holds serde and the JSON Schema to [`MILLIS_ROWS`],
//! `generic_types_tests/declarations.rs` the TypeScript type and the Zod schema, and
//! `service_schema_emitted_client_tests/run_untagged_millis.rs` each client.

use chrono::{DateTime, Utc};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use tixschema::model_schema;

/// Each payload: the enum it is read as, the payload, what serde and each client write back of it
/// (`None` where each refuses it), and what the Zod schema parses it to (`None` where it refuses
/// it). The Zod coercer of a date under `as_number` reads a date's text as the epoch milliseconds
/// it stands for, which serde refuses.
pub const MILLIS_ROWS: [(&str, &str, Option<&str>, Option<&str>); 12] = [
    (
        "MillisOrFlag",
        "1704164645678",
        Some("1704164645678"),
        Some("1704164645678"),
    ),
    ("MillisOrFlag", "true", Some("true"), Some("true")),
    (
        "MillisOrFlag",
        r#""2024-01-02T03:04:05.678Z""#,
        None,
        Some("1704164645678"),
    ),
    ("MillisOrFlag", "1.5", None, None),
    (
        "MillisNamed",
        r#"{"at":1704164645678}"#,
        Some(r#"{"at":1704164645678}"#),
        Some(r#"{"at":1704164645678}"#),
    ),
    (
        "MillisNamed",
        r#"{"at":"2024-01-02T03:04:05.678Z"}"#,
        None,
        Some(r#"{"at":1704164645678}"#),
    ),
    (
        "MillisMaybe",
        "1704164645678",
        Some("1704164645678"),
        Some("1704164645678"),
    ),
    (
        "MillisMaybe",
        r#""2024-01-02T03:04:05.678Z""#,
        None,
        Some("1704164645678"),
    ),
    (
        "MillisLater",
        r#"{"later":1704164645678,"settled":null}"#,
        Some(r#"{"later":1704164645678,"settled":null}"#),
        Some(r#"{"later":1704164645678,"settled":null}"#),
    ),
    (
        "MillisLater",
        r#"{"settled":1704164645678}"#,
        Some(r#"{"settled":1704164645678}"#),
        Some(r#"{"settled":1704164645678}"#),
    ),
    (
        "MillisLater",
        r#"{"later":1704164645678,"settled":1704164645678}"#,
        Some(r#"{"later":1704164645678,"settled":1704164645678}"#),
        Some(r#"{"later":1704164645678,"settled":1704164645678}"#),
    ),
    (
        "MillisLater",
        r#"{"settled":"2024-01-02T03:04:05.678Z"}"#,
        None,
        Some(r#"{"settled":1704164645678}"#),
    ),
];

/// A date under `as_number` in a newtype member.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(untagged)]
pub enum MillisOrFlag {
    Flag(bool),
    Millis(#[model_schema_prop(as_number)] DateTime<Utc>),
}

/// A date under `as_number` in a named member.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(untagged)]
pub enum MillisNamed {
    Flag(bool),
    Logged {
        #[model_schema_prop(as_number)]
        at: DateTime<Utc>,
    },
}

/// A date under `as_number` in an `Option` of a newtype member.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(untagged)]
pub enum MillisMaybe {
    Flag(bool),
    Millis(#[model_schema_prop(as_number)] Option<DateTime<Utc>>),
}

/// A date under `as_number` in an `Option` of a named member that drops its key, and in a
/// `nullable` one.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(untagged)]
pub enum MillisLater {
    Flag(bool),
    Logged {
        #[model_schema_prop(as_number)]
        #[serde(default, skip_serializing_if = "Option::is_none")]
        later: Option<DateTime<Utc>>,
        #[model_schema_prop(as_number, nullable)]
        settled: Option<DateTime<Utc>>,
    },
}

/// What serde writes back of `json` read as the enum `read_as` names, `None` where it refuses it.
pub fn serde_written(read_as: &str, json: &str) -> Option<String> {
    match read_as {
        "MillisOrFlag" => written_back::<MillisOrFlag>(json),
        "MillisNamed" => written_back::<MillisNamed>(json),
        "MillisMaybe" => written_back::<MillisMaybe>(json),
        _ => written_back::<MillisLater>(json),
    }
}

/// What serde writes back of `json` read as `T`, `None` where it refuses it.
fn written_back<T>(json: &str) -> Option<String>
where
    T: DeserializeOwned + Serialize,
{
    serde_json::from_str::<T>(json)
        .ok()
        .map(|read| serde_json::to_string(&read).unwrap())
}
