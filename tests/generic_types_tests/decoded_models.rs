//! The models the decode functions are checked against: a struct with a rule, a holder of it, an
//! `f64`, an `i64`, an `Option`, an externally tagged enum and a holder of it, an internally tagged
//! enum, a recursive struct, a generic struct, a brand and a plain enum, each a shape whose schema
//! reads more than its draft describes.

use alloc::vec::Vec;
use serde::{Deserialize, Serialize};
use tixschema::model_schema;

/// A struct with a rule on a member.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DecodedParcel {
    #[model_schema_prop(minLength = 1)]
    pub label: String,
    pub parcel_count: u32,
}

/// A struct that holds a [`DecodedParcel`].
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct DecodedManifest {
    pub note: String,
    pub parcel: DecodedParcel,
}

/// A struct of an `f64`, which its schema reads through a `z.preprocess`.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct DecodedGauge {
    pub depth: f64,
}

/// A struct of an `i64`, which its schema reads from a number or a `bigint`.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct DecodedWide {
    pub count: i64,
}

/// A struct of an `Option`, which its schema reads from `null`.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct DecodedNoted {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

/// An externally tagged enum, which its schema reads from `unknown`.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum DecodedPayment {
    Card(String),
    Pending,
}

/// A struct that holds a [`DecodedPayment`].
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct DecodedBill {
    pub payment: DecodedPayment,
}

/// An internally tagged enum, whose unit variant's constant is an object.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "kind")]
pub enum DecodedShipping {
    Courier { company: String },
    Pickup,
}

/// A struct that holds a list of itself.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct DecodedNode {
    pub children: Vec<DecodedNode>,
    pub name: String,
}

/// A struct of one member of a type parameter.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct DecodedWrap<Inner> {
    pub inner: Inner,
}

/// A brand of a text of one character or more.
#[model_schema(minLength = 1)]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(transparent)]
pub struct DecodedLabel(String);

/// A plain enum, which has no draft.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum DecodedStatus {
    Active,
    Closed,
}

/// The TypeScript and Zod text of every model above.
pub fn decoded_texts() -> Vec<String> {
    vec![
        DecodedParcel::ts_definition(),
        DecodedParcel::zod_schema(),
        DecodedManifest::ts_definition(),
        DecodedManifest::zod_schema(),
        DecodedGauge::ts_definition(),
        DecodedGauge::zod_schema(),
        DecodedWide::ts_definition(),
        DecodedWide::zod_schema(),
        DecodedNoted::ts_definition(),
        DecodedNoted::zod_schema(),
        DecodedPayment::ts_definition(),
        DecodedPayment::zod_schema(),
        DecodedBill::ts_definition(),
        DecodedBill::zod_schema(),
        DecodedShipping::ts_definition(),
        DecodedShipping::zod_schema(),
        DecodedNode::ts_definition(),
        DecodedNode::zod_schema(),
        DecodedWrap::<String>::ts_definition(),
        DecodedWrap::<String>::zod_schema(),
        DecodedLabel::ts_definition(),
        DecodedLabel::zod_schema(),
        DecodedStatus::ts_definition(),
        DecodedStatus::zod_schema(),
    ]
}
