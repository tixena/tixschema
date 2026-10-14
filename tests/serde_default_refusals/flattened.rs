//! A flattened member: serde reads each key it leaves out from the default of the flattened type.

use serde::{Deserialize, Serialize};
use tixschema::model_schema;

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct Mooring {
    pub line: String,
}

#[model_schema()]
#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "kind")]
pub enum Berthing {
    Docked {
        #[serde(flatten, default)]
        mooring: Mooring,
        pier: u32,
    },
}

fn main() {}
