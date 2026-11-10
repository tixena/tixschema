//! The declaring crate: a service that asks for `mcp`, and nothing placed.

use serde::{Deserialize, Serialize};
use tixschema::{model_schema, service_schema};

#[model_schema(enable = ["jsonschema"])]
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct GetVersionRequest {
    #[model_schema_prop(minLength = 1)]
    pub document_id: String,
}

#[model_schema(enable = ["jsonschema"])]
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case", tag = "errorCode")]
pub enum VersionError {
    NotFound,
}

#[model_schema(enable = ["jsonschema"])]
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct VersionResponse {
    pub version: u32,
}

/// Reads and renames documents in the caller's workspace.
#[service_schema(transports = ["mcp"])]
pub trait DocumentService<Ctx> {
    /// Answers the current version of one document.
    async fn get_version(
        &self,
        ctx: &Ctx,
        req: GetVersionRequest,
    ) -> Result<VersionResponse, VersionError>;

    /// Marks one document as read.
    #[service_schema_op(one_way)]
    async fn touch_document(&self, ctx: &Ctx, document_id: String, reader: String);
}
