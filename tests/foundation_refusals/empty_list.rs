//! A list with no entry.

use serde::{Deserialize, Serialize};
use tixschema::model_schema;

tixschema::decode_with_foundation!();

#[model_schema(enable = [])]
#[derive(Debug, Deserialize, Serialize)]
pub struct InvoiceLookup {
    pub number: String,
}

fn main() {}
