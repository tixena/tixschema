//! The models of `swift_tests/listed_maps.rs` and of
//! `service_schema_emitted_client_tests/run_swift_listed_maps.rs`: a map under a key Swift reads
//! through a wrapper, a number, a plain enum and a `bool`, in a list, in a list of optionals, in an
//! optional list and in an optional, as a field, as a member of a variant under each tagging serde
//! accepts, as a brand's value, in a tuple struct's one slot, in a tuple's slot, as a map's value
//! and in a variant's slot, in a type that declares a parameter and in one that declares none.

use std::collections::HashMap;

#[cfg(feature = "chrono")]
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tixschema::model_schema;

/// A plain enum a map is keyed by.
#[model_schema()]
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub enum Rung {
    High,
    Low,
}

/// Each shape over each key as a field, a list read as its default where its key is left out and
/// a list read from what a function answers.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Pantry {
    #[serde(default)]
    pub defaulted: Vec<HashMap<u32, String>>,
    pub flag_lists: Vec<HashMap<bool, String>>,
    pub flag_lists_or_none: Vec<Option<HashMap<bool, String>>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub maybe_flag_lists: Option<Vec<HashMap<bool, String>>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub maybe_flags: Option<HashMap<bool, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub maybe_number_lists: Option<Vec<HashMap<u32, String>>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub maybe_numbers: Option<HashMap<u32, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub maybe_rung_lists: Option<Vec<HashMap<Rung, String>>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub maybe_rungs: Option<HashMap<Rung, String>>,
    pub number_lists: Vec<HashMap<u32, String>>,
    pub number_lists_or_none: Vec<Option<HashMap<u32, String>>>,
    pub rung_lists: Vec<HashMap<Rung, String>>,
    pub rung_lists_or_none: Vec<Option<HashMap<Rung, String>>>,
    #[model_schema_prop(preprocess = ["verbatim"])]
    pub verbatim_lists: Vec<HashMap<u32, String>>,
}

/// [`Pantry`] in a type that declares a parameter.
#[model_schema(default_types(Stowed = String))]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct StowedPantry<Stowed> {
    #[serde(default)]
    pub defaulted: Vec<HashMap<u32, Stowed>>,
    pub flag_lists: Vec<HashMap<bool, Stowed>>,
    pub flag_lists_or_none: Vec<Option<HashMap<bool, Stowed>>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub maybe_flag_lists: Option<Vec<HashMap<bool, Stowed>>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub maybe_flags: Option<HashMap<bool, Stowed>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub maybe_number_lists: Option<Vec<HashMap<u32, Stowed>>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub maybe_numbers: Option<HashMap<u32, Stowed>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub maybe_rung_lists: Option<Vec<HashMap<Rung, Stowed>>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub maybe_rungs: Option<HashMap<Rung, Stowed>>,
    pub number_lists: Vec<HashMap<u32, Stowed>>,
    pub number_lists_or_none: Vec<Option<HashMap<u32, Stowed>>>,
    pub rung_lists: Vec<HashMap<Rung, Stowed>>,
    pub rung_lists_or_none: Vec<Option<HashMap<Rung, Stowed>>>,
    #[model_schema_prop(preprocess = ["verbatim"])]
    pub verbatim_lists: Vec<HashMap<u32, Stowed>>,
}

/// Each shape in a tuple's slot.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Larder {
    pub flag_lists_or_none: (Vec<Option<HashMap<bool, String>>>, u8),
    pub maybe_numbers: (Option<HashMap<u32, String>>, u8),
    pub maybe_rung_lists: (Option<Vec<HashMap<Rung, String>>>, u8),
    pub number_lists: (Vec<HashMap<u32, String>>, u8),
}

