//! A type that lists "mongodb", holding a type of another crate that lists "decode_with" alone:
//! `invoice_lookups`, whose manifest lists neither `bson` nor `mongodb`.

extern crate bson2 as bson;

use invoice_lookups::InvoiceLookup;
use serde::{Deserialize, Serialize};
use tixschema::model_schema;

// The JSON schema of a type holding an `InvoiceLookup` names its module from here.
use invoice_lookups::invoice_lookup_schema;

tixschema::decode_with_foundation!();
tixschema::mongodb_foundation!();

#[model_schema(enable = ["decode_with", "mongodb"])]
#[derive(Debug, Deserialize, Serialize)]
pub struct Invoice {
    pub lookup: InvoiceLookup,
    pub number: String,
}

fn main() {}
