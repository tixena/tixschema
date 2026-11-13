//! A valid type does not derive `Default`, which would build a value `validate()` never saw. The
//! draft keeps the derive.

use serde::{Deserialize, Serialize};
use tixschema::model_schema;

#[model_schema()]
#[derive(Debug, Default, Deserialize, Serialize)]
pub struct Shipment {
    pub parcel_count: u32,
}

fn main() {
    let _draft = ShipmentDraft::default();
    let _shipment = Shipment::default();
}