/// [`Larder`] in a type that declares a parameter.
#[model_schema(default_types(Stowed = String))]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct StowedLarder<Stowed> {
    pub flag_lists_or_none: (Vec<Option<HashMap<bool, Stowed>>>, u8),
    pub maybe_numbers: (Option<HashMap<u32, Stowed>>, u8),
    pub maybe_rung_lists: (Option<Vec<HashMap<Rung, Stowed>>>, u8),
    pub number_lists: (Vec<HashMap<u32, Stowed>>, u8),
}

/// Each shape in a slot of a tuple struct of several slots.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Cellar(
    pub Vec<HashMap<u32, String>>,
    pub Vec<Option<HashMap<bool, String>>>,
    pub Option<Vec<HashMap<Rung, String>>>,
    pub Option<HashMap<u32, String>>,
);

/// [`Cellar`] in a type that declares a parameter.
#[model_schema(default_types(Stowed = String))]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct StowedCellar<Stowed>(
    pub Vec<HashMap<u32, Stowed>>,
    pub Vec<Option<HashMap<bool, Stowed>>>,
    pub Option<Vec<HashMap<Rung, Stowed>>>,
    pub Option<HashMap<u32, Stowed>>,
);

/// Each shape as the value of a map Swift keys natively and of one under a key it wraps.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Attic {
    pub flag_lists_by_number: HashMap<u32, Vec<HashMap<bool, String>>>,
    pub maybe_number_lists_by_name: HashMap<String, Option<Vec<HashMap<u32, String>>>>,
    pub maybe_rungs_by_rung: HashMap<Rung, Option<HashMap<Rung, String>>>,
    pub number_lists_by_name: HashMap<String, Vec<HashMap<u32, String>>>,
    pub rung_lists_or_none_by_name: HashMap<String, Vec<Option<HashMap<Rung, String>>>>,
}

/// [`Attic`] in a type that declares a parameter.
#[model_schema(default_types(Stowed = String))]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct StowedAttic<Stowed> {
    pub flag_lists_by_number: HashMap<u32, Vec<HashMap<bool, Stowed>>>,
    pub maybe_number_lists_by_name: HashMap<String, Option<Vec<HashMap<u32, Stowed>>>>,
    pub maybe_rungs_by_rung: HashMap<Rung, Option<HashMap<Rung, Stowed>>>,
    pub number_lists_by_name: HashMap<String, Vec<HashMap<u32, Stowed>>>,
    pub rung_lists_or_none_by_name: HashMap<String, Vec<Option<HashMap<Rung, Stowed>>>>,
}

/// A tuple struct whose one slot is a list of maps.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Cupboard(pub Vec<HashMap<u32, String>>);

/// A tuple struct whose one slot is a list of optional maps.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Sideboard(pub Vec<Option<HashMap<bool, String>>>);

/// A tuple struct whose one slot is an optional list of maps.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Dresser(pub Option<Vec<HashMap<Rung, String>>>);

/// A tuple struct whose one slot is an optional map.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Caddy(pub Option<HashMap<u32, String>>);

/// [`Cupboard`] in a type that declares a parameter.
#[model_schema(default_types(Stowed = String))]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct StowedCupboard<Stowed>(pub Vec<HashMap<u32, Stowed>>);

/// [`Sideboard`] in a type that declares a parameter.
#[model_schema(default_types(Stowed = String))]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct StowedSideboard<Stowed>(pub Vec<Option<HashMap<bool, Stowed>>>);

/// [`Dresser`] in a type that declares a parameter.
#[model_schema(default_types(Stowed = String))]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct StowedDresser<Stowed>(pub Option<Vec<HashMap<Rung, Stowed>>>);

/// [`Caddy`] in a type that declares a parameter.
#[model_schema(default_types(Stowed = String))]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct StowedCaddy<Stowed>(pub Option<HashMap<u32, Stowed>>);

/// A brand over a list of maps.
#[model_schema(no_display)]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(transparent)]
pub struct Creel(pub Vec<HashMap<u32, String>>);

