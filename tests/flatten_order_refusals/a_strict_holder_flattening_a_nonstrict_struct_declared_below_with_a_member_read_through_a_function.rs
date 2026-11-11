//! A strict struct that flattens a `nonstrict` struct declared below it whose flattened member is
//! read through a `deserialize_with` of its author's: the holder reads from the hidden constant of
//! the `nonstrict` struct that it reads past a key no member declares.

use serde::{Deserialize, Deserializer, Serialize};
use tixschema::model_schema;

#[model_schema()]
#[derive(Debug, Deserialize, Serialize)]
pub struct Shipment {
    pub id: String,
    #[serde(flatten)]
    pub release: Release,
}

#[model_schema(nonstrict)]
#[derive(Debug, Deserialize, Serialize)]
pub struct Release {
    pub note: String,
    #[serde(flatten, deserialize_with = "as_written")]
    pub version: Version,
}

fn as_written<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Version, D::Error> {
    Version::deserialize(deserializer)
}

#[model_schema()]
#[derive(Debug, Deserialize, Serialize)]
pub struct Version {
    pub major: u32,
}

fn main() {}
