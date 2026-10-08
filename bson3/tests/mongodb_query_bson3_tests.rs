//! The query types `#[model_schema(decode_with)]` adds under `mongodb`: `Filter`, `Update`, and
//! the typed paths that build them, built against version 3 of the `bson` library, in the package
//! whose MongoDB driver is built for it.

/// Version 3's hook that stores a chrono date as a BSON date, over an `Option` of one. The
/// library reads that form through `serde_with`, which this package does not depend on, so it is
/// written here over the hook itself.
#[cfg(test)]
#[cfg(all(feature = "chrono", feature = "mongodb"))]
mod optional_date_hook {
    use chrono::{DateTime, Utc};
    use serde::{Deserialize as _, Deserializer, Serializer};

    use super::date_hook;

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<DateTime<Utc>>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let stored = Option::<bson::DateTime>::deserialize(deserializer)?;
        Ok(stored.map(bson::DateTime::to_chrono))
    }

    /// serde hands a hook its field by reference, here an `Option`: read as the date it may hold.
    pub fn serialize<'held, S, T>(value: &'held T, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
        &'held T: Into<Option<&'held DateTime<Utc>>>,
    {
        let held: Option<&DateTime<Utc>> = value.into();
        match held {
            Some(at) => date_hook::serialize(at, serializer),
            None => serializer.serialize_none(),
        }
    }
}

#[cfg(test)]
#[cfg(all(feature = "chrono", feature = "mongodb"))]
#[path = "../../tests/mongodb_query_tests/tests.rs"]
mod tests;

// Version 3's hook that stores a chrono date as a BSON date, under the name the shared sources
// write as a field's `with`.
#[cfg(test)]
#[cfg(all(feature = "chrono", feature = "mongodb"))]
use bson::serde_helpers::datetime::FromChrono04DateTime as date_hook;
