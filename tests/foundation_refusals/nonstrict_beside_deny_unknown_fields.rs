//! A type that reads past a key no member declares and refuses one, both.

use serde::{Deserialize, Serialize};
use tixschema::model_schema;

#[model_schema(nonstrict)]
#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct InvoiceLookup {
    pub number: String,
}

fn main() {}
