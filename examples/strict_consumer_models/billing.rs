//! Invoices, the customers they bill, and the lookup a request names an invoice by: the models
//! another crate reads.

use serde::{Deserialize, Serialize};
use tixschema::model_schema;

/// A customer an invoice bills.
#[model_schema(enable = ["decode_with", "mongodb"])]
#[derive(Debug, Deserialize, Serialize)]
#[non_exhaustive]
#[serde(rename_all = "camelCase")]
pub struct Customer {
    /// The customer's name.
    pub name: String,
    /// How many invoices the customer has not paid.
    pub open_invoices: u32,
}

/// A stored invoice.
#[model_schema(enable = ["decode_with", "mongodb"])]
#[derive(Debug, Deserialize, Serialize)]
#[non_exhaustive]
#[serde(rename_all = "camelCase")]
pub struct Invoice {
    /// Who the invoice bills.
    pub customer: Customer,
    /// The number printed on the invoice.
    pub number: String,
}

/// What a request names an invoice by. It is never stored.
#[model_schema(enable = ["decode_with"])]
#[derive(Debug, Deserialize, Serialize)]
#[non_exhaustive]
pub struct InvoiceLookup {
    /// The number printed on the invoice.
    pub number: String,
}