/// A brand over a list of optional maps.
#[model_schema(no_display)]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(transparent)]
pub struct Trug(pub Vec<Option<HashMap<bool, String>>>);

/// A brand with a named member over a list of maps.
#[model_schema(no_display)]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(transparent)]
pub struct Hopper {
    pub rung_lists: Vec<HashMap<Rung, String>>,
}

/// A brand over a list of maps of the parameter.
#[model_schema(default_types(Stowed = String))]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(transparent)]
pub struct StowedCreel<Stowed>(pub Vec<HashMap<Rung, Stowed>>);

/// Each shape in a variant's one slot, two in the slots of a variant of several, and each as a
/// member of a variant, under no tag attribute.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum Galley {
    Listed(Vec<HashMap<u32, String>>),
    ListedOrNone(Vec<Option<HashMap<bool, String>>>),
    Maybe(Option<HashMap<Rung, String>>),
    MaybeListed(Option<Vec<HashMap<u32, String>>>),
    Paired(Vec<HashMap<bool, String>>, Option<HashMap<Rung, String>>),
    Stocked {
        flag_lists: Vec<HashMap<bool, String>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        maybe_number_lists: Option<Vec<HashMap<u32, String>>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        maybe_rungs: Option<HashMap<Rung, String>>,
        number_lists_or_none: Vec<Option<HashMap<u32, String>>>,
    },
}

/// [`Galley`] in a type that declares a parameter.
#[model_schema(default_types(Stowed = String))]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum StowedGalley<Stowed> {
    Listed(Vec<HashMap<u32, Stowed>>),
    ListedOrNone(Vec<Option<HashMap<bool, Stowed>>>),
    Maybe(Option<HashMap<Rung, Stowed>>),
    MaybeListed(Option<Vec<HashMap<u32, Stowed>>>),
    Paired(Vec<HashMap<bool, Stowed>>, Option<HashMap<Rung, Stowed>>),
    Stocked {
        flag_lists: Vec<HashMap<bool, Stowed>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        maybe_number_lists: Option<Vec<HashMap<u32, Stowed>>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        maybe_rungs: Option<HashMap<Rung, Stowed>>,
        number_lists_or_none: Vec<Option<HashMap<u32, Stowed>>>,
    },
}

/// [`Galley`] under a tag and a content key.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "kind", content = "held")]
pub enum AdjacentGalley {
    Listed(Vec<HashMap<u32, String>>),
    ListedOrNone(Vec<Option<HashMap<bool, String>>>),
    Maybe(Option<HashMap<Rung, String>>),
    MaybeListed(Option<Vec<HashMap<u32, String>>>),
    Paired(Vec<HashMap<bool, String>>, Option<HashMap<Rung, String>>),
    Stocked {
        flag_lists: Vec<HashMap<bool, String>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        maybe_number_lists: Option<Vec<HashMap<u32, String>>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        maybe_rungs: Option<HashMap<Rung, String>>,
        number_lists_or_none: Vec<Option<HashMap<u32, String>>>,
    },
}

/// [`StowedGalley`] under a tag and a content key.
#[model_schema(default_types(Stowed = String))]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "kind", content = "held")]
pub enum AdjacentStowedGalley<Stowed> {
    Listed(Vec<HashMap<u32, Stowed>>),
    ListedOrNone(Vec<Option<HashMap<bool, Stowed>>>),
    Maybe(Option<HashMap<Rung, Stowed>>),
    MaybeListed(Option<Vec<HashMap<u32, Stowed>>>),
    Paired(Vec<HashMap<bool, Stowed>>, Option<HashMap<Rung, Stowed>>),
    Stocked {
        flag_lists: Vec<HashMap<bool, Stowed>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        maybe_number_lists: Option<Vec<HashMap<u32, Stowed>>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        maybe_rungs: Option<HashMap<Rung, Stowed>>,
        number_lists_or_none: Vec<Option<HashMap<u32, Stowed>>>,
    },
}

