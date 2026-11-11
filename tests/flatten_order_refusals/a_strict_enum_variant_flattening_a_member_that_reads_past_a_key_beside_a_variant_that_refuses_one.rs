//! A strict enum whose struct variant flattens a `nonstrict` untagged enum, which reads every key
//! still left and refuses none, beside a variant that flattens a strict one, which refuses a key it
//! does not declare: each variant is judged by itself, so the one that reads past is refused.

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
#[serde(untagged)]
pub enum SupplierContact {
    Email { email: String },
    Phone { phone: String },
}

#[model_schema()]
#[derive(Debug, Deserialize, Serialize)]
pub enum Order {
    Placed {
        #[serde(flatten)]
        contact: SupplierContact,
        name: String,
    },
    Signed {
        #[serde(flatten)]
        contact: Contact,
        name: String,
    },
}

fn main() {}
