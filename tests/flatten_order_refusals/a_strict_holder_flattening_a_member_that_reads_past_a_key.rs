//! A strict struct that flattens a `nonstrict` untagged enum, which reads every key still left and
//! refuses none: serde cannot refuse a key no member declares beside it.

use serde::{Deserialize, Serialize};
use tixschema::model_schema;

#[model_schema(nonstrict)]
#[derive(Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum Contact {
    Email { email: String },
    Phone { phone: String },
}

#[model_schema()]
#[derive(Debug, Deserialize, Serialize)]
pub struct Customer {
    pub name: String,
    #[serde(flatten)]
    pub contact: Contact,
}

fn main() {}
