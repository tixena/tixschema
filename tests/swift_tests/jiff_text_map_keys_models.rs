//! The models of `swift_tests/text_map_keys.rs` and of
//! `service_schema_emitted_client_tests/run_swift_text_map_keys.rs`: a map under each key Swift
//! holds as the text serde writes for it.

use core::hash::Hash;
use std::collections::HashMap;

use jiff::civil;
use jiff::civil::{Date, DateTime, Time};
use serde::{Deserialize, Serialize};
use tixschema::model_schema;

/// A model with a rule, the value of a map under a date.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Moorage {
    #[model_schema_prop(minLength = 1)]
    pub label: String,
}

/// A map under each key Swift holds as its text, and one whose value is a model.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Almanac {
    pub berths: HashMap<Date, Moorage>,
    pub days: HashMap<Date, String>,
    pub hours: HashMap<Time, String>,
    pub moments: HashMap<DateTime, String>,
}

/// [`Almanac`] in a type that declares a parameter.
#[model_schema(default_types(Stowed = String))]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Ephemeris<Stowed> {
    pub days: HashMap<Date, Stowed>,
    pub hours: HashMap<Time, String>,
    pub moments: HashMap<DateTime, Stowed>,
}

/// The parameter as the key of a map.
#[model_schema(default_types(Stowed = String))]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Gazetteer<Stowed: Eq + Hash> {
    pub by_stowed: HashMap<Stowed, u32>,
}

/// A variant with named members that holds a map under each key, under no tag attribute.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum Tidings {
    Dated {
        days: HashMap<Date, String>,
        hours: HashMap<Time, String>,
        moments: HashMap<DateTime, String>,
    },
    Plain {
        note: String,
    },
}

/// [`Tidings`] under a tag alone.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "kind")]
pub enum TaggedTidings {
    Dated {
        days: HashMap<Date, String>,
        hours: HashMap<Time, String>,
        moments: HashMap<DateTime, String>,
    },
    Plain {
        note: String,
    },
}

/// [`Tidings`] under a tag and a content key.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "kind", content = "body")]
pub enum AdjacentTidings {
    Dated {
        days: HashMap<Date, String>,
        hours: HashMap<Time, String>,
        moments: HashMap<DateTime, String>,
    },
    Plain {
        note: String,
    },
}

/// [`Tidings`] untagged.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum UntaggedTidings {
    Dated {
        days: HashMap<Date, String>,
        hours: HashMap<Time, String>,
        moments: HashMap<DateTime, String>,
    },
    Plain {
        note: String,
    },
}

/// [`Tidings`] in a type that declares a parameter.
#[model_schema(default_types(Stowed = String))]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum Bulletin<Stowed> {
    Dated {
        days: HashMap<Date, Stowed>,
        hours: HashMap<Time, String>,
        moments: HashMap<DateTime, Stowed>,
    },
    Plain {
        note: Stowed,
    },
}

/// [`Bulletin`] under a tag alone.
#[model_schema(default_types(Stowed = String))]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "kind")]
pub enum TaggedBulletin<Stowed> {
    Dated {
        days: HashMap<Date, Stowed>,
        hours: HashMap<Time, String>,
        moments: HashMap<DateTime, Stowed>,
    },
    Plain {
        note: Stowed,
    },
}

/// [`Bulletin`] under a tag and a content key.
#[model_schema(default_types(Stowed = String))]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "kind", content = "body")]
pub enum AdjacentBulletin<Stowed> {
    Dated {
        days: HashMap<Date, Stowed>,
        hours: HashMap<Time, String>,
        moments: HashMap<DateTime, Stowed>,
    },
    Plain {
        note: Stowed,
    },
}

/// [`Bulletin`] untagged.
#[model_schema(default_types(Stowed = String))]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum UntaggedBulletin<Stowed> {
    Dated {
        days: HashMap<Date, Stowed>,
        hours: HashMap<Time, String>,
        moments: HashMap<DateTime, Stowed>,
    },
    Plain {
        note: Stowed,
    },
}

/// A brand over a map under a date.
#[model_schema(no_display)]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(transparent)]
pub struct Datebook(HashMap<Date, String>);

/// A brand with a named member over a map under a time.
#[model_schema(no_display)]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(transparent)]
pub struct Hourbook {
    hours: HashMap<Time, String>,
}

/// A brand over a map of the parameter under a date and time.
#[model_schema(default_types(Stowed = String))]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(transparent)]
pub struct Logsheet<Stowed>(HashMap<DateTime, Stowed>);

/// The date every map of these models is keyed by.
pub const fn day() -> Date {
    civil::date(2024, 1, 2)
}

/// The time every map of these models is keyed by.
pub const fn hour() -> Time {
    civil::time(3, 4, 5, 678 * 1_000_000)
}

/// The date and time every map of these models is keyed by.
pub const fn moment() -> DateTime {
    day().to_datetime(hour())
}
