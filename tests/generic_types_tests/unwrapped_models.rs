//! The models whose TypeScript valid type is the draft branded with no `Readonly<>` (an untagged
//! enum that holds itself, an untagged enum over a brand, and a tuple struct whose every slot is
//! skipped), and the holders that flatten an alias of a filled generic struct declared above them,
//! below them and in another crate.

use alloc::collections::BTreeMap;
use flattened_members_apart::ManifestListApart;
use serde::{Deserialize, Serialize};
use tixschema::model_schema;

/// An untagged enum that holds itself in a list and in a map.
#[model_schema(name = "Sounding")]
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(untagged)]
pub enum SoundingData {
    Depth(f64),
    Flag(bool),
    Keyed(BTreeMap<String, Self>),
    Listed(Vec<Self>),
    Note(String),
}

/// A tuple struct whose every slot is skipped, which serde writes as `[]`.
#[model_schema()]
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct Unrigged(#[serde(skip)] (), #[serde(skip)] ());

/// A struct that holds an [`Unrigged`].
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Rigging {
    pub mast: String,
    pub spare_lines: Unrigged,
}

/// A plain enum whose variants are written as texts with a slash.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum HullForm {
    #[serde(rename = "form/flat")]
    Flat,
    #[serde(rename = "form/round")]
    Round,
}

/// A brand of a text of one character or more.
#[model_schema(minLength = 1)]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(transparent)]
pub struct HullLabel(String);

/// An untagged enum over a plain enum and a brand, each of them a text.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(untagged)]
pub enum HullKind {
    Form(HullForm),
    Label(HullLabel),
}

/// A struct with a list of items, which each alias below fills.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Hold<Item> {
    pub items: Vec<Item>,
}

/// The item of each [`Hold`] an alias fills.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Bale {
    pub weight: u32,
}

/// An alias of a filled [`Hold`], declared above the struct that flattens it.
#[model_schema(name = "Holds")]
pub type HoldAbove = Hold<Bale>;

/// A struct that flattens [`HoldAbove`].
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct StowedAbove {
    #[serde(flatten)]
    pub hold: HoldAbove,
    pub note: String,
}

/// A struct that flattens [`HoldBelow`], declared below it.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct StowedBelow {
    #[serde(flatten)]
    pub hold: HoldBelow,
    pub note: String,
}

/// An alias of a filled [`Hold`], declared below the struct that flattens it.
#[model_schema(name = "HoldsBelow")]
pub type HoldBelow = Hold<Bale>;

/// A struct that flattens an alias of another crate.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct StowedApart {
    #[serde(flatten)]
    pub manifest: ManifestListApart,
    pub note: String,
}

/// The TypeScript and Zod text of every model above, and of the models of another crate
/// [`StowedApart`] flattens.
pub fn unwrapped_texts() -> Vec<String> {
    use flattened_members_apart::{CargoApart, ParcelApart, manifest_list_apart_schema};
    vec![
        SoundingData::ts_definition(),
        SoundingData::zod_schema(),
        Unrigged::ts_definition(),
        Unrigged::zod_schema(),
        Rigging::ts_definition(),
        Rigging::zod_schema(),
        HullForm::ts_definition(),
        HullForm::zod_schema(),
        HullLabel::ts_definition(),
        HullLabel::zod_schema(),
        HullKind::ts_definition(),
        HullKind::zod_schema(),
        Bale::ts_definition(),
        Bale::zod_schema(),
        Hold::<Bale>::ts_definition(),
        Hold::<Bale>::zod_schema(),
        hold_above_schema::Schema::ts_definition(),
        hold_above_schema::Schema::zod_schema(),
        StowedAbove::ts_definition(),
        StowedAbove::zod_schema(),
        StowedBelow::ts_definition(),
        StowedBelow::zod_schema(),
        hold_below_schema::Schema::ts_definition(),
        hold_below_schema::Schema::zod_schema(),
        ParcelApart::ts_definition(),
        ParcelApart::zod_schema(),
        CargoApart::<ParcelApart>::ts_definition(),
        CargoApart::<ParcelApart>::zod_schema(),
        manifest_list_apart_schema::Schema::ts_definition(),
        manifest_list_apart_schema::Schema::zod_schema(),
        StowedApart::ts_definition(),
        StowedApart::zod_schema(),
    ]
}
