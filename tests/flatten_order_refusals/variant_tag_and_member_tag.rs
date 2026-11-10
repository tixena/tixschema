//! A variant of an enum under the tag `kind` that flattens a struct under its own tag `kind`.

use serde::{Deserialize, Serialize};
use tixschema::model_schema;

#[model_schema()]
#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "kind")]
pub struct Stamp {
    pub at: String,
}

#[model_schema()]
#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "kind")]
pub enum Parcel {
    Stamped {
        #[serde(flatten)]
        stamp: Stamp,
    },
}

fn main() {}
