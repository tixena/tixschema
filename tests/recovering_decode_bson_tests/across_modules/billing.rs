//! The holder, declared below the module of the type it holds.

use serde::{Deserialize, Serialize};
use tixschema::model_schema;

use super::customers::Customer;

// The JSON schema of a type holding a `Customer` names its module from here, beside the type.
#[cfg(feature = "jsonschema")]
use super::customers::customer_schema;

#[model_schema(enable = ["decode_with", "mongodb"])]
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Invoice {
    pub customer: Customer,
    pub number: String,
}
