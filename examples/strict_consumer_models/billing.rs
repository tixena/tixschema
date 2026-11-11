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

/// The totals of an invoice, computed after it is read and never sent.
#[model_schema(enable = ["decode_with"])]
#[derive(Debug, Default, Deserialize, Serialize)]
#[non_exhaustive]
#[serde(default)]
pub struct InvoiceTotals {
    /// What the invoice bills.
    #[serde(skip)]
    pub billed: u32,
    /// How many lines the invoice lists.
    #[serde(skip, default = "one_line")]
    pub lines: u32,
}

/// What a reader keeps beside an invoice, never sent.
#[model_schema()]
#[derive(Debug, Deserialize, Serialize)]
#[non_exhaustive]
pub struct InvoiceCache<T: Default> {
    /// What the reader keeps.
    #[serde(skip)]
    pub held: T,
}

/// A count of the invoices a reader keeps, which `validate()` holds to at least one.
#[model_schema()]
#[derive(Debug, Deserialize, Serialize)]
#[non_exhaustive]
pub struct InvoiceTally<T: Default> {
    /// How many invoices are counted.
    #[model_schema_prop(minimum = 1)]
    pub count: u32,
    /// What the reader keeps beside the count.
    #[serde(skip)]
    pub held: T,
}

/// The lines an invoice lists before any is counted.
const fn one_line() -> u32 {
    1
}
