//! Holders of a flattened untagged enum beside a member of their own: `Customer` and the variant of
//! `Event`, marked `nonstrict`, over an enum that reads every key still left and refuses none,
//! `Supplier`, strict, over a strict enum that refuses a key it does not declare, and `Order`,
//! marked `nonstrict`, a variant over each. `Agreement` holds the members of such a variant in
//! `Signature`, marked `nonstrict`, under a newtype variant. `Delivery`, `Booking` and `Remark`,
//! marked `nonstrict`, are enums under a tag alone, under a tag and a content key, and untagged,
//! each with a variant that refuses a key it does not declare beside one that reads it past.
//! `run_read_past.rs` reads a key no member declares past on every surface where serde reads it
//! past, and refuses it on every surface where serde refuses it.

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

/// An enum whose variants each flatten one of [`SupplierContact`] and [`Contact`], marked
/// `nonstrict`: serde refuses a stray key in `Placed` and reads it past in `Signed`.
#[model_schema(enable = ["jsonschema"], nonstrict)]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
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

/// A holder of [`Contact`], which serde cannot close, so it is marked `nonstrict`.
#[model_schema(enable = ["jsonschema"], nonstrict)]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Signature {
    #[serde(flatten)]
    pub contact: Contact,
    pub name: String,
}

/// A strict enum whose newtype variant holds [`Signature`]: serde reads a stray key past in
/// `Signed` and refuses it in `Voided`.
#[model_schema(enable = ["jsonschema"])]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum Agreement {
    Signed(Signature),
    Voided { reason: String },
}

/// A strict struct, which refuses a key it does not declare.
#[model_schema(enable = ["jsonschema"])]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Ticket {
    pub code: String,
}

/// An enum under a tag alone, marked `nonstrict`: serde hands the keys beside the tag to the
/// [`Ticket`] of `Labelled`, which refuses a stray one, reads one past in `Loose` and refuses one
/// in `Placed`.
#[model_schema(enable = ["jsonschema"], nonstrict)]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind")]
pub enum Delivery {
    Labelled(Ticket),
    Loose {
        note: String,
    },
    Placed {
        #[serde(flatten)]
        contact: SupplierContact,
        name: String,
    },
}

/// An enum under a tag and a content key, marked `nonstrict`: serde reads a stray key past beside
/// the tag and the content key and in the content of `Noted`, and refuses one in the content of
/// `Placed`.
#[model_schema(enable = ["jsonschema"], nonstrict)]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", content = "body")]
pub enum Booking {
    Noted {
        note: String,
    },
    Placed {
        #[serde(flatten)]
        contact: SupplierContact,
        name: String,
    },
}

/// An untagged enum, marked `nonstrict`: serde reads a stray key past in `Noted`, and refuses one
/// in `Placed` and in the [`Ticket`] of `Ticketed`.
#[model_schema(enable = ["jsonschema"], nonstrict)]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(untagged)]
pub enum Remark {
    Noted {
        note: String,
    },
    Placed {
        #[serde(flatten)]
        contact: SupplierContact,
        name: String,
    },
    Ticketed(Ticket),
}
