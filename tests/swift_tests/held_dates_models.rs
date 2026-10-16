//! The models of `swift_tests/held_dates.rs` and of
//! `service_schema_emitted_client_tests/run_swift_held_dates.rs`: a date in a tuple's slot, as a
//! map's value, in a variant's slots under each tagging serde accepts and as a type argument, in a
//! type that declares a parameter and in one that declares none, beside a date a field, a variant's
//! member and a brand hold.

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tixschema::model_schema;

/// What a date fills as a type argument.
#[model_schema(default_types(Held = String))]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Pannier<Held> {
    pub held: Held,
}

/// What a list of dates and an optional date fill as type arguments.
#[model_schema(default_types(First = String, Second = String))]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Pendulum<First, Second> {
    pub first: First,
    pub second: Second,
}

/// What a date fills beside the parameter of the type that holds it.
#[model_schema(default_types(First = String, Second = String))]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Chime<First, Second> {
    pub first: First,
    pub second: Second,
}

/// A date in a tuple, as a map's value and as a type argument, beside a field's own date.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PlainChart {
    pub at: DateTime<Utc>,
    pub by_name: HashMap<String, DateTime<Utc>>,
    pub by_number: HashMap<u32, DateTime<Utc>>,
    pub by_turn: HashMap<isize, DateTime<Utc>>,
    pub dated: Pannier<DateTime<Utc>>,
    pub marks_by_name: HashMap<String, Vec<DateTime<Utc>>>,
    pub maybe_by_name: HashMap<String, Option<DateTime<Utc>>>,
    pub pair: (DateTime<Utc>, u8),
    pub spans: (Vec<DateTime<Utc>>, Option<DateTime<Utc>>),
    pub swung: Pendulum<Vec<DateTime<Utc>>, Option<DateTime<Utc>>>,
}

/// [`PlainChart`] in a type that declares a parameter.
#[model_schema(default_types(Stowed = String))]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct StowedChart<Stowed> {
    pub at: DateTime<Utc>,
    pub by_name: HashMap<String, DateTime<Utc>>,
    pub by_number: HashMap<u32, DateTime<Utc>>,
    pub dated: Chime<DateTime<Utc>, Stowed>,
    pub held: Stowed,
    pub pair: (DateTime<Utc>, Stowed),
}

/// A date in a variant of several slots and in a variant of one, beside a variant's member, under
/// no tag attribute.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum PlainTide {
    Logged { at: DateTime<Utc> },
    Paired(DateTime<Utc>, u8),
    Pending(Option<DateTime<Utc>>),
    Stamped(DateTime<Utc>),
}

/// [`PlainTide`] under a tag and a content key.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "kind", content = "held")]
pub enum AdjacentPlainTide {
    Logged { at: DateTime<Utc> },
    Paired(DateTime<Utc>, u8),
    Pending(Option<DateTime<Utc>>),
    Stamped(DateTime<Utc>),
}

/// The variants an untagged enum takes: a variant's member and a date in a variant of one slot.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum UntaggedPlainTide {
    Logged { at: DateTime<Utc> },
    Stamped(DateTime<Utc>),
}

/// A tuple struct whose slot holds a date.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Tock(pub DateTime<Utc>, pub u8);

/// A brand over a date.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(transparent)]
pub struct Chimed(pub DateTime<Utc>);

/// A date under `as_number` in a variant of one slot, an optional one, and one of several slots,
/// under no tag attribute.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum CountedTide {
    Counted(#[model_schema_prop(as_number)] DateTime<Utc>),
    Maybe(#[model_schema_prop(as_number)] Option<DateTime<Utc>>),
    Spanned(#[model_schema_prop(as_number)] DateTime<Utc>, u8),
}

/// [`CountedTide`] under a tag and a content key.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "kind", content = "held")]
pub enum AdjacentCountedTide {
    Counted(#[model_schema_prop(as_number)] DateTime<Utc>),
    Maybe(#[model_schema_prop(as_number)] Option<DateTime<Utc>>),
    Spanned(#[model_schema_prop(as_number)] DateTime<Utc>, u8),
}

/// A date under `as_number` in a variant of one slot, untagged. The Swift run compiles it and reads
/// none of it: serde writes the date of an untagged variant as its text, flag or no flag.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum UntaggedCountedTide {
    Counted(#[model_schema_prop(as_number)] DateTime<Utc>),
}

/// A tuple struct whose slot holds a date under `as_number`.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Ticker(#[model_schema_prop(as_number)] pub DateTime<Utc>, pub u8);

/// [`Ticker`] beside the parameter.
#[model_schema(default_types(Stowed = String))]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct StowedTicker<Stowed>(
    #[model_schema_prop(as_number)] pub DateTime<Utc>,
    pub Stowed,
);

/// A date with a fraction of whole milliseconds, which chrono writes with three digits.
pub const fn first() -> DateTime<Utc> {
    DateTime::from_timestamp_millis(1_704_164_645_678).unwrap()
}

/// A second such date, whose fraction ends on zeros chrono writes.
pub const fn second() -> DateTime<Utc> {
    DateTime::from_timestamp_millis(1_709_208_000_100).unwrap()
}
