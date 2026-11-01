//! An `openapi` path that does not start with `/`.

use serde::{Deserialize, Serialize};
use tixschema::service_schema;

#[derive(Deserialize, Serialize)]
pub struct BalanceRequest {
    pub customer_id: String,
}

#[derive(Deserialize, Serialize)]
pub struct Balance {
    pub invoices: u32,
}

#[derive(Deserialize, Serialize)]
pub enum LedgerError {
    DbError,
}

#[service_schema(transports = ["http_rest"], openapi(path = "openapi.json"))]
pub trait LedgerService<Ctx> {
    async fn balance(&self, ctx: &Ctx, req: BalanceRequest) -> Result<Balance, LedgerError>;
}

fn main() {}
