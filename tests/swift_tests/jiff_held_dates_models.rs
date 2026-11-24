//! The models of `swift_tests/held_dates.rs` and of
//! `service_schema_emitted_client_tests/run_swift_held_dates.rs`: a date in a tuple's slot, as a
//! map's value, in a variant's slots under each tagging serde accepts and as a type argument, in a
//! type that declares a parameter and in one that declares none, beside a date a field, a variant's
//! member and a brand hold.

use std::collections::HashMap;

use jiff::Timestamp;
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
    pub at: Timestamp,
    pub by_name: HashMap<String, Timestamp>,
    pub by_number: HashMap<u32, Timestamp>,
    pub by_turn: HashMap<isize, Timestamp>,
    pub dated: Pannier<Timestamp>,
    pub marks_by_name: HashMap<String, Vec<Timestamp>>,
    pub maybe_by_name: HashMap<String, Option<Timestamp>>,
    pub pair: (Timestamp, u8),
    pub spans: (Vec<Timestamp>, Option<Timestamp>),
    pub swung: Pendulum<Vec<Timestamp>, Option<Timestamp>>,
}

/// What a date, a tuple and a map keyed by a number fill, in a holder of its own.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct FirstHolder {
    pub dated: Pannier<Timestamp>,
    pub numbered: Pannier<HashMap<u32, String>>,
    pub paired: Pannier<(String, u8)>,
}

/// [`FirstHolder`] beside it, which fills one generic type the same way at the same position.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SecondHolder {
    pub dated: Pannier<Timestamp>,
    pub numbered: Pannier<HashMap<u32, String>>,
    pub paired: Pannier<(String, u8)>,
}

/// One holder whose two fields fill one generic type the same way at the same position.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct TwinHolder {
    pub dated: Pannier<Timestamp>,
    pub later: Pannier<Timestamp>,
    pub numbered: Pannier<HashMap<u32, String>>,
    pub other_numbered: Pannier<HashMap<u32, String>>,
    pub other_paired: Pannier<(String, u8)>,
    pub paired: Pannier<(String, u8)>,
}

/// [`PlainChart`] in a type that declares a parameter.
#[model_schema(default_types(Stowed = String))]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct StowedChart<Stowed> {
    pub at: Timestamp,
    pub by_name: HashMap<String, Timestamp>,
    pub by_number: HashMap<u32, Timestamp>,
    pub dated: Chime<Timestamp, Stowed>,
    pub held: Stowed,
    pub pair: (Timestamp, Stowed),
}

/// A date in a variant of several slots and in a variant of one, beside a variant's member, under
/// no tag attribute.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum PlainTide {
    Logged { at: Timestamp },
    Paired(Timestamp, u8),
    Pending(Option<Timestamp>),
    Stamped(Timestamp),
}

/// [`PlainTide`] under a tag and a content key.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "kind", content = "held")]
pub enum AdjacentPlainTide {
    Logged { at: Timestamp },
    Paired(Timestamp, u8),
    Pending(Option<Timestamp>),
    Stamped(Timestamp),
}

/// The variants an untagged enum takes: a variant's member and a date in a variant of one slot.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum UntaggedPlainTide {
    Logged { at: Timestamp },
    Stamped(Timestamp),
}

/// A tuple struct whose slot holds a date.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Tock(pub Timestamp, pub u8);

/// A brand over a date.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(transparent)]
pub struct Chimed(Timestamp);

/// A date under `as_number` in a variant of one slot, an optional one, and one of several slots,
/// under no tag attribute.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum CountedTide {
    Counted(#[model_schema_prop(as_number)] Timestamp),
    Maybe(#[model_schema_prop(as_number)] Option<Timestamp>),
    Spanned(#[model_schema_prop(as_number)] Timestamp, u8),
}

/// [`CountedTide`] under a tag and a content key.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "kind", content = "held")]
pub enum AdjacentCountedTide {
    Counted(#[model_schema_prop(as_number)] Timestamp),
    Maybe(#[model_schema_prop(as_number)] Option<Timestamp>),
    Spanned(#[model_schema_prop(as_number)] Timestamp, u8),
}

/// A date under `as_number` in a variant of one slot, untagged.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum UntaggedCountedTide {
    Counted(#[model_schema_prop(as_number)] Timestamp),
}

/// A tuple struct whose slot holds a date under `as_number`.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Ticker(#[model_schema_prop(as_number)] pub Timestamp, pub u8);

/// [`Ticker`] beside the parameter.
#[model_schema(default_types(Stowed = String))]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct StowedTicker<Stowed>(#[model_schema_prop(as_number)] pub Timestamp, pub Stowed);

/// A date with a fraction of whole milliseconds, which jiff writes with three digits.
pub fn first() -> Timestamp {
    Timestamp::from_millisecond(1_704_164_645_678).unwrap()
}

/// A second such date, whose fraction ends on zeros jiff writes.
pub fn second() -> Timestamp {
    Timestamp::from_millisecond(1_709_208_000_100).unwrap()
}
