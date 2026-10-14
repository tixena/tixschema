//! Parcels and the manifests that list them: models this crate alone reads.

use serde::{Deserialize, Serialize};
use tixschema::model_schema;

/// A stored parcel.
#[model_schema(enable = ["decode_with", "mongodb"])]
#[derive(Debug, Deserialize, Serialize)]
pub struct Parcel {
    /// The label printed on the parcel.
    pub label: String,
}

/// The parcels one shipment carries, read from JSON alone.
#[model_schema(enable = ["decode_with"])]
#[derive(Debug, Deserialize, Serialize)]
pub struct Manifest {
    /// The parcels the shipment carries.
    pub parcels: Vec<Parcel>,
}
