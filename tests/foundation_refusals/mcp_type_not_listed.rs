//! A service that asks for `mcp` answers with a type that does not list `"jsonschema"`: its tool
//! list calls the type's `json_schema_within`, which that type does not have.

use serde::{Deserialize, Serialize};
use tixschema::{model_schema, service_schema};

#[model_schema()]
#[derive(Debug, Deserialize, Serialize)]
pub struct Balance {
    pub invoices: u32,
}

#[model_schema(enable = ["jsonschema"])]
#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "errorCode")]
pub enum LedgerError {
    DbError,
}

#[service_schema(transports = ["mcp"])]
pub trait LedgerService<Ctx> {
    /// Answers the open balance.
    async fn balance(&self, ctx: &Ctx) -> Result<Balance, LedgerError>;
}

fn main() {}
