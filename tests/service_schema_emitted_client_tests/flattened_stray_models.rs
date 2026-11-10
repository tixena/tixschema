//! Holders of a flattened member of each kind serde reads a stray key through: a struct that
//! flattens one, an enum under a tag alone, an untagged enum, a struct of one slot over a struct,
//! an externally tagged enum and an `Option` of one, and a variant of an enum under a tag alone
//! beside one that flattens. serde refuses a stray key beside each of them.

use serde::{Deserialize, Serialize};
use tixschema::model_schema;

#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Address {
    pub city: String,
}

#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Contact {
    #[serde(flatten)]
    pub address: Address,
    pub phone: String,
}

/// A struct that flattens one, flattened.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Customer {
    #[serde(flatten)]
    pub contact: Contact,
    pub name: String,
}

#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind")]
pub enum Payment {
    Card { last4: String },
    Cash,
}

/// An enum under a tag alone, flattened.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Order {
    pub label: String,
    #[serde(flatten)]
    pub payment: Payment,
}

#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(untagged)]
pub enum Delivery {
    Courier { courier: String },
    Pickup { store: String },
}

/// An untagged enum, flattened.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Shipment {
    #[serde(flatten)]
    pub delivery: Delivery,
    pub label: String,
}

#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Weight {
    pub grams: u32,
}

#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Measured(pub Weight);

/// A struct of one slot over a struct, flattened.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Parcel {
    pub label: String,
    #[serde(flatten)]
    pub weight: Measured,
}

#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum Size {
    Large(Weight),
    Small { width: u32 },
}

/// An externally tagged enum, flattened.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Carton {
    pub label: String,
    #[serde(flatten)]
    pub size: Size,
}

/// An `Option` of an externally tagged enum, flattened.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Pallet {
    pub label: String,
    #[serde(
        flatten,
        default,
        skip_serializing_if = "::core::option::Option::is_none"
    )]
    pub size: Option<Size>,
}

/// An enum under a tag alone whose one variant flattens a struct beside others that do not.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "type")]
pub enum Event {
    Counted {
        count: u32,
    },
    Moved {
        #[serde(flatten)]
        address: Address,
        label: String,
    },
}
