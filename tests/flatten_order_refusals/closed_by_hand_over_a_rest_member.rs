//! A holder its author closed with `deny_unknown_fields` that flattens a struct of a flattened
//! member: serde leaves the struct's keys in place, and the holder refuses them.

use serde::{Deserialize, Serialize};
use tixschema::model_schema;

#[model_schema()]
#[derive(Debug, Deserialize, Serialize)]
pub struct Stamp {
    pub at: String,
}

#[model_schema(nonstrict)]
#[derive(Debug, Deserialize, Serialize)]
pub struct Postmark {
    pub office: String,
    #[serde(flatten)]
    pub stamp: Stamp,
}

#[model_schema()]
#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Parcel {
    pub label: String,
    #[serde(flatten)]
    pub postmark: Postmark,
}

fn main() {}
