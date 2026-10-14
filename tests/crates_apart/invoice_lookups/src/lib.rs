//! A crate apart from the test binaries whose types list "decode_with" alone: its manifest lists no
//! `mongodb`, and names `bson`, which its types and its library name where tixschema has `bson` on.
//! It builds with every warning denied in a build where tixschema has `mongodb` on.

#![deny(warnings)]

use serde::{Deserialize, Serialize};
use tixschema::model_schema;

tixschema::decode_with_foundation!();

/// A parcel an invoice lookup names.
#[model_schema(enable = ["decode_with"])]
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Parcel {
    /// The label printed on the parcel.
    pub label: String,
}

/// What a request names an invoice by, with the parcels it ships. It is never stored.
#[model_schema(enable = ["decode_with"])]
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct InvoiceLookup {
    /// The number printed on the invoice.
    pub number: String,
    /// The parcels the invoice ships.
    pub parcels: Vec<Parcel>,
}
