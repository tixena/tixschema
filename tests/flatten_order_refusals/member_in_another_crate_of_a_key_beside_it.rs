//! A field of the holder's own beside a flattened struct of another crate that writes its key.

use serde::{Deserialize, Serialize};
use tixschema::model_schema;

#[model_schema()]
#[derive(Debug, Deserialize, Serialize)]
pub struct Shipment {
    pub grams: u32,
    #[serde(flatten)]
    pub weight: invoice_lookups::ParcelWeight,
}

fn main() {}
