//! A nonstrict untagged enum whose first variant reads every object, its one member optional:
//! serde reads the value of the variant after it as the first one.

use serde::{Deserialize, Serialize};
use tixschema::model_schema;

#[model_schema(nonstrict)]
#[derive(Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum Discount {
    Open {
        #[serde(default, skip_serializing_if = "::core::option::Option::is_none")]
        note: Option<String>,
    },
    Percent {
        percent: u32,
    },
}

fn main() {}
