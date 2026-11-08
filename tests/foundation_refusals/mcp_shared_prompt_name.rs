//! Two services that each list a prompt called `review` cannot share an endpoint built in a
//! `const`: the name they share stops the build.

use serde::{Deserialize, Serialize};
use tixschema::{model_schema, service_schema};

#[model_schema(enable = ["jsonschema"])]
#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "errorCode")]
pub enum ReviewError {
    NotFound,
}

/// Reads documents.
#[service_schema(transports = ["mcp"])]
pub trait DocumentService<Ctx> {
    /// Reviews one document.
    #[service_schema_op(mcp(prompt))]
    async fn review(&self, ctx: &Ctx, document_id: String) -> Result<String, ReviewError>;
}

/// Bills accounts.
#[service_schema(transports = ["mcp"])]
pub trait BillingService<Ctx> {
    /// Reviews one account.
    #[service_schema_op(mcp(prompt, name = "review"))]
    async fn review_account(&self, ctx: &Ctx, account_id: String) -> Result<String, ReviewError>;
}

mod document_mcp {
    document_service_mcp_dispatcher!();
}

mod billing_mcp {
    billing_service_mcp_dispatcher!();
}

const WORKSPACE: document_mcp::McpEndpoint = document_mcp::McpEndpoint::new(
    &[DocumentServiceSchema::MCP_NAMES, BillingServiceSchema::MCP_NAMES],
    &[document_mcp::catalog, billing_mcp::catalog],
);

fn main() {}
