//! A crate apart from the test binaries: a stored `Customer`, which the `Invoice` of
//! `tests/crates_apart_tests.rs` holds and reads through its own crate's support libraries.

// No lint table reaches a crate apart, and clippy lints none.
#![deny(warnings)]

use serde::{Deserialize, Serialize};
use tixschema::model_schema;

tixschema::decode_with_foundation!();
tixschema::mongodb_foundation!();

/// A customer an invoice bills.
#[model_schema(enable = ["decode_with", "mongodb"])]
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Customer {
    /// The customer's name.
    pub name: String,
    /// How many invoices the customer has not paid.
    pub open_invoices: u32,
}
