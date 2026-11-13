//! A member of a valid value is not borrowed mutably.

use serde::{Deserialize, Serialize};
use tixschema::model_schema;

#[model_schema()]
#[derive(Debug, Deserialize, Serialize)]
pub struct Shipment {
    pub parcel_count: u32,
}

fn borrow(shipment: &mut Shipment) -> &mut u32 {
    &mut shipment.parcel_count
}

fn main() {}
