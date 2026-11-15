//! A struct that declares a rule has no `From` its draft: only `validate()` makes it valid.

use serde::{Deserialize, Serialize};
use tixschema::model_schema;

#[model_schema()]
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Shipment {
    #[model_schema_prop(minLength = 1)]
    pub label: String,
}

fn main() {
    let _shipment: Shipment = ShipmentDraft {
        label: String::new(),
    }
    .into();
}
