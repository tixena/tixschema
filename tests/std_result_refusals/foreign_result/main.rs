//! A type that holds `Result<u32, String>`, where `Result` names a type of the crate's own that no
//! `#[model_schema]` was written on: every surface describes the std `Result`, and the check
//! written beside the holder refuses the name.

use serde::{Deserialize, Serialize};
use tixschema::model_schema;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Result<Held, Refused> {
    pub held: Held,
    pub refused: Refused,
}

#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Shelved {
    pub outcome: Result<u32, String>,
}

fn main() {}
