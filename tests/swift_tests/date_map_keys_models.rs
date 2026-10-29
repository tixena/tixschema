//! The models of `swift_tests/date_map_keys.rs` and of
//! `service_schema_emitted_client_tests/run_swift_date_map_keys.rs`: a map keyed by a date, as a
//! field, as a member of a variant under each tagging and as a brand's value, and in a list, in a
//! list of optionals, in an optional list and in an optional.

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tixschema::model_schema;

/// A model with a rule, the value of a map under a date.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Sailing {
    #[model_schema_prop(minLength = 1)]
    pub label: String,
}

/// A map keyed by a date, and one whose value is a model.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Timetable {
    pub by_at: HashMap<DateTime<Utc>, u8>,
    pub sailings: HashMap<DateTime<Utc>, Sailing>,
}

/// A map keyed by a date in a type that declares a parameter.
#[model_schema(default_types(Stowed = String))]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Tidetable<Stowed> {
    pub by_at: HashMap<DateTime<Utc>, Stowed>,
    pub counts: HashMap<DateTime<Utc>, u8>,
}

/// A variant with named members that holds a map keyed by a date, under no tag attribute.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum Watch {
    Logged {
        by_at: HashMap<DateTime<Utc>, u8>,
        sailings: HashMap<DateTime<Utc>, Sailing>,
    },
    Plain {
        note: String,
    },
}

/// [`Watch`] under a tag alone.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "kind")]
pub enum TaggedWatch {
    Logged {
        by_at: HashMap<DateTime<Utc>, u8>,
        sailings: HashMap<DateTime<Utc>, Sailing>,
    },
    Plain {
        note: String,
    },
}

/// [`Watch`] under a tag and a content key.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "kind", content = "body")]
pub enum AdjacentWatch {
    Logged {
        by_at: HashMap<DateTime<Utc>, u8>,
        sailings: HashMap<DateTime<Utc>, Sailing>,
    },
    Plain {
        note: String,
    },
}

/// [`Watch`] untagged.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum UntaggedWatch {
    Logged {
        by_at: HashMap<DateTime<Utc>, u8>,
        sailings: HashMap<DateTime<Utc>, Sailing>,
    },
    Plain {
        note: String,
    },
}

/// [`Watch`] in a type that declares a parameter.
#[model_schema(default_types(Stowed = String))]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum Shift<Stowed> {
    Logged {
        by_at: HashMap<DateTime<Utc>, Stowed>,
        counts: HashMap<DateTime<Utc>, u8>,
    },
    Plain {
        note: Stowed,
    },
}

/// [`Shift`] under a tag alone.
#[model_schema(default_types(Stowed = String))]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "kind")]
pub enum TaggedShift<Stowed> {
    Logged {
        by_at: HashMap<DateTime<Utc>, Stowed>,
        counts: HashMap<DateTime<Utc>, u8>,
    },
    Plain {
        note: Stowed,
    },
}

/// [`Shift`] under a tag and a content key.
#[model_schema(default_types(Stowed = String))]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "kind", content = "body")]
pub enum AdjacentShift<Stowed> {
    Logged {
        by_at: HashMap<DateTime<Utc>, Stowed>,
        counts: HashMap<DateTime<Utc>, u8>,
    },
    Plain {
        note: Stowed,
    },
}

/// [`Shift`] untagged.
#[model_schema(default_types(Stowed = String))]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum UntaggedShift<Stowed> {
    Logged {
        by_at: HashMap<DateTime<Utc>, Stowed>,
        counts: HashMap<DateTime<Utc>, u8>,
    },
    Plain {
        note: Stowed,
    },
}

/// A brand over a map keyed by a date.
#[model_schema(no_display)]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(transparent)]
pub struct Daybook(pub HashMap<DateTime<Utc>, String>);

/// A brand with a named member over a map keyed by a date.
#[model_schema(no_display)]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(transparent)]
pub struct Chronicle {
    pub by_at: HashMap<DateTime<Utc>, u8>,
}

/// A brand over a map of the parameter keyed by a date.
#[model_schema(default_types(Stowed = String))]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(transparent)]
pub struct Diary<Stowed>(pub HashMap<DateTime<Utc>, Stowed>);

/// A map keyed by a date in a list, in a list of optionals, in an optional list and in an
/// optional, and in a list whose maps hold a model.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Logbook {
    pub listed: Vec<HashMap<DateTime<Utc>, u8>>,
    pub listed_or_none: Vec<Option<HashMap<DateTime<Utc>, u8>>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub maybe: Option<HashMap<DateTime<Utc>, u8>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub maybe_listed: Option<Vec<HashMap<DateTime<Utc>, u8>>>,
    pub sailing_lists: Vec<HashMap<DateTime<Utc>, Sailing>>,
}

/// [`Logbook`] in a type that declares a parameter.
#[model_schema(default_types(Stowed = String))]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct StowedLogbook<Stowed> {
    pub listed: Vec<HashMap<DateTime<Utc>, Stowed>>,
    pub listed_or_none: Vec<Option<HashMap<DateTime<Utc>, Stowed>>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub maybe: Option<HashMap<DateTime<Utc>, Stowed>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub maybe_listed: Option<Vec<HashMap<DateTime<Utc>, Stowed>>>,
}

/// A brand over a list of maps keyed by a date.
#[model_schema(no_display)]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(transparent)]
pub struct Daybooks(pub Vec<HashMap<DateTime<Utc>, String>>);

/// A date, held beside a map keyed by dates.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Moment {
    pub at: DateTime<Utc>,
}

/// An optional date under `as_number`.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Reminder {
    #[model_schema_prop(as_number)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub due: Option<DateTime<Utc>>,
}

/// A map keyed by a date whose value is a model with a rule, in a type that declares a parameter.
#[model_schema(default_types(Stowed = String))]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Convoy<Stowed> {
    pub held: Stowed,
    pub sailings: HashMap<DateTime<Utc>, Sailing>,
}

/// A date with a fraction of whole milliseconds, which chrono writes with three digits.
pub const fn first() -> DateTime<Utc> {
    DateTime::from_timestamp_millis(1_704_164_645_678).unwrap()
}

/// A second such date, whose fraction ends on zeros chrono writes.
pub const fn second() -> DateTime<Utc> {
    DateTime::from_timestamp_millis(1_709_208_000_100).unwrap()
}
