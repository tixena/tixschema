//! Three holders of a flattened untagged enum beside a member of their own: `Customer` and the
//! variant of `Event`, marked `nonstrict`, over an enum that reads every key still left and refuses
//! none, and `Supplier`, strict, over a strict enum that refuses a key it does not declare.
//! `run_read_past.rs` reads a key no member declares past on every surface for the first two, and
//! refuses it on every surface for the third.

use serde::{Deserialize, Serialize};
use tixschema::model_schema;

/// An untagged enum that reads every key still left and refuses none.
#[model_schema(enable = ["jsonschema"], nonstrict)]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(untagged)]
pub enum Contact {
    Email { email: String },
    Phone { phone: String },
}

/// A holder of [`Contact`], which serde cannot close, so it is marked `nonstrict`.
#[model_schema(enable = ["jsonschema"], nonstrict)]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Customer {
    #[serde(flatten)]
    pub contact: Contact,
    pub name: String,
}

/// An enum whose variant holds [`Contact`], which serde cannot close, so it is marked `nonstrict`.
#[model_schema(enable = ["jsonschema"], nonstrict)]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum Event {
    Signed {
        #[serde(flatten)]
        contact: Contact,
        name: String,
    },
}

/// An untagged enum that reads every key still left and refuses one it does not declare.
#[model_schema(enable = ["jsonschema"])]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(untagged)]
pub enum SupplierContact {
    Email { email: String },
    Phone { phone: String },
}

/// A strict holder of [`SupplierContact`], which refuses the key the holder leaves to it.
#[model_schema(enable = ["jsonschema"])]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Supplier {
    #[serde(flatten)]
    pub contact: SupplierContact,
    pub name: String,
}
