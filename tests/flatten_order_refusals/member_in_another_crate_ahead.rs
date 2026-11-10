//! A tag-alone enum of another crate flattened ahead of that crate's struct: the holder reads what
//! the enum is from its hidden constant.

use serde::{Deserialize, Serialize};
use tixschema::model_schema;

#[model_schema()]
#[derive(Debug, Deserialize, Serialize)]
pub struct Shipment {
    #[serde(flatten)]
    pub mark: invoice_lookups::ParcelMark,
    #[serde(flatten)]
    pub parcel: invoice_lookups::Parcel,
}

fn main() {}
