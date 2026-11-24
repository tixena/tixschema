//! The query types `#[model_schema(enable = ["decode_with", "mongodb"])]` adds under the
//! `mongodb` feature: `Filter`, `Update`, and the typed paths that build them, built against
//! version 2 of the `bson` library.

extern crate bson2 as bson;

tixschema::decode_with_foundation!();
tixschema::mongodb_foundation!();

#[cfg(test)]
#[cfg(all(any(feature = "chrono", feature = "jiff"), feature = "mongodb"))]
#[path = "mongodb_query_tests/tests.rs"]
mod tests;

// Outside `cfg(test)`, where clippy's `large_stack_frames` reports what a type generates.
#[cfg(all(any(feature = "chrono", feature = "jiff"), feature = "mongodb"))]
#[path = "mongodb_query_tests/wide.rs"]
mod wide;

// Version 2's hooks that store a chrono date as a BSON date, over a date and over an `Option` of
// one, under the names the shared sources write as a field's `with`.
#[cfg(test)]
#[cfg(all(feature = "chrono", feature = "mongodb"))]
use bson::serde_helpers::{
    chrono_datetime_as_bson_datetime as date_hook,
    chrono_datetime_as_bson_datetime_optional as optional_date_hook,
};

/// A jiff `Timestamp` stored as a BSON date, which version 2 has no hook for, under the name the
/// shared sources write as a field's `with`.
#[cfg(test)]
#[cfg(all(feature = "jiff", feature = "mongodb"))]
mod date_hook {
    use jiff::Timestamp;
    use serde::de::Error as _;
    use serde::{Deserialize as _, Deserializer, Serialize as _, Serializer};

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Timestamp, D::Error>
    where
        D: Deserializer<'de>,
    {
        let stored = bson::DateTime::deserialize(deserializer)?;
        Timestamp::from_millisecond(stored.timestamp_millis()).map_err(D::Error::custom)
    }

    pub fn serialize<S>(at: &Timestamp, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        bson::DateTime::from_millis(at.as_millisecond()).serialize(serializer)
    }
}

/// That hook over an `Option` of a `Timestamp`.
#[cfg(test)]
#[cfg(all(feature = "jiff", feature = "mongodb"))]
mod optional_date_hook {
    use jiff::Timestamp;
    use serde::de::Error as _;
    use serde::{Deserialize as _, Deserializer, Serializer};

    use super::date_hook;

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<Timestamp>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let stored = Option::<bson::DateTime>::deserialize(deserializer)?;
        stored
            .map(|at| Timestamp::from_millisecond(at.timestamp_millis()).map_err(D::Error::custom))
            .transpose()
    }

    /// serde hands a hook its field by reference, here an `Option`: read as the date it may hold.
    pub fn serialize<'held, S, T>(value: &'held T, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
        &'held T: Into<Option<&'held Timestamp>>,
    {
        let held: Option<&Timestamp> = value.into();
        match held {
            Some(at) => date_hook::serialize(at, serializer),
            None => serializer.serialize_none(),
        }
    }
}
