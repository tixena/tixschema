//! An enum under a tag and a content key whose struct variant flattens a tag-alone enum, which
//! keeps it from refusing a key beside its tag and content key.

use serde::{Deserialize, Serialize};
use tixschema::model_schema;

#[model_schema()]
#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "kind")]
pub enum Mark {
    Line { length: u32 },
}

#[model_schema()]
#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "form", content = "body")]
pub enum Label {
    Marked {
        code: String,
        #[serde(flatten)]
        mark: Mark,
    },
    Blank,
}

fn main() {}
