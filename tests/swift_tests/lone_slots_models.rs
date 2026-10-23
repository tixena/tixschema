//! The models of `swift_tests/lone_slots.rs` and of the table
//! `service_schema_emitted_client_tests/lone_slot_rows.rs` holds the Zod schema and each client to:
//! a tuple struct that declares one slot, which serde writes as the slot's value, over a number
//! with a bound, a text with a length, a model with a rule, a list, an optional, a map Swift keys
//! natively and one under a key it wraps, a date and a tuple, in a type that declares a parameter
//! and in one that declares none, and held in a field, in a list, as a map's value, as a type
//! argument, flattened and in a variant's one slot under each tagging serde accepts. A struct of
//! one slot and a brand over an externally tagged enum with a unit variant are flattened and held
//! beside a tag, where serde writes the variant as an entry of its name over `null`.

use std::collections::HashMap;

#[cfg(feature = "chrono")]
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tixschema::model_schema;

/// A model with a rule.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Bale {
    #[model_schema_prop(minLength = 1)]
    pub label: String,
}

/// [`Bale`] in a type that declares a parameter.
#[model_schema(default_types(Stowed = String))]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Sack<Stowed> {
    #[model_schema_prop(minLength = 1)]
    pub label: String,
    pub stowed: Stowed,
}

/// A number with a bound.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Count(#[model_schema_prop(minimum = 1)] pub u32);

/// A text with a length.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Moniker(#[model_schema_prop(minLength = 1)] pub String);

/// A model with a rule.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Baled(pub Bale);

/// A list of models, with a count of its own.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Bales(#[model_schema_prop(minLength = 1)] pub Vec<Bale>);

/// An optional model.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct MaybeBale(pub Option<Bale>);

/// A map of models Swift keys natively.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct BalesByName(pub HashMap<String, Bale>);

/// A map of models under a key Swift wraps.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct BalesByNumber(pub HashMap<u32, Bale>);

/// A map of texts under a key Swift wraps, which `validate()` walks nothing of.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct LabelsByNumber(pub HashMap<u32, String>);

/// A date.
#[cfg(feature = "chrono")]
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Ticked(pub DateTime<Utc>);

/// A date under `as_number`, which serde writes as a number.
#[cfg(feature = "chrono")]
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct TickedMillis(#[model_schema_prop(as_number)] pub DateTime<Utc>);

/// A tuple that holds a model.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct BalePair(pub (Bale, u8));

/// A slot under an attribute serde reads no lone slot by.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Unskipped(#[serde(skip)] pub String);

/// A struct of two slots, one of which serde skips: an array of the other.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Clipped(
    #[model_schema_prop(minLength = 1)] pub String,
    #[serde(skip)] pub u8,
);

/// The parameter itself.
#[model_schema(default_types(Stowed = String))]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Single<Stowed>(pub Stowed);

/// A model with a rule that holds the parameter.
#[model_schema(default_types(Stowed = String))]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Sacked<Stowed>(pub Sack<Stowed>);

/// A list of the parameter, with a count of its own.
#[model_schema(default_types(Stowed = String))]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct StowedList<Stowed>(#[model_schema_prop(minLength = 1)] pub Vec<Stowed>);

/// An optional parameter.
#[model_schema(default_types(Stowed = String))]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct StowedMaybe<Stowed>(pub Option<Stowed>);

/// A map of the parameter Swift keys natively.
#[model_schema(default_types(Stowed = String))]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct StowedByName<Stowed>(pub HashMap<String, Stowed>);

/// A map of the parameter under a key Swift wraps.
#[model_schema(default_types(Stowed = String))]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct StowedByNumber<Stowed>(pub HashMap<u32, Stowed>);

/// A date beside the parameter.
#[cfg(feature = "chrono")]
#[model_schema(default_types(Stowed = String))]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct StowedStamp<Stowed>(pub (DateTime<Utc>, Stowed));

/// A model with a rule beside the parameter.
#[model_schema(default_types(Stowed = String))]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct StowedPair<Stowed>(pub (Bale, Stowed));

/// Each struct of one slot in a field and in a list.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Tote {
    pub baled: Baled,
    pub counts: Vec<Count>,
    pub moniker: Moniker,
    pub sacked: Sacked<u8>,
    pub single: Single<u32>,
}

/// A model a brand holds.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Twine {
    #[model_schema_prop(minLength = 1)]
    pub twine: String,
}

/// A brand over a model, which serde writes as the model.
#[model_schema(no_display)]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(transparent)]
pub struct Twined(pub Twine);

/// A struct of one slot over a text, a number, a model and a list, each under a key, in a list, as
/// a map's value and as a type argument, and the one over a model flattened beside a flattened
/// brand over a model.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Laden {
    #[serde(flatten)]
    pub baled: Baled,
    pub bales: Bales,
    pub carried_bales: Sack<Bales>,
    pub carried_count: Sack<Count>,
    pub carried_model: Sack<Baled>,
    pub carried_moniker: Sack<Moniker>,
    pub count: Count,
    pub counts_by_name: HashMap<String, Count>,
    pub models: Vec<Baled>,
    pub models_by_name: HashMap<String, Baled>,
    pub monikers: Vec<Moniker>,
    pub monikers_by_name: HashMap<String, Moniker>,
    pub stacks: Vec<Bales>,
    pub stacks_by_name: HashMap<String, Bales>,
    #[serde(flatten)]
    pub twined: Twined,
}

/// An externally tagged enum with a unit variant, which serde writes flattened, and beside a tag,
/// as an entry of its name over `null`.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum Sling {
    Slack,
    Taut { tension: u32 },
}

/// A struct of one slot over [`Sling`].
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Slung(pub Sling);

/// A brand over [`Sling`].
#[model_schema(no_display)]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(transparent)]
pub struct Girth(pub Sling);

/// [`Slung`] flattened beside a member of its holder's own.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Strop {
    pub count: u32,
    #[serde(flatten)]
    pub slung: Slung,
}

/// [`Girth`] flattened beside a member of its holder's own.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Girthed {
    pub count: u32,
    #[serde(flatten)]
    pub girth: Girth,
}

/// [`Slung`] and [`Girth`] in the one slot of a variant under a tag alone, which serde writes
/// beside the tag.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "kind")]
pub enum Rigger {
    Girth(Girth),
    Slung(Slung),
}

/// Each struct of one slot in a variant's one slot, under no tag attribute.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum Stow {
    Counted(Count),
    Listed(Vec<Baled>),
    Named(Moniker),
    Packed(Baled),
    Sacked(Sacked<u8>),
    Stacked(Bales),
}

/// [`Stow`] under a tag and a content key.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "kind", content = "held")]
pub enum AdjacentStow {
    Counted(Count),
    Listed(Vec<Baled>),
    Named(Moniker),
    Packed(Baled),
    Sacked(Sacked<u8>),
    Stacked(Bales),
}

/// [`Stow`] under no tag, each variant read from a payload of another kind.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum UntaggedStow {
    Counted(Count),
    Listed(Vec<Baled>),
    Named(Moniker),
    Packed(Baled),
}

/// The variants of [`Stow`] a tag alone takes: a struct of one slot over a model.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "kind")]
pub enum TaggedStow {
    Packed(Baled),
    Sacked(Sacked<u8>),
}
