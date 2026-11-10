//! Holders of a flattened member of each kind serde reads a stray key through: a struct that
//! flattens one, an enum under a tag alone, an untagged enum, a struct of one slot over a struct,
//! an externally tagged enum and an `Option` of one, and a variant of an enum under a tag alone
//! beside one that flattens. serde refuses a stray key beside each of them.

use serde::{Deserialize, Serialize};
use tixschema::model_schema;

#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct BillingAddress {
    pub city: String,
}

#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct BillingContact {
    #[serde(flatten)]
    pub address: BillingAddress,
    pub phone: String,
}

/// A struct that flattens one, flattened.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct BillingCustomer {
    #[serde(flatten)]
    pub contact: BillingContact,
    pub name: String,
}

#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind")]
pub enum CheckoutPayment {
    Card { last4: String },
    Cash,
}

/// An enum under a tag alone, flattened.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CheckoutOrder {
    pub label: String,
    #[serde(flatten)]
    pub payment: CheckoutPayment,
}

#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(untagged)]
pub enum CourierDelivery {
    Courier { courier: String },
    Pickup { store: String },
}

/// An untagged enum, flattened.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CourierShipment {
    #[serde(flatten)]
    pub delivery: CourierDelivery,
    pub label: String,
}

#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct PackageWeight {
    pub grams: u32,
}

#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct WeighedPackage(pub PackageWeight);

/// A struct of one slot over a struct, flattened.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct WeighedParcel {
    pub label: String,
    #[serde(flatten)]
    pub weight: WeighedPackage,
}

#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum BoxSize {
    Large(PackageWeight),
    Small { width: u32 },
}

/// An externally tagged enum, flattened.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SizedCarton {
    pub label: String,
    #[serde(flatten)]
    pub size: BoxSize,
}

/// An `Option` of an externally tagged enum, flattened.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SizedPallet {
    pub label: String,
    #[serde(
        flatten,
        default,
        skip_serializing_if = "::core::option::Option::is_none"
    )]
    pub size: Option<BoxSize>,
}

/// An enum under a tag alone whose one variant flattens a struct beside others that do not.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "type")]
pub enum ShopEvent {
    Counted {
        count: u32,
    },
    Moved {
        #[serde(flatten)]
        address: BillingAddress,
        label: String,
    },
}

#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(untagged)]
pub enum DoorFinish {
    Bare {
        #[serde(default, skip_serializing_if = "::core::option::Option::is_none")]
        gloss: Option<u32>,
    },
    Painted {
        colour: String,
    },
}

/// A strict untagged enum whose first variant reads from an object of none of its keys, flattened:
/// serde reads `Painted` where `Bare` refuses its key.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct PaintedDoor {
    #[serde(flatten)]
    pub finish: DoorFinish,
    pub label: String,
}
