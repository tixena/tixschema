//! The models the emitted Dart, Swift and Kotlin types are named from as Rust names them: a field
//! written `Self`, a model declared below the model that holds it, and a variant whose own
//! `rename_all` names its members under every tagging serde accepts.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use tixschema::model_schema;

/// A tree: a list of itself beside a bound of its own.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Bough {
    #[model_schema_prop(minLength = 1)]
    pub label: String,
    pub twigs: Vec<Self>,
}

/// `Self` in a map's value.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Spur {
    #[model_schema_prop(minLength = 1)]
    pub label: String,
    pub offshoots: HashMap<String, Self>,
}

/// `Self` in an `Option` over a `Box`.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Tip {
    #[model_schema_prop(minLength = 1)]
    pub label: String,
    #[model_schema_prop(nullable)]
    pub next: Option<Box<Self>>,
}

/// `Self` in a variant's slot.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "kind")]
pub enum Fork {
    Leaf {
        #[model_schema_prop(minLength = 1)]
        label: String,
    },
    Split {
        halves: Vec<Self>,
    },
}

/// `Self` in a generic type, named with its own parameter.
#[model_schema(default_types(StrandType = i32))]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Rope<StrandType> {
    pub links: Vec<Self>,
    pub strand: StrandType,
}

/// A holder of a model declared below it.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Hitch {
    #[model_schema_prop(minLength = 1)]
    pub hook: String,
    pub trailer: HitchedTrailer,
}

#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct HitchedTrailer {
    #[model_schema_prop(minLength = 1)]
    pub plate: String,
}

/// A holder of models declared below it, in an `Option` and in a list.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Hauler {
    #[model_schema_prop(minLength = 1)]
    pub name: String,
    #[model_schema_prop(nullable)]
    pub spare: Option<TowedTrailer>,
    pub trailers: Vec<TowedTrailer>,
}

#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct TowedTrailer {
    #[model_schema_prop(minLength = 1)]
    pub tag: String,
}

/// A variant whose members take its own `rename_all`, one that keeps the enum's
/// `rename_all_fields`, and a member under a rename of its own.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "mode", rename_all_fields = "SCREAMING_SNAKE_CASE")]
pub enum Carriage {
    #[serde(rename_all = "camelCase")]
    OverLand {
        #[model_schema_prop(minLength = 1)]
        road_name: String,
        #[serde(rename = "axle_total")]
        axle_count: u8,
    },
    Sea {
        port_name: String,
    },
}

/// [`Carriage`] under its external tagging.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all_fields = "SCREAMING_SNAKE_CASE")]
pub enum CarriageExternal {
    #[serde(rename_all = "camelCase")]
    OverLand {
        #[model_schema_prop(minLength = 1)]
        road_name: String,
        #[serde(rename = "axle_total")]
        axle_count: u8,
    },
    Sea {
        port_name: String,
    },
}

/// [`Carriage`] under its adjacent tagging.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(
    tag = "mode",
    content = "body",
    rename_all_fields = "SCREAMING_SNAKE_CASE"
)]
pub enum CarriageAdjacent {
    #[serde(rename_all = "camelCase")]
    OverLand {
        #[model_schema_prop(minLength = 1)]
        road_name: String,
        #[serde(rename = "axle_total")]
        axle_count: u8,
    },
    Sea {
        port_name: String,
    },
}

/// [`Carriage`] untagged.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(untagged, rename_all_fields = "SCREAMING_SNAKE_CASE")]
pub enum CarriageUntagged {
    #[serde(rename_all = "camelCase")]
    OverLand {
        #[model_schema_prop(minLength = 1)]
        road_name: String,
        #[serde(rename = "axle_total")]
        axle_count: u8,
    },
    Sea {
        port_name: String,
    },
}
