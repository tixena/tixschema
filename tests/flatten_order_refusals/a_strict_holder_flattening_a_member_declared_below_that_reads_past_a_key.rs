//! A strict struct that flattens a `nonstrict` untagged enum declared below it: the holder reads
//! from its hidden constant that the enum reads every key still left and refuses none.

use serde::{Deserialize, Serialize};
use tixschema::model_schema;

#[model_schema()]
#[derive(Debug, Deserialize, Serialize)]
pub struct Customer {
    pub name: String,
    #[serde(flatten)]
    pub contact: Contact,
}

#[model_schema(nonstrict)]
#[derive(Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum Contact {
    Email { email: String },
    Phone { phone: String },
}

fn main() {}
