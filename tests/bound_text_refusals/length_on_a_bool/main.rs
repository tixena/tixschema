//! A length on a `bool` field, owned and read through a reference, and on a model: none holds a
//! string or a list for the bound to measure. A pattern on a brand over a list: it holds no text.

use serde::{Deserialize, Serialize};
use tixschema::model_schema;

#[model_schema()]
#[derive(Debug, Serialize)]
pub struct Flagged<'held> {
    #[model_schema_prop(minLength = 3)]
    pub flag: bool,
    #[model_schema_prop(minLength = 3)]
    pub held_flag: &'held bool,
}

#[model_schema()]
#[derive(Debug, Deserialize, Serialize)]
pub struct Version {
    pub number: i32,
}

#[model_schema(no_display)]
#[derive(Debug, Deserialize, Serialize)]
#[serde(transparent)]
pub struct Keywords(pub Vec<String>);

#[model_schema()]
#[derive(Debug, Serialize)]
pub struct Filed {
    #[model_schema_prop(minLength = 3)]
    pub version: Version,
    #[model_schema_prop(pattern = "^a$")]
    pub keywords: Keywords,
}

fn main() {}
