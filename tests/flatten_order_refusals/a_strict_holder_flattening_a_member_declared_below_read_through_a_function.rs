//! A strict struct that flattens a member read through a `deserialize_with` of its author's, of a
//! type declared below it: the function, not the type, decides what serde reads, so the build stops
//! where the holder is expanded.

use serde::{Deserialize, Deserializer, Serialize};
use tixschema::model_schema;

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

#[model_schema()]
#[derive(Debug, Deserialize, Serialize)]
pub struct Version {
    pub major: u32,
}

fn main() {}
