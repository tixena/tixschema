//! An enum serde reads and does not write: the JSON of a default cannot be written.

use serde::Deserialize;
use tixschema::model_schema;

#[model_schema()]
#[derive(Debug, Deserialize)]
#[serde(tag = "kind")]
pub enum Berthing {
    Docked {
        #[serde(default)]
        dock: String,
        pier: u32,
    },
}

fn main() {}
