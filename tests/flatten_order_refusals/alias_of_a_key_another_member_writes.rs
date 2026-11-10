//! A member that reads, under an alias, a key another member writes: serde reads the key as the
//! writer's, where it reads it first, and the Kotlin and Swift reads hand it to both.

use serde::{Deserialize, Serialize};
use tixschema::model_schema;

#[model_schema()]
#[derive(Debug, Deserialize, Serialize)]
pub struct Price {
    pub amount: u32,
}

#[model_schema()]
#[derive(Debug, Deserialize, Serialize)]
pub struct Discount {
    #[serde(alias = "amount")]
    pub percent: u32,
}

#[model_schema()]
#[derive(Debug, Deserialize, Serialize)]
pub struct Order {
    pub label: String,
    #[serde(flatten)]
    pub price: Price,
    #[serde(flatten)]
    pub discount: Discount,
}

#[model_schema()]
#[derive(Debug, Deserialize, Serialize)]
pub struct Quote {
    pub amount: u32,
    #[serde(flatten)]
    pub discount: Discount,
}

fn main() {}
