//! `DocumentService` and `BillingService`, both asking for `mcp`, and the stand-in that answers
//! them.

#![cfg(feature = "serde")]

use core::future::{Future, ready};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::panic::resume_unwind;
use std::sync::Mutex;
use tixschema::{model_schema, service_schema};

#[model_schema(enable = ["jsonschema"])]
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case", tag = "errorCode")]
pub enum BalanceError {
    UnknownAccount,
}

#[model_schema(enable = ["jsonschema"])]
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case", tag = "errorCode")]
pub enum VersionError {
    NotFound,
}

#[model_schema(enable = ["jsonschema"])]
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct BalanceResponse {
    pub cents: i64,
}

#[model_schema(enable = ["jsonschema"])]
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(transparent)]
pub struct DocumentId(pub String);

#[model_schema(enable = ["jsonschema"])]
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct GetVersionRequest {
    #[model_schema_prop(minLength = 1)]
    pub document_id: String,
}

#[model_schema(enable = ["jsonschema"])]
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct VersionResponse {
    pub version: u32,
}

/// Answers what a caller owes.
#[service_schema(transports = ["mcp"])]
pub trait BillingService<Ctx> {
    /// Answers the balance of one account in one ledger.
    #[service_schema_op(http(
        method = "POST",
        path = "/get-balance",
        header_in("x-ledger" = ledger)
    ))]
    async fn get_balance(
        &self,
        ctx: &Ctx,
        account_id: String,
        ledger: String,
    ) -> Result<BalanceResponse, BalanceError>;

    /// One invoice.
    #[service_schema_op(mcp(
        resource = "billing://invoices/{invoice_id}",
        mime_type = "application/json",
        ttl_ms = 1000,
        cache_scope = "public"
    ))]
    async fn get_invoice(&self, ctx: &Ctx, invoice_id: String) -> Result<String, BalanceError>;

    /// One invoice of one ledger.
    #[service_schema_op(mcp(resource = "billing://ledgers/{ledger}/invoices/{invoice_id}"))]
    async fn get_ledger_invoice(
        &self,
        ctx: &Ctx,
        ledger: String,
        invoice_id: String,
    ) -> Result<String, BalanceError>;

    /// Lists the invoices of the caller's organization, one page at a time.
    #[service_schema_op(mcp(list_resources, ttl_ms = 60000, cache_scope = "public"))]
    async fn list_invoices(
        &self,
        ctx: &Ctx,
        cursor: Option<String>,
    ) -> Result<billing_service_schema::McpResourcePage, BalanceError>;
}

/// Reads and renames documents in the caller's workspace.
#[service_schema(transports = ["mcp"])]
pub trait DocumentService<Ctx> {
    /// The guide to the configuration of a workspace.
    #[service_schema_op(mcp(
        resource = "guide://configure-workspace",
        mime_type = "text/markdown"
    ))]
    async fn configuration_guide(&self, ctx: &Ctx) -> Result<String, VersionError>;

    /// The file of one document.
    #[service_schema_op(mcp(resource = "workspace://documents/{document_id}"))]
    async fn document_file(
        &self,
        ctx: &Ctx,
        document_id: String,
    ) -> Result<(Vec<u8>, String), VersionError>;

    /// Answers the current version of one document, unless it still carries the tag the caller knows.
    #[service_schema_op(http(
        method = "POST",
        path = "/get-tagged",
        header_in("if-none-match" = known_tag),
        header_out("etag")
    ))]
    async fn get_tagged(
        &self,
        ctx: &Ctx,
        id: String,
        known_tag: Option<String>,
    ) -> Result<(VersionResponse, String), VersionError>;

    /// Answers the current version of one document.
    #[service_schema_op(mcp(title = "Current Document Version"))]
    async fn get_version(
        &self,
        ctx: &Ctx,
        req: GetVersionRequest,
    ) -> Result<VersionResponse, VersionError>;

    /// Lists the documents the caller can read, one page at a time.
    #[service_schema_op(mcp(list_resources))]
    async fn list_documents(
        &self,
        ctx: &Ctx,
        cursor: Option<String>,
    ) -> Result<document_service_schema::McpResourcePage, VersionError>;

    /// Answers the current version of the document the identifier names.
    #[service_schema_op(mcp(name = "lookup-document"))]
    async fn lookup(&self, ctx: &Ctx, id: DocumentId) -> Result<VersionResponse, VersionError>;

    /// Renames one document.
    async fn rename_document(
        &self,
        ctx: &Ctx,
        document_id: String,
        title: String,
    ) -> Result<(), VersionError>;

    /// Suggests the documents whose name starts with what the user typed.
    #[service_schema_op(mcp(completes(resource = "document_file", parameter = "document_id")))]
    async fn suggest_documents(
        &self,
        ctx: &Ctx,
        typed: String,
        chosen: HashMap<String, String>,
    ) -> Result<Vec<String>, VersionError>;

    /// Marks one document as read.
    #[service_schema_op(one_way)]
    async fn touch_document(&self, ctx: &Ctx, document_id: String, reader: String);
}

/// Answers every operation of both services, and writes down what reached it.
#[derive(Default)]
pub struct Archive {
    pub reached: Mutex<Vec<String>>,
}

impl Archive {
    fn reach(&self, call: String) {
        self.reached.lock().unwrap().push(call);
    }

    fn version_of(document_id: &str) -> Result<VersionResponse, VersionError> {
        match document_id {
            "doc-1" => Ok(VersionResponse { version: 7 }),
            "panic" => came_apart(),
            _ => Err(VersionError::NotFound),
        }
    }
}

