//! A strict enum whose struct variant flattens a member read through the `with` module of its
//! author's: tixschema cannot see what its `deserialize` answers.

use serde::{Deserialize, Serialize};
use tixschema::model_schema;

#[model_schema()]
#[derive(Debug, Deserialize, Serialize)]
pub struct Version {
    pub major: u32,
}

mod as_written {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub fn serialize<S: Serializer>(
        version: &super::Version,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        version.serialize(serializer)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<super::Version, D::Error> {
        super::Version::deserialize(deserializer)
    }
}

#[model_schema()]
#[derive(Debug, Deserialize, Serialize)]
pub enum Event {
    Released {
        id: String,
        #[serde(flatten, with = "as_written")]
        version: Version,
    },
}

fn main() {}
