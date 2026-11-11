//! A strict struct that flattens a member read through a `deserialize_with` of its author's:
//! tixschema cannot see what the function answers, so only serde could refuse a key no member
//! declares as serde does.

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

#[model_schema()]
#[derive(Debug, Deserialize, Serialize)]
pub struct Release {
    pub id: String,
    #[serde(flatten, deserialize_with = "as_written")]
    pub version: Version,
}

fn main() {}
