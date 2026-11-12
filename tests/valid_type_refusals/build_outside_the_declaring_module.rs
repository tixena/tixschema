//! A valid type is not built by its tuple constructor, from a module that does not declare it.

use serde::{Deserialize, Serialize};
use tixschema::model_schema;

mod declared {
    use super::*;

    #[model_schema()]
    #[derive(Debug, Deserialize, Serialize)]
    pub struct Shipment {
        pub parcel_count: u32,
    }
}

fn main() {
    let draft = declared::ShipmentDraft { parcel_count: 3 };
    let _shipment = declared::Shipment(draft);
}
