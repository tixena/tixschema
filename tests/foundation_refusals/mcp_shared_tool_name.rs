//! Two services that each list a tool called `get-version` cannot share an endpoint built in a
//! `const`: the name they share stops the build. A tool and a prompt of one name build, because
//! MCP keeps them in separate lists.

use serde::{Deserialize, Serialize};
use tixschema::{model_schema, service_schema};

#[model_schema(enable = ["jsonschema"])]
#[derive(Debug, Deserialize, Serialize)]
pub struct VersionResponse {
    pub version: u32,
}

#[model_schema(enable = ["jsonschema"])]
#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "errorCode")]
pub enum VersionError {
    NotFound,
}

/// Reads documents.
#[service_schema(transports = ["mcp"])]
pub trait DocumentService<Ctx> {
    /// Answers the current version of one document.
    async fn get_version(&self, ctx: &Ctx) -> Result<VersionResponse, VersionError>;
}

/// Bills accounts.
#[service_schema(transports = ["mcp"])]
pub trait BillingService<Ctx> {
    /// Answers the version of the ledger.
    #[service_schema_op(mcp(name = "get-version"))]
    async fn read_version(&self, ctx: &Ctx) -> Result<VersionResponse, VersionError>;
}

mod document_mcp {
    document_service_mcp_dispatcher!();
}

mod billing_mcp {
    billing_service_mcp_dispatcher!();
}

const TOOL_AND_PROMPT: document_mcp::McpEndpoint = document_mcp::McpEndpoint::new(
    &[&[("tool", "get-version")], &[("prompt", "get-version")]],
    &[],
);

const WORKSPACE: document_mcp::McpEndpoint = document_mcp::McpEndpoint::new(
    &[DocumentServiceSchema::MCP_NAMES, BillingServiceSchema::MCP_NAMES],
    &[document_mcp::catalog, billing_mcp::catalog],
);

fn main() {}
