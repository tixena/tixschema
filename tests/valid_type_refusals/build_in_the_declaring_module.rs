//! A valid type is not built by its tuple constructor, in the module that declares it.

use serde::{Deserialize, Serialize};
use tixschema::model_schema;

#[model_schema()]
#[derive(Debug, Deserialize, Serialize)]
pub struct Shipment {
    pub parcel_count: u32,
}

fn main() {
    let draft = ShipmentDraft { parcel_count: 3 };
    let _shipment = Shipment(draft);
}
