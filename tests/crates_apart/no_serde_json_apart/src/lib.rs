//! A crate apart from the test binaries that lists no `serde_json`: its model types have
//! `from_deserializer`, which names no format, and a caller picks the one it reads.

// No lint table reaches a crate apart, and clippy lints none.
#![deny(warnings)]

use serde::{Deserialize, Deserializer, Serialize};
use tixschema::model_schema;

/// A struct with two rules.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Shipment {
    #[model_schema_prop(minLength = 1)]
    pub label: String,
    #[model_schema_prop(maximum = 100)]
    pub parcel_count: u32,
}

/// An enum with a draft, a rule on each variant.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", tag = "kind")]
pub enum Leg {
    Air {
        #[model_schema_prop(maximum = 9)]
        stops: u32,
    },
    Sea {
        #[model_schema_prop(minLength = 1)]
        vessel: String,
    },
}

/// A plain enum, which is its own valid type.
#[model_schema()]
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum Mode {
    Air,
    Sea,
}

/// A generic struct with a rule of its own.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct Hold<Item> {
    pub items: Vec<Item>,
    #[model_schema_prop(minLength = 1)]
    pub label: String,
}

/// A brand with a bound.
#[model_schema(minLength = 2)]
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(transparent)]
pub struct Tag(String);

/// Reads a shipment over whatever deserializer it is handed.
pub fn shipment<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Shipment, Vec<String>> {
    Shipment::from_deserializer(deserializer)
}
