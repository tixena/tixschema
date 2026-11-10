//! One struct of another crate flattened twice: both write its key.

use serde::{Deserialize, Serialize};
use tixschema::model_schema;

#[model_schema()]
#[derive(Debug, Deserialize, Serialize)]
pub struct Shipment {
    #[serde(flatten)]
    pub gross: invoice_lookups::ParcelWeight,
    #[serde(flatten)]
    pub net: invoice_lookups::ParcelWeight,
}

fn main() {}
