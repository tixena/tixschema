//! Generic structs of another crate flattened side by side at a type parameter no
//! `default_types` entry fills: the holder cannot name their hidden constants.

use core::fmt::Display;

use serde::{Deserialize, Serialize};
use tixschema::model_schema;

#[model_schema()]
#[derive(Debug, Deserialize, Serialize)]
pub struct Handover<IdType: Display> {
    #[serde(flatten)]
    pub recipient: invoice_lookups::ParcelRecipient<IdType>,
    #[serde(flatten)]
    pub sender: invoice_lookups::ParcelSender<IdType>,
}

fn main() {}
