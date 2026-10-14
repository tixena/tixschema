//! A member serde leaves out of what it writes, which is no `Option`.

use serde::{Deserialize, Serialize};
use tixschema::model_schema;

#[model_schema()]
#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "kind")]
pub enum Berthing {
    Docked {
        #[serde(default, skip_serializing_if = "String::is_empty")]
        dock: String,
        pier: u32,
    },
}

fn main() {}
