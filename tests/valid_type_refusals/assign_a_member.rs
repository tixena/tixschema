//! A member of a valid value is not assigned.

use serde::{Deserialize, Serialize};
use tixschema::model_schema;

#[model_schema()]
#[derive(Debug, Deserialize, Serialize)]
pub struct Shipment {
    pub parcel_count: u32,
}

fn change(shipment: &mut Shipment) {
    shipment.parcel_count = 1;
}

fn main() {}
