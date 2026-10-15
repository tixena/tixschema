//! The models of `swift_tests/lone_slots.rs` and of
//! `service_schema_emitted_client_tests/run_swift_lone_slots.rs`: a tuple struct that declares one
//! slot, which serde writes as the slot's value, over a number with a bound, a text with a length,
//! a model with a rule, a list, an optional, a map Swift keys natively and one under a key it
//! wraps, a date and a tuple, in a type that declares a parameter and in one that declares none,
//! and held in a field, in a list and in a variant's one slot under each tagging serde accepts.

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

/// Each struct of one slot in a variant's one slot, under no tag attribute.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum Stow {
    Counted(Count),
    Listed(Vec<Baled>),
    Named(Moniker),
    Packed(Baled),
    Sacked(Sacked<u8>),
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
