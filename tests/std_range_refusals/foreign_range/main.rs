//! A type that holds `Range<u32>`, where `Range` names a type of the crate's own that no
//! `#[model_schema]` was written on: every surface describes the std range, and the check written
//! beside the holder refuses the name.

use serde::{Deserialize, Serialize};
use tixschema::model_schema;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Range<Bound> {
    pub high: Bound,
    pub low: Bound,
}

#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Shelved {
    pub span: Range<u32>,
}

fn main() {}
