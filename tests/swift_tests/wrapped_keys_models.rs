//! The models of `swift_tests/wrapped_keys.rs` and of
//! `service_schema_emitted_client_tests/run_swift_wrapped_keys.rs`: a map keyed by a brand over a
//! string, a number, a flag and a date, by an alias of each, and by a plain enum, as a field, as a
//! member of a variant under each tagging and as a brand's value, in a type that declares a
//! parameter and in one that declares none.

use std::collections::HashMap;

#[cfg(feature = "chrono")]
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tixschema::model_schema;

/// A brand over a string a map is keyed by.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(transparent)]
pub struct Callsign(pub String);

/// A brand over a number a map is keyed by.
#[model_schema()]
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(transparent)]
pub struct Burgee(pub u32);

/// A brand over a flag a map is keyed by.
#[model_schema()]
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(transparent)]
pub struct Signal(pub bool);

/// A brand over a brand a map is keyed by.
#[model_schema()]
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(transparent)]
pub struct Lanyard(pub Burgee);

/// A brand over a date a map is keyed by.
#[cfg(feature = "chrono")]
#[model_schema()]
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(transparent)]
pub struct Postmark(pub DateTime<Utc>);

/// A brand that derives no `Hash`, which no map is keyed by.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(transparent)]
pub struct Unhashed(pub String);

/// An alias of a string a map is keyed by.
#[model_schema()]
pub type Greeting = String;

/// An alias of a number a map is keyed by.
#[model_schema()]
pub type Hull = u32;

/// An alias of a brand a map is keyed by.
#[model_schema()]
pub type Hailer = Callsign;

/// An alias of a date a map is keyed by.
#[cfg(feature = "chrono")]
#[model_schema()]
pub type PostmarkKey = DateTime<Utc>;

/// A plain enum a map is keyed by.
#[model_schema()]
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub enum Berthing {
    Port,
    Starboard,
}

/// A map under each key, as a field.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Taskforce {
    pub berthing: HashMap<Berthing, u8>,
    pub burgee: HashMap<Burgee, u8>,
    pub callsign: HashMap<Callsign, u8>,
    pub greeting: HashMap<Greeting, u8>,
    pub hailer: HashMap<Hailer, u8>,
    pub hull: HashMap<Hull, u8>,
    pub lanyard: HashMap<Lanyard, u8>,
    pub signal: HashMap<Signal, u8>,
}

/// A map under a date key, a brand over a date and an alias of one, as a field.
#[cfg(feature = "chrono")]
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Regatta {
    pub aliased: HashMap<PostmarkKey, u8>,
    pub branded: HashMap<Postmark, u8>,
}

/// [`Taskforce`] in a type that declares a parameter.
#[model_schema(default_types(Stowed = String))]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct StowedTaskforce<Stowed> {
    pub burgee: HashMap<Burgee, Stowed>,
    pub callsign: HashMap<Callsign, Stowed>,
    pub counts: HashMap<Hailer, u8>,
    pub hull: HashMap<Hull, Stowed>,
}

/// A list, an optional and a map of lists, over a brand and an alias.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Armada {
    pub listed: Vec<HashMap<Callsign, u8>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub maybe: Option<HashMap<Hull, u8>>,
    pub nested: HashMap<Hull, HashMap<Callsign, u8>>,
}

/// A variant that holds a map under a brand and under an alias, under no tag attribute.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum Squadron {
    Docked {
        callsign: HashMap<Callsign, u8>,
        hailer: HashMap<Hailer, u8>,
    },
    Idle {
        note: String,
    },
}

/// [`Squadron`] under a tag alone.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "kind")]
pub enum TaggedSquadron {
    Docked {
        callsign: HashMap<Callsign, u8>,
        hailer: HashMap<Hailer, u8>,
    },
    Idle {
        note: String,
    },
}

/// [`Squadron`] under a tag and a content key.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "kind", content = "body")]
pub enum AdjacentSquadron {
    Docked {
        callsign: HashMap<Callsign, u8>,
        hailer: HashMap<Hailer, u8>,
    },
    Idle {
        note: String,
    },
}

/// [`Squadron`] untagged.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum UntaggedSquadron {
    Docked {
        callsign: HashMap<Callsign, u8>,
        hailer: HashMap<Hailer, u8>,
    },
    Idle {
        note: String,
    },
}

/// [`Squadron`] in a type that declares a parameter.
#[model_schema(default_types(Stowed = String))]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum Convoy<Stowed> {
    Docked {
        callsign: HashMap<Callsign, Stowed>,
        hailer: HashMap<Hailer, u8>,
    },
    Idle {
        note: Stowed,
    },
}

/// [`Convoy`] under a tag alone.
#[model_schema(default_types(Stowed = String))]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "kind")]
pub enum TaggedConvoy<Stowed> {
    Docked {
        callsign: HashMap<Callsign, Stowed>,
        hailer: HashMap<Hailer, u8>,
    },
    Idle {
        note: Stowed,
    },
}

/// [`Convoy`] under a tag and a content key.
#[model_schema(default_types(Stowed = String))]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "kind", content = "body")]
pub enum AdjacentConvoy<Stowed> {
    Docked {
        callsign: HashMap<Callsign, Stowed>,
        hailer: HashMap<Hailer, u8>,
    },
    Idle {
        note: Stowed,
    },
}

/// [`Convoy`] untagged.
#[model_schema(default_types(Stowed = String))]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum UntaggedConvoy<Stowed> {
    Docked {
        callsign: HashMap<Callsign, Stowed>,
        hailer: HashMap<Hailer, u8>,
    },
    Idle {
        note: Stowed,
    },
}

/// A brand over a map keyed by a brand.
#[model_schema(no_display)]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(transparent)]
pub struct Logroll(pub HashMap<Callsign, u8>);

/// A brand over a map of the parameter keyed by an alias.
#[model_schema(default_types(Stowed = String), no_display)]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(transparent)]
pub struct Logroster<Stowed>(pub HashMap<Hull, Stowed>);

/// A map keyed by a brand over a date, by a date alias, in a type that declares a parameter.
#[cfg(feature = "chrono")]
#[model_schema(default_types(Stowed = String))]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct StowedRegatta<Stowed> {
    pub aliased: HashMap<PostmarkKey, Stowed>,
    pub branded: HashMap<Postmark, Stowed>,
}

/// A model with a rule, the value of a map under a brand and under an alias.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Cleat {
    #[model_schema_prop(minLength = 1)]
    pub label: String,
}

/// A map of models under a brand over a string, a brand over a flag and an alias of a number,
/// which `validate()` walks in the order of the key.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Dockage {
    pub callsign: HashMap<Callsign, Cleat>,
    pub hull: HashMap<Hull, Cleat>,
    pub signal: HashMap<Signal, Cleat>,
}

/// A date with a fraction of whole milliseconds, which chrono writes with three digits.
#[cfg(feature = "chrono")]
pub const fn first() -> DateTime<Utc> {
    DateTime::from_timestamp_millis(1_704_164_645_678).unwrap()
}
