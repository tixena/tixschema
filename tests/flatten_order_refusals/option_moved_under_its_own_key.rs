//! A flattened `Option` that carries no `skip_serializing_if`, which a refusal tells to move under a
//! key of its own: the hint names the attribute an `Option` member takes there.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use tixschema::model_schema;

#[model_schema()]
#[derive(Debug, Deserialize, Serialize)]
pub struct Address {
    pub city: String,
}

#[model_schema()]
#[derive(Debug, Deserialize, Serialize)]
pub struct Invoice {
    pub number: String,
    #[serde(flatten)]
    pub extra: Option<BTreeMap<String, u32>>,
}

#[model_schema()]
#[derive(Debug, Deserialize, Serialize)]
pub struct Shipment {
    #[serde(flatten)]
    pub origin: Address,
    #[serde(flatten)]
    pub destination: Option<Address>,
}

fn main() {}
