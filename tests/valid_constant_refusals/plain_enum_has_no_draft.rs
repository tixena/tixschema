//! A plain enum that names no check has no draft: its variants are its values.

use serde::{Deserialize, Serialize};
use tixschema::model_schema;

#[model_schema()]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum InvoiceStatus {
    Draft,
    PastDue,
}

fn main() {
    let _status = InvoiceStatusDraft::PastDue;
}
