//! The held type, declared in a module of its own.

use serde::{Deserialize, Serialize};
use tixschema::model_schema;

#[model_schema(enable = ["decode_with"])]
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Customer {
    pub name: String,
    pub open_invoices: u32,
}
