//! A declaration whose draft takes the name of another type is refused, naming both.

use serde::{Deserialize, Serialize};
use tixschema::model_schema;

#[model_schema()]
#[derive(Debug, Deserialize, Serialize)]
pub struct Shipment {
    pub parcel_count: u32,
}

#[model_schema()]
#[derive(Debug, Deserialize, Serialize)]
pub struct ShipmentDraft {
    pub label: String,
}

fn main() {}
