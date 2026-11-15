//! Models that hold a type whose draft admits `null`, declared above it, below it, behind an alias
//! and in another crate, and models whose slot is a type parameter.

use serde::{Deserialize, Serialize};
use tixschema::model_schema;
use untagged_members_apart::ReadingApart;

/// A newtype over a model declared below it.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct HeldAbove(pub ReadingBelow);

/// An untagged enum with a variant over a model declared below it.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum ChoiceAbove {
    Flag(bool),
    Reading(ReadingBelow),
}

/// A model whose draft admits `null`.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum ReadingBelow {
    Absent(Option<String>),
    Count(u32),
}

/// A model whose draft admits `null`, declared above its holders.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum ReadingAbove {
    Absent(Option<String>),
    Count(u32),
}

/// A newtype over a model declared above it.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct HeldBelow(pub ReadingAbove);

/// An untagged enum with a variant over a model declared above it.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum ChoiceBelow {
    Flag(bool),
    Reading(ReadingAbove),
}

/// An alias of a model whose draft admits `null`.
#[model_schema()]
pub type ReadingAlias = ReadingAbove;

/// An untagged enum with a variant over an alias of such a model.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum ChoiceAliased {
    Flag(bool),
    Reading(ReadingAlias),
}

/// A newtype over an alias of such a model.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct HeldAliased(pub ReadingAlias);

/// A newtype over a model declared in another crate.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct HeldApart(pub ReadingApart);

/// An untagged enum with a variant over a model declared in another crate.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum ChoiceApart {
    Flag(bool),
    Reading(ReadingApart),
}

/// An untagged enum whose one variant is its own type parameter.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum Slot<Item> {
    Held(Item),
}

/// A newtype over its own type parameter.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Boxed<Item>(pub Item);

/// A model whose draft never admits `null`.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Plain {
    pub label: String,
}

/// A newtype over a model whose draft never admits `null`, declared below it.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct HeldPlainAbove(pub PlainBelow);

/// A model whose draft never admits `null`, declared below its holder.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PlainBelow {
    pub label: String,
}

/// A newtype over a model whose draft never admits `null`, declared above it.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct HeldPlainBelow(pub Plain);

/// A newtype over a type that holds a type parameter filled with an `Option`.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct HeldSlot(pub Slot<Option<u32>>);
