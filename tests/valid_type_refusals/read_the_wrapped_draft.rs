//! The draft a valid value wraps is not reached through the wrapper's field, in the declaring
//! module or outside it.

use serde::{Deserialize, Serialize};
use tixschema::model_schema;

#[model_schema()]
#[derive(Debug, Deserialize, Serialize)]
pub struct Shipment {
    pub parcel_count: u32,
}

fn main() {
    let mut shipment = ShipmentDraft { parcel_count: 3 }.validate().unwrap();
    shipment.0.parcel_count = 1;
}
