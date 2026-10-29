//! The models of `swift_tests/held_maps.rs` and of
//! `service_schema_emitted_client_tests/run_swift_held_maps.rs`: a map under a key Swift wraps, and
//! a date, held as a type argument and named by an alias, in a type that declares a parameter and
//! in one that declares none.

use std::collections::HashMap;

#[cfg(feature = "chrono")]
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tixschema::model_schema;

/// A plain enum a map is keyed by.
#[model_schema()]
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub enum Currents {
    East,
    West,
}

/// What a map fills as a type argument.
#[model_schema(default_types(Held = String))]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Rucksack<Held> {
    pub held: Held,
}

/// A map under each key Swift wraps, and one it keys natively, as a type argument.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Haversack {
    pub by_current: Rucksack<HashMap<Currents, String>>,
    pub by_flag: Rucksack<HashMap<bool, String>>,
    pub by_name: Rucksack<HashMap<String, String>>,
    pub by_number: Rucksack<HashMap<u32, String>>,
    pub listed: Rucksack<Vec<HashMap<u32, String>>>,
    pub maybe: Rucksack<Option<HashMap<u32, String>>>,
}

/// [`Haversack`] in a type that declares a parameter.
#[model_schema(default_types(Stowed = String))]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct StowedHaversack<Stowed> {
    pub by_number: Rucksack<HashMap<u32, Stowed>>,
    pub held: Stowed,
}

/// An alias of a map keyed by a number, with a type parameter.
#[model_schema(default_types(Stowed = String))]
pub type Tabulation<Stowed> = HashMap<u32, Stowed>;

/// An alias of a map keyed by a plain enum.
#[model_schema()]
pub type Sequencing = HashMap<Currents, String>;

/// An alias of a map keyed by a flag.
#[model_schema()]
pub type Beaconing = HashMap<bool, String>;

/// An alias of a map keyed by a string.
#[model_schema()]
pub type Captioning = HashMap<String, String>;

/// An alias of a date.
#[cfg(feature = "chrono")]
#[model_schema()]
pub type Moonrise = DateTime<Utc>;

/// A model with a rule, the value of a map an alias names.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Leadline {
    #[model_schema_prop(minLength = 1)]
    pub label: String,
}

/// An alias of a map keyed by a number whose values `validate()` walks.
#[model_schema()]
pub type Soundings = HashMap<u32, Leadline>;

/// A field typed with an alias of a map whose values `validate()` walks, and a type argument that
/// fills with one.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Plumbline {
    pub argument: Rucksack<HashMap<u32, Leadline>>,
    pub sounded: Soundings,
}

/// A field typed with each alias.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Pocketing {
    pub beaconing: Beaconing,
    pub captioning: Captioning,
    #[cfg(feature = "chrono")]
    pub moonrise: Moonrise,
    pub sequencing: Sequencing,
    pub tabulation: Tabulation<String>,
}

/// A field typed with an alias that declares a parameter, in a type that declares one.
#[model_schema(default_types(Stowed = String))]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct StowedPocketing<Stowed> {
    pub tabulation: Tabulation<Stowed>,
}

/// A date with a fraction of whole milliseconds, which chrono writes with three digits.
#[cfg(feature = "chrono")]
pub const fn at() -> DateTime<Utc> {
    DateTime::from_timestamp_millis(1_704_164_645_678).unwrap()
}