/// The variant a tag alone takes, each shape as a member. serde reads it from what it buffers
/// beside the tag, where it refuses a number key and a `bool` key written as text, so each map is
/// keyed by a plain enum.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "kind")]
pub enum TaggedGalley {
    Stocked {
        #[serde(skip_serializing_if = "Option::is_none")]
        maybe_rung_lists: Option<Vec<HashMap<Rung, String>>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        maybe_rungs: Option<HashMap<Rung, String>>,
        rung_lists: Vec<HashMap<Rung, String>>,
        rung_lists_or_none: Vec<Option<HashMap<Rung, String>>>,
    },
}

/// [`TaggedGalley`] in a type that declares a parameter.
#[model_schema(default_types(Stowed = String))]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "kind")]
pub enum TaggedStowedGalley<Stowed> {
    Stocked {
        #[serde(skip_serializing_if = "Option::is_none")]
        maybe_rung_lists: Option<Vec<HashMap<Rung, Stowed>>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        maybe_rungs: Option<HashMap<Rung, Stowed>>,
        rung_lists: Vec<HashMap<Rung, Stowed>>,
        rung_lists_or_none: Vec<Option<HashMap<Rung, Stowed>>>,
    },
}

/// The variants an untagged enum takes, each shape in a variant's one slot and as a member, each
/// map keyed by a plain enum as in [`TaggedGalley`].
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum UntaggedGalley {
    Listed(Vec<HashMap<Rung, String>>),
    ListedOrNone(Vec<Option<HashMap<Rung, String>>>),
    Maybe(Option<HashMap<Rung, String>>),
    Stocked {
        #[serde(skip_serializing_if = "Option::is_none")]
        maybe_rung_lists: Option<Vec<HashMap<Rung, String>>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        maybe_rungs: Option<HashMap<Rung, String>>,
        rung_lists: Vec<HashMap<Rung, String>>,
        rung_lists_or_none: Vec<Option<HashMap<Rung, String>>>,
    },
}

/// [`UntaggedGalley`] in a type that declares a parameter.
#[model_schema(default_types(Stowed = String))]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum UntaggedStowedGalley<Stowed> {
    Listed(Vec<HashMap<Rung, Stowed>>),
    ListedOrNone(Vec<Option<HashMap<Rung, Stowed>>>),
    Maybe(Option<HashMap<Rung, Stowed>>),
    Stocked {
        #[serde(skip_serializing_if = "Option::is_none")]
        maybe_rung_lists: Option<Vec<HashMap<Rung, Stowed>>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        maybe_rungs: Option<HashMap<Rung, Stowed>>,
        rung_lists: Vec<HashMap<Rung, Stowed>>,
        rung_lists_or_none: Vec<Option<HashMap<Rung, Stowed>>>,
    },
}

/// A list of maps of dates Swift keys natively, whose conversion parses each date, beside a list
/// of maps of dates under a key Swift wraps, whose wrapper parses them.
#[cfg(feature = "chrono")]
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Timeline {
    pub by_name_lists: Vec<HashMap<String, DateTime<Utc>>>,
    pub by_number_lists: Vec<HashMap<u32, DateTime<Utc>>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub maybe_by_name: Option<HashMap<String, DateTime<Utc>>>,
}

/// A map of optional dates under a key Swift wraps, and one under a key Swift keys natively, whose
/// values `validate()` walks.
#[cfg(feature = "chrono")]
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Dateline {
    pub maybe_by_name: HashMap<String, Option<Plank>>,
    pub maybe_by_number: HashMap<u32, Option<DateTime<Utc>>>,
}

/// A model with a rule, the value of a map that holds none.
#[cfg(feature = "chrono")]
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Plank {
    #[model_schema_prop(minLength = 1)]
    pub label: String,
}

/// The payload as it was written: the function [`Pantry`] and [`StowedPantry`] name.
const fn verbatim(payload: Value) -> Value {
    payload
}
