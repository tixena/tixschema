//! The services the Dart `mcp` server and client are run on: a document service, a billing
//! service beside it on one endpoint, and an audit service whose tool shares a name with the
//! document service's.

#![cfg(feature = "dart")]

use core::future::ready;
use serde::{Deserialize, Serialize};
use tixschema::{model_schema, service_schema};

#[model_schema(enable = ["jsonschema"])]
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum McpFormat {
    Pdf,
    Text,
}

#[model_schema(enable = ["jsonschema"])]
#[derive(Debug, Deserialize, Serialize)]
pub struct McpVersionRequest {
    #[model_schema_prop(minLength = 1)]
    pub document_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub format: Option<McpFormat>,
}

#[model_schema(enable = ["jsonschema"])]
#[derive(Debug, Deserialize, Serialize)]
pub struct McpVersionResponse {
    pub version: u32,
}

#[model_schema(enable = ["jsonschema"])]
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case", tag = "errorCode")]
pub enum McpVersionError {
    NotFound,
}

#[model_schema(enable = ["jsonschema"])]
#[derive(Debug, Deserialize, Serialize)]
#[serde(transparent)]
pub struct McpDocumentId(pub String);

#[model_schema(enable = ["jsonschema"])]
#[derive(Debug, Deserialize, Serialize)]
pub struct McpInvoiceRequest {
    pub organization: String,
}

#[model_schema(enable = ["jsonschema"])]
#[derive(Debug, Deserialize, Serialize)]
pub struct McpInvoiceList {
    pub invoices: Vec<String>,
}

/// Reads and renames documents in the caller's workspace.
#[service_schema(transports = ["mcp"])]
pub trait DartDocumentService<Ctx> {
    /// Answers the current version of one document, unless it still carries the tag the caller
    /// knows.
    #[service_schema_op(http(
        method = "POST",
        path = "/get-tagged",
        header_in("if-none-match" = known_tag),
        header_out("etag")
    ))]
    async fn get_tagged(
        &self,
        ctx: &Ctx,
        document_id: String,
        known_tag: Option<String>,
    ) -> Result<(McpVersionResponse, String), McpVersionError>;

    /// Answers the current version of one document.
    #[service_schema_op(mcp(title = "Current Document Version", scopes = ["documents:read"]))]
    async fn get_version(
        &self,
        ctx: &Ctx,
        req: McpVersionRequest,
    ) -> Result<McpVersionResponse, McpVersionError>;

    /// Answers the current version of the document the identifier names.
    #[service_schema_op(mcp(name = "lookup-document"))]
    async fn lookup(
        &self,
        ctx: &Ctx,
        id: McpDocumentId,
    ) -> Result<McpVersionResponse, McpVersionError>;

    /// Renames one document.
    #[service_schema_op(mcp(scopes = ["documents:write"]))]
    async fn rename_document(
        &self,
        ctx: &Ctx,
        document_id: String,
        title: String,
    ) -> Result<(), McpVersionError>;

    /// Marks one document as read.
    #[service_schema_op(one_way, mcp(scopes = ["documents:read", "documents:touch"]))]
    async fn touch_document(&self, ctx: &Ctx, document_id: String, reader: String);
}

/// Lists the invoices of an organization.
#[service_schema(transports = ["mcp"])]
pub trait DartBillingService<Ctx> {
    /// Lists the invoices of the caller's organization.
    async fn list_invoices(
        &self,
        ctx: &Ctx,
        req: McpInvoiceRequest,
    ) -> Result<McpInvoiceList, McpVersionError>;
}

/// Answers the version of the audit log.
#[service_schema(transports = ["mcp"])]
pub trait AuditService<Ctx> {
    /// Answers the version of the audit log.
    async fn get_version(
        &self,
        ctx: &Ctx,
        req: McpVersionRequest,
    ) -> Result<McpVersionResponse, McpVersionError>;
}

/// Binds a header to an argument named after each local the emitted code writes around it.
#[service_schema(transports = ["mcp"])]
pub trait TaggingService<Ctx> {
    /// Joins the header arguments with the identifier.
    #[service_schema_op(http(
        method = "POST",
        path = "/retag",
        header_in("x-arguments" = arguments),
        header_in("x-result" = result),
        header_in("x-members" = members),
        header_in("x-failure" = failure)
    ))]
    async fn retag(
        &self,
        ctx: &Ctx,
        id: String,
        arguments: String,
        result: String,
        members: Option<String>,
        failure: String,
    ) -> Result<String, McpVersionError>;
}

/// Answers every operation of every service with its first answer, so each trait is implemented
/// rather than merely declared.
pub struct Archive;

impl DartDocumentService<()> for Archive {
    async fn get_tagged(
        &self,
        _ctx: &(),
        _document_id: String,
        _known_tag: Option<String>,
    ) -> Result<(McpVersionResponse, String), McpVersionError> {
        ready(()).await;
        Err(McpVersionError::NotFound)
    }

    async fn get_version(
        &self,
        _ctx: &(),
        req: McpVersionRequest,
    ) -> Result<McpVersionResponse, McpVersionError> {
        ready(()).await;
        Ok(McpVersionResponse {
            version: u32::try_from(req.document_id.len()).unwrap_or_default(),
        })
    }

    async fn lookup(
        &self,
        _ctx: &(),
        _id: McpDocumentId,
    ) -> Result<McpVersionResponse, McpVersionError> {
        ready(()).await;
        Err(McpVersionError::NotFound)
    }

    async fn rename_document(
        &self,
        _ctx: &(),
        _document_id: String,
        _title: String,
    ) -> Result<(), McpVersionError> {
        ready(()).await;
        Ok(())
    }

    async fn touch_document(&self, _ctx: &(), _document_id: String, _reader: String) {
        ready(()).await;
    }
}

impl DartBillingService<()> for Archive {
    async fn list_invoices(
        &self,
        _ctx: &(),
        req: McpInvoiceRequest,
    ) -> Result<McpInvoiceList, McpVersionError> {
        ready(()).await;
        Ok(McpInvoiceList {
            invoices: vec![req.organization],
        })
    }
}

impl TaggingService<()> for Archive {
    async fn retag(
        &self,
        _ctx: &(),
        id: String,
        arguments: String,
        result: String,
        members: Option<String>,
        failure: String,
    ) -> Result<String, McpVersionError> {
        ready(()).await;
        Ok([id, arguments, result, members.unwrap_or_default(), failure].join(":"))
    }
}

impl AuditService<()> for Archive {
    async fn get_version(
        &self,
        _ctx: &(),
        _req: McpVersionRequest,
    ) -> Result<McpVersionResponse, McpVersionError> {
        ready(()).await;
        Err(McpVersionError::NotFound)
    }
}