// Every method below is synchronous work wrapped in an already-ready `Future`: nothing here waits
// on anything.
impl BillingService<()> for Archive {
    fn get_balance(
        &self,
        _ctx: &(),
        account_id: String,
        ledger: String,
    ) -> impl Future<Output = Result<BalanceResponse, BalanceError>> {
        self.reach(format!("get-balance {account_id} {ledger}"));
        ready(if account_id == "acct-1" {
            Ok(BalanceResponse { cents: 1250 })
        } else {
            Err(BalanceError::UnknownAccount)
        })
    }

    fn get_invoice(
        &self,
        _ctx: &(),
        invoice_id: String,
    ) -> impl Future<Output = Result<String, BalanceError>> {
        self.reach(format!("get-invoice {invoice_id}"));
        ready(if invoice_id == "inv-1" {
            Ok("{\"total\":120}".to_owned())
        } else {
            Err(BalanceError::UnknownAccount)
        })
    }

    fn get_ledger_invoice(
        &self,
        _ctx: &(),
        ledger: String,
        invoice_id: String,
    ) -> impl Future<Output = Result<String, BalanceError>> {
        self.reach(format!("get-ledger-invoice {ledger} {invoice_id}"));
        ready(Ok(format!("{ledger}/{invoice_id}")))
    }

    fn list_invoices(
        &self,
        _ctx: &(),
        cursor: Option<String>,
    ) -> impl Future<Output = Result<billing_service_schema::McpResourcePage, BalanceError>> {
        self.reach(format!("list-invoices {cursor:?}"));
        ready(Ok(billing_service_schema::McpResourcePage {
            resources: vec![billing_service_schema::McpResource {
                uri: "billing://invoices/inv-1".to_owned(),
                name: "inv-1".to_owned(),
                title: None,
                description: None,
                mime_type: None,
                size: None,
            }],
            next_cursor: None,
        }))
    }
}

impl DocumentService<()> for Archive {
    fn configuration_guide(&self, _ctx: &()) -> impl Future<Output = Result<String, VersionError>> {
        self.reach("configuration-guide".to_owned());
        ready(Ok("# Configure the workspace".to_owned()))
    }

    fn document_file(
        &self,
        _ctx: &(),
        document_id: String,
    ) -> impl Future<Output = Result<(Vec<u8>, String), VersionError>> {
        self.reach(format!("document-file {document_id}"));
        ready(match document_id.as_str() {
            "doc-1" => Ok((b"%PDF-".to_vec(), "application/pdf".to_owned())),
            "panic" => came_apart(),
            _ => Err(VersionError::NotFound),
        })
    }

    fn get_tagged(
        &self,
        _ctx: &(),
        id: String,
        known_tag: Option<String>,
    ) -> impl Future<Output = Result<(VersionResponse, String), VersionError>> {
        self.reach(format!("get-tagged {id} {known_tag:?}"));
        ready(Self::version_of(&id).map(|found| (found, "\"v7\"".to_owned())))
    }

    fn get_version(
        &self,
        _ctx: &(),
        req: GetVersionRequest,
    ) -> impl Future<Output = Result<VersionResponse, VersionError>> {
        self.reach(format!("get-version {}", req.document_id));
        ready(Self::version_of(&req.document_id))
    }

    fn list_documents(
        &self,
        _ctx: &(),
        cursor: Option<String>,
    ) -> impl Future<Output = Result<document_service_schema::McpResourcePage, VersionError>> {
        self.reach(format!("list-documents {cursor:?}"));
        let named = |uri: &str, name: &str| document_service_schema::McpResource {
            uri: uri.to_owned(),
            name: name.to_owned(),
            title: None,
            description: None,
            mime_type: Some("application/pdf".to_owned()),
            size: None,
        };
        ready(Ok(match cursor.as_deref() {
            None => document_service_schema::McpResourcePage {
                resources: vec![named("workspace://documents/doc-1", "id-card.pdf")],
                next_cursor: Some("page-2".to_owned()),
            },
            Some("page-2") => document_service_schema::McpResourcePage {
                resources: vec![named("workspace://documents/doc-2", "passport.pdf")],
                next_cursor: None,
            },
            Some("panic") => came_apart(),
            Some(_) => return ready(Err(VersionError::NotFound)),
        }))
    }

    fn lookup(
        &self,
        _ctx: &(),
        id: DocumentId,
    ) -> impl Future<Output = Result<VersionResponse, VersionError>> {
        self.reach(format!("lookup-document {}", id.0));
        ready(Self::version_of(&id.0))
    }

    fn rename_document(
        &self,
        _ctx: &(),
        document_id: String,
        title: String,
    ) -> impl Future<Output = Result<(), VersionError>> {
        self.reach(format!("rename-document {document_id} {title}"));
        ready(Self::version_of(&document_id).map(|_found| ()))
    }

    fn suggest_documents(
        &self,
        _ctx: &(),
        typed: String,
        chosen: HashMap<String, String>,
    ) -> impl Future<Output = Result<Vec<String>, VersionError>> {
        self.reach(format!(
            "suggest-documents {typed} {:?}",
            chosen.get("kind")
        ));
        ready(match typed.as_str() {
            "many" => Ok((0..150).map(|at| format!("many-{at}")).collect()),
            "panic" => came_apart(),
            "none" => Err(VersionError::NotFound),
            _ => Ok(["doc-1", "doc-2"]
                .into_iter()
                .filter(|name| name.starts_with(&typed))
                .map(str::to_owned)
                .collect()),
        })
    }

    fn touch_document(
        &self,
        _ctx: &(),
        document_id: String,
        reader: String,
    ) -> impl Future<Output = ()> {
        self.reach(format!("touch-document {document_id} {reader}"));
        if reader == "panic" {
            came_apart();
        }
        ready(())
    }
}

/// A handler that panics with the sentence the handler of the document's own sample says.
fn came_apart() -> ! {
    resume_unwind(Box::new("the handler came apart"))
}
