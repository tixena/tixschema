//! One holder of each class of flattened members `#[model_schema]` accepts, which
//! `run_flatten_order.rs` reads on every client: members that take their own keys beside a key of
//! the holder's own and alone, an open member that reads every key ahead of one that takes its own,
//! one that refuses a key declared last, two open members, a map alone, and a type parameter
//! filled with a strict struct and flattened beside a key of its holder's own.

use alloc::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use tixschema::model_schema;

/// A struct that takes its own key.
#[model_schema(enable = ["jsonschema"])]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Stamp {
    pub at: String,
}

/// A second struct that takes its own key.
#[model_schema(enable = ["jsonschema"])]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Ink {
    pub colour: String,
}

/// An enum under a tag and a content key, which takes both.
#[model_schema(enable = ["jsonschema"])]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "lid", content = "body")]
pub enum Lid {
    Hinged { hinge: String },
    Loose,
}

/// An open enum under a tag alone, which reads every key and refuses none.
#[model_schema(enable = ["jsonschema"], nonstrict)]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "tone")]
pub enum Hue {
    Cold,
    Warm { warmth: u32 },
}

/// A strict enum under a tag alone, which reads every key and refuses one it does not declare.
#[model_schema(enable = ["jsonschema"])]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "form")]
pub enum Shape {
    Flat,
    Round { radius: u32 },
}

/// An open untagged enum, which reads every key and refuses none.
#[model_schema(enable = ["jsonschema"], nonstrict)]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(untagged)]
pub enum Seal {
    Foil { foil: u32 },
    Wax { wax: String },
}

/// Members that take their own keys beside a key of the holder's own: closed.
#[model_schema(enable = ["jsonschema"])]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ClosedParcel {
    pub label: String,
    #[serde(flatten)]
    pub lid: Lid,
    #[serde(flatten)]
    pub stamp: Stamp,
}

/// Members that take their own keys and no key of the holder's own: closed.
#[model_schema(enable = ["jsonschema"])]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct BareParcel {
    #[serde(flatten)]
    pub ink: Ink,
    #[serde(flatten)]
    pub stamp: Stamp,
}

/// An open member that reads every key, ahead of a member that takes its own: open.
#[model_schema(enable = ["jsonschema"], nonstrict)]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct OpenParcel {
    #[serde(flatten)]
    pub hue: Hue,
    pub label: String,
    #[serde(flatten)]
    pub stamp: Stamp,
}

/// A member that refuses a key it does not declare, declared last: open.
#[model_schema(enable = ["jsonschema"])]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct LastParcel {
    pub label: String,
    #[serde(flatten)]
    pub postmark: Stamp,
    #[serde(flatten)]
    pub shape: Shape,
}

/// Two open members that read every key: open.
#[model_schema(enable = ["jsonschema"], nonstrict)]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct TwoOpenParcel {
    #[serde(flatten)]
    pub hue: Hue,
    #[serde(flatten)]
    pub seal: Seal,
}

/// A map, the one member: open.
#[model_schema(enable = ["jsonschema"])]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct MapParcel {
    #[serde(flatten)]
    pub counts: BTreeMap<String, u32>,
}
