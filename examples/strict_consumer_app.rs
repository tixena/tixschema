//! A binary a strict consumer writes: models of its own, one stored and one read from JSON alone,
//! with both support libraries written at its root, beside a request type of another crate.

extern crate bson2 as bson;

use shipment_schema::DecodeWith as _;
use std::process::ExitCode;

use bson::doc;
use invoice_lookup_schema::DecodeWith as _;
use invoice_lookups::{InvoiceLookup, invoice_lookup_schema};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tixschema::model_schema;

tixschema::decode_with_foundation!();
tixschema::mongodb_foundation!();

/// A shipment this binary stores.
#[model_schema(enable = ["decode_with", "mongodb"])]
#[derive(Debug, Deserialize, Serialize)]
struct Shipment {
    /// How many parcels the shipment carries.
    parcels: u32,
}

/// The wharf a request names, read from JSON alone.
#[model_schema(enable = ["decode_with"])]
#[derive(Debug, Deserialize, Serialize)]
struct Wharf {
    /// The wharf's name.
    name: String,
}

fn main() -> ExitCode {
    let lookup = InvoiceLookup::from_value_with(
        json!({ "number": "INV-1", "parcels": [] }),
        |_raw, _found| invoice_lookup_schema::Verdict::Reject,
    );
    let shipment = Shipment::from_bson_with(doc! { "parcels": 2_i32 }, |_raw, _found| {
        shipment_schema::Verdict::Reject
    });
    let wharf = Wharf::from_value_with(json!({ "name": "north" }), |_raw, _found| {
        wharf_schema::Verdict::Reject
    });
    let filter: bson::Document = Shipment::MONGO_FIELDS
        .parcels()
        .eq(2)
        .map_or_else(|_| bson::Document::new(), Into::into);
    if lookup.is_ok()
        && shipment.is_ok_and(|read| read.parcels == 2)
        && wharf.is_ok_and(|read| !read.name.is_empty())
        && !filter.is_empty()
    {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}
