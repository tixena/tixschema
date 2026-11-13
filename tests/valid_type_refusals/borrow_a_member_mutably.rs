//! A member of a valid value is not borrowed mutably.

use serde::{Deserialize, Serialize};
use tixschema::model_schema;

#[model_schema()]
#[derive(Debug, Deserialize, Serialize)]
pub struct Shipment {
    pub parcel_count: u32,
}

fn main() {
    let mut shipment = ShipmentDraft { parcel_count: 3 }.validate().unwrap();
    let _borrowed = &mut shipment.parcel_count;
}
