//! A strict enum whose struct variant flattens a `nonstrict` untagged enum beside a variant whose
//! members take their own keys: the enum cannot refuse a key in the one without refusing the keys
//! of the other's member, so the refusal names the newtype variant that keeps it strict.

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
pub enum Event {
    Signed {
        #[serde(flatten)]
        contact: Contact,
        name: String,
    },
    Voided {
        reason: String,
    },
}

fn main() {}
