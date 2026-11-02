//! A service in a submodule that asks for `openapi`, whose crate root re-exports its generated
//! module and its trait and not its `<Service>Schema`.

mod contract {
    use serde::{Deserialize, Serialize};
    use tixschema::{model_schema, service_schema};

    #[model_schema(enable = ["jsonschema"])]
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

    #[service_schema(transports = ["http_rest"], openapi)]
    pub trait LedgerService<Ctx> {
        async fn balance(&self, ctx: &Ctx) -> Result<Balance, LedgerError>;
    }
}

pub use contract::{LedgerService, ledger_service_schema};

fn main() {}
