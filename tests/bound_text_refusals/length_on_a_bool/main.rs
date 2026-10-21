//! A length on a `bool` field, owned and read through a reference: neither holds a string for the
//! bound to measure.

use serde::Serialize;
use tixschema::model_schema;

#[model_schema()]
#[derive(Debug, Serialize)]
pub struct Flagged<'held> {
    #[model_schema_prop(minLength = 3)]
    pub flag: bool,
    #[model_schema_prop(minLength = 3)]
    pub held_flag: &'held bool,
}

fn main() {}
