//! The flag written as it was before the `enable` list.

use serde::{Deserialize, Serialize};
use tixschema::model_schema;

tixschema::decode_with_foundation!();

#[model_schema(decode_with)]
#[derive(Debug, Deserialize, Serialize)]
pub struct InvoiceLookup {
    pub number: String,
}

fn main() {}
