//! A strict enum whose struct variant flattens a `nonstrict` untagged enum declared below it: the
//! enum reads from its hidden constant that the member reads every key still left and refuses none.

use serde::{Deserialize, Serialize};
use tixschema::model_schema;

#[model_schema()]
#[derive(Debug, Deserialize, Serialize)]
pub enum Event {
    Signed {
        #[serde(flatten)]
        contact: Contact,
        name: String,
    },
}

#[model_schema(nonstrict)]
#[derive(Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum Contact {
    Email { email: String },
    Phone { phone: String },
}

fn main() {}
