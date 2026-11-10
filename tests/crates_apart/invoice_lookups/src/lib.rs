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

/// The mark stamped on a parcel, under a tag alone: flattened, serde reads it from every key left,
/// so a holder in another crate reads what it is from its hidden constant.
#[model_schema()]
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "mark")]
pub enum ParcelMark {
    /// A stamp, under the ink it was stamped in.
    Stamped {
        /// The ink.
        ink: String,
    },
    /// No mark.
    Plain,
}

/// What a parcel weighs: its number is read in Kotlin through a helper this crate numbers.
#[model_schema()]
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ParcelWeight {
    /// The weight in grams.
    pub grams: u32,
}

/// Where a parcel is left, described in JSON Schema as well: a holder in another crate names it by
/// its path.
#[model_schema(enable = ["jsonschema"])]
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DropOff {
    /// The depot the parcel is left at.
    pub depot: String,
}
