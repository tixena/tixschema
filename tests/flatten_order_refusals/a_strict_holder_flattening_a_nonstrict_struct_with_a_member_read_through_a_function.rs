//! A strict struct that flattens a `nonstrict` struct whose flattened member is read through a
//! `deserialize_with` of its author's: the `nonstrict` struct reads past a key no member declares,
//! so serde cannot refuse such a key beside it.

use serde::{Deserialize, Deserializer, Serialize};
use tixschema::model_schema;

#[model_schema()]
#[derive(Debug, Deserialize, Serialize)]
pub struct Version {
    pub major: u32,
}

fn as_written<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Version, D::Error> {
    Version::deserialize(deserializer)
}

#[model_schema(nonstrict)]
#[derive(Debug, Deserialize, Serialize)]
pub struct Release {
    pub note: String,
    #[serde(flatten, deserialize_with = "as_written")]
    pub version: Version,
}

#[model_schema()]
#[derive(Debug, Deserialize, Serialize)]
pub struct Shipment {
    pub id: String,
    #[serde(flatten)]
    pub release: Release,
}

fn main() {}
