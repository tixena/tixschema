//! A library a strict consumer writes: models another crate reads, in a public module, and models
//! this crate alone reads, in a private one, with both support libraries written at its root. A
//! service asking for `mcp` is in a module of its own, its dispatcher, client and a writer in public ones.

pub mod billing;
#[macro_use]
pub mod desk;
pub mod desk_client;
pub mod desk_dispatcher;
pub mod desk_writer;
mod shelving;

use desk::{DeskService, DeskServiceSchema, desk_service_schema};
use shelving::manifest_schema::DecodeWith as _;

extern crate bson2 as bson;

tixschema::decode_with_foundation!();
tixschema::mongodb_foundation!();

/// The labels of the parcels the manifest `raw` holds, or `None` where it holds none.
#[must_use]
pub fn manifest_labels(raw: serde_json::Value) -> Option<Vec<String>> {
    shelving::Manifest::from_value_with(raw, |_raw, _found| {
        shelving::manifest_schema::Verdict::Reject
    })
    .ok()
    .map(|manifest| {
        manifest
            .parcels
            .into_iter()
            .map(|parcel| parcel.label)
            .collect()
    })
}

/// The label of the stored parcel `raw` holds, or `None` where it holds none.
#[must_use]
pub fn parcel_label(raw: bson::Document) -> Option<String> {
    shelving::Parcel::from_bson_with(raw, |_raw, _found| shelving::parcel_schema::Verdict::Reject)
        .ok()
        .map(|parcel| parcel.label)
}
