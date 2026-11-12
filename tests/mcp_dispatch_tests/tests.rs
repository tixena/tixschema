//! `DocumentService` and `BillingService`, both asking for `mcp`, and the stand-in that answers
//! them.

#![cfg(feature = "serde")]

use alloc::sync::Arc;
use core::future::{Future, poll_fn, ready};
use core::task::Poll;
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
pub struct DocumentId(String);

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

#[model_schema(enable = ["jsonschema"])]
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Reference {
    pub name: String,
}

#[model_schema(enable = ["jsonschema"])]
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Draft {
    pub references: Vec<Reference>,
    #[model_schema_prop(minLength = 1)]
    pub title: String,
}

#[model_schema(enable = ["jsonschema"])]
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DraftBatch {
    pub drafts: Vec<Draft>,
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
    /// Answers the number of drafts it was handed.
    #[service_schema_op(mcp(title = "Count Of Drafts"))]
    async fn check_drafts(
        &self,
        ctx: &Ctx,
        req: DraftBatch,
    ) -> Result<VersionResponse, VersionError>;

    /// Answers the versions of several documents, one request for each.
    #[service_schema_op(mcp(title = "Versions Of Many"))]
    async fn check_many(
        &self,
        ctx: &Ctx,
        req: Vec<GetVersionRequest>,
    ) -> Result<VersionResponse, VersionError>;

    /// Answers the version of a document, if one is asked for.
    #[service_schema_op(mcp(title = "Version Of Maybe One"))]
    async fn check_maybe(
        &self,
        ctx: &Ctx,
        req: Option<GetVersionRequest>,
    ) -> Result<VersionResponse, VersionError>;

    /// Answers the versions of documents asked for by name.
    #[service_schema_op(mcp(title = "Versions By Name"))]
    async fn check_named(
        &self,
        ctx: &Ctx,
        req: HashMap<String, GetVersionRequest>,
    ) -> Result<VersionResponse, VersionError>;

    /// Answers the version of the second of two documents.
    #[service_schema_op(mcp(title = "Versions Of A Pair"))]
    async fn check_pair(
        &self,
        ctx: &Ctx,
        first: GetVersionRequest,
        second: GetVersionRequest,
    ) -> Result<VersionResponse, VersionError>;

    /// The guide to the configuration of a workspace.
    #[service_schema_op(mcp(
        resource = "guide://configure-workspace",
        mime_type = "text/markdown",
        scopes = ["guides:read"]
    ))]
    async fn configuration_guide(&self, ctx: &Ctx) -> Result<String, VersionError>;

    /// The file of one document.
    #[service_schema_op(mcp(
        resource = "workspace://documents/{document_id}",
        scopes = ["documents:read", "files:read"]
    ))]
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
    #[service_schema_op(mcp(title = "Current Document Version", scopes = ["documents:read"]))]
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
    #[service_schema_op(mcp(name = "lookup-document", scopes = ["documents:read"]))]
    async fn lookup(&self, ctx: &Ctx, id: DocumentId) -> Result<VersionResponse, VersionError>;

    /// Renames one document.
    #[service_schema_op(mcp(scopes = ["documents:write"]))]
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
    #[service_schema_op(one_way, mcp(scopes = ["documents:read", "documents:touch"]))]
    async fn touch_document(&self, ctx: &Ctx, document_id: String, reader: String);
}

/// Exports folders.
#[service_schema(transports = ["mcp"], mcp(skills = "tests/mcp_skills"))]
pub trait ExportService<Ctx> {
    /// Exports one folder, reporting one step of three.
    async fn export_folder(&self, ctx: &Ctx, folder: String) -> Result<(), VersionError>;

    /// The report of one folder, reporting one step of three while it is read.
    #[service_schema_op(mcp(resource = "export://reports/{folder}", mime_type = "text/plain"))]
    async fn export_report(&self, ctx: &Ctx, folder: String) -> Result<String, VersionError>;

    /// Writes the figures of the caller's workspace.
    #[service_schema_op(mcp(skill = "workspace-figures"))]
    async fn figures_skill(
        &self,
        ctx: &Ctx,
        path: String,
    ) -> Result<(Vec<u8>, String), VersionError>;

    /// Writes the release notes of the caller's workspace: "what changed", from its latest changes.
    #[service_schema_op(mcp(skill = "release-notes"))]
    async fn release_notes_skill(&self, ctx: &Ctx, path: String) -> Result<String, VersionError>;
}

/// What a handler reaches of the request in flight, whichever dispatcher's call it came from.
pub struct Reporting {
    cancelled: Box<dyn Fn() -> bool + Send + Sync>,
    progress: Box<ProgressFn>,
}

type ProgressFn = dyn Fn(f64, Option<f64>, Option<&str>) + Send + Sync;

impl Reporting {
    pub fn new(
        progress: impl Fn(f64, Option<f64>, Option<&str>) + Send + Sync + 'static,
        cancelled: impl Fn() -> bool + Send + Sync + 'static,
    ) -> Self {
        Self {
            progress: Box::new(progress),
            cancelled: Box::new(cancelled),
        }
    }
}

/// Answers every operation of both services, and writes down what reached it.
#[derive(Clone, Default)]
pub struct Archive {
    pub reached: Arc<Mutex<Vec<String>>>,
}

impl Archive {
    fn figures(path: &str) -> Result<(Vec<u8>, String), VersionError> {
        match path {
            "SKILL.md" => Ok((b"# Figures\n".to_vec(), "text/markdown".to_owned())),
            "figures.bin" => Ok((vec![0, 255, 128, 1], "application/octet-stream".to_owned())),
            _ => Err(VersionErrorDraft::NotFound.validate().unwrap()),
        }
    }

    fn reach(&self, call: String) {
        self.reached.lock().unwrap().push(call);
    }

    fn release_notes(path: &str) -> Result<String, VersionError> {
        match path {
            "SKILL.md" => Ok("# Release notes\n\nRead the changes since 2026-11-01, then write one line per change.\n".to_owned()),
            "references/changes.md" => Ok("- renamed a document\n".to_owned()),
            "panic" => came_apart(),
            _ => Err(VersionErrorDraft::NotFound.validate().unwrap()),
        }
    }

    fn version_of(document_id: &str) -> Result<VersionResponse, VersionError> {
        match document_id {
            "doc-1" => Ok(VersionResponseDraft { version: 7 }.validate().unwrap()),
            "panic" => came_apart(),
            _ => Err(VersionErrorDraft::NotFound.validate().unwrap()),
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
            Ok(BalanceResponseDraft { cents: 1250 })
        } else {
            Err(BalanceErrorDraft::UnknownAccount)
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
            Err(BalanceErrorDraft::UnknownAccount)
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

impl<Ctx> DocumentService<Ctx> for Archive {
    fn check_drafts(
        &self,
        _ctx: &Ctx,
        req: DraftBatch,
    ) -> impl Future<Output = Result<VersionResponse, VersionError>> {
        ready(Ok(VersionResponseDraft {
            version: u32::try_from(req.drafts.len()).unwrap_or(u32::MAX),
        }))
    }

    fn check_many(
        &self,
        _ctx: &Ctx,
        req: Vec<GetVersionRequest>,
    ) -> impl Future<Output = Result<VersionResponse, VersionError>> {
        ready(
            req.first()
                .map_or(Err(VersionErrorDraft::NotFound), |first| {
                    Self::version_of(&first.document_id)
                }),
        )
    }

    fn check_maybe(
        &self,
        _ctx: &Ctx,
        req: Option<GetVersionRequest>,
    ) -> impl Future<Output = Result<VersionResponse, VersionError>> {
        ready(req.map_or(Err(VersionErrorDraft::NotFound), |found| {
            Self::version_of(&found.document_id)
        }))
    }

    fn check_named(
        &self,
        _ctx: &Ctx,
        req: HashMap<String, GetVersionRequest>,
    ) -> impl Future<Output = Result<VersionResponse, VersionError>> {
        ready(
            req.get("first")
                .map_or(Err(VersionErrorDraft::NotFound), |found| {
                    Self::version_of(&found.document_id)
                }),
        )
    }

    fn check_pair(
        &self,
        _ctx: &Ctx,
        _first: GetVersionRequest,
        second: GetVersionRequest,
    ) -> impl Future<Output = Result<VersionResponse, VersionError>> {
        ready(Self::version_of(&second.document_id))
    }

    fn configuration_guide(
        &self,
        _ctx: &Ctx,
    ) -> impl Future<Output = Result<String, VersionError>> {
        self.reach("configuration-guide".to_owned());
        ready(Ok("# Configure the workspace".to_owned()))
    }

    fn document_file(
        &self,
        _ctx: &Ctx,
        document_id: String,
    ) -> impl Future<Output = Result<(Vec<u8>, String), VersionError>> {
        self.reach(format!("document-file {document_id}"));
        ready(match document_id.as_str() {
            "doc-1" => Ok((b"%PDF-".to_vec(), "application/pdf".to_owned())),
            "panic" => came_apart(),
            _ => Err(VersionErrorDraft::NotFound),
        })
    }

    fn get_tagged(
        &self,
        _ctx: &Ctx,
        id: String,
        known_tag: Option<String>,
    ) -> impl Future<Output = Result<(VersionResponse, String), VersionError>> {
        self.reach(format!("get-tagged {id} {known_tag:?}"));
        ready(Self::version_of(&id).map(|found| (found, "\"v7\"".to_owned())))
    }

    fn get_version(
        &self,
        _ctx: &Ctx,
        req: GetVersionRequest,
    ) -> impl Future<Output = Result<VersionResponse, VersionError>> {
        self.reach(format!("get-version {}", req.document_id));
        ready(Self::version_of(&req.document_id))
    }

    fn list_documents(
        &self,
        _ctx: &Ctx,
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
            Some(_) => return ready(Err(VersionErrorDraft::NotFound)),
        }))
    }

    fn lookup(
        &self,
        _ctx: &Ctx,
        id: DocumentId,
    ) -> impl Future<Output = Result<VersionResponse, VersionError>> {
        self.reach(format!("lookup-document {}", id.as_inner()));
        ready(Self::version_of(id.as_inner()))
    }

    fn rename_document(
        &self,
        _ctx: &Ctx,
        document_id: String,
        title: String,
    ) -> impl Future<Output = Result<(), VersionError>> {
        self.reach(format!("rename-document {document_id} {title}"));
        ready(Self::version_of(&document_id).map(|_found| ()))
    }

    fn suggest_documents(
        &self,
        _ctx: &Ctx,
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
            "none" => Err(VersionErrorDraft::NotFound),
            _ => Ok(["doc-1", "doc-2"]
                .into_iter()
                .filter(|name| name.starts_with(&typed))
                .map(str::to_owned)
                .collect()),
        })
    }

    fn touch_document(
        &self,
        _ctx: &Ctx,
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

impl ExportService<Reporting> for Archive {
    fn export_folder(
        &self,
        ctx: &Reporting,
        folder: String,
    ) -> impl Future<Output = Result<(), VersionError>> {
        self.reach(format!("export-folder {folder}"));
        async move {
            (ctx.progress)(1.0_f64, Some(3.0_f64), Some("exported 1 of 3 files"));
            match folder.as_str() {
                "forever" => {
                    poll_fn(|_cx| {
                        if (ctx.cancelled)() {
                            Poll::Ready(())
                        } else {
                            Poll::Pending
                        }
                    })
                    .await;
                }
                "steps" => {
                    let mut yielded = false;
                    poll_fn(|cx| {
                        if yielded {
                            return Poll::Ready(());
                        }
                        yielded = true;
                        cx.waker().wake_by_ref();
                        Poll::Pending
                    })
                    .await;
                    (ctx.progress)(2.0_f64, Some(3.0_f64), None);
                }
                _ => {}
            }
            Ok(())
        }
    }

    fn export_report(
        &self,
        ctx: &Reporting,
        folder: String,
    ) -> impl Future<Output = Result<String, VersionError>> {
        self.reach(format!("export-report {folder}"));
        (ctx.progress)(1.0_f64, Some(3.0_f64), Some("exported 1 of 3 files"));
        ready(Ok(format!("report of {folder}")))
    }

    fn figures_skill(
        &self,
        _ctx: &Reporting,
        path: String,
    ) -> impl Future<Output = Result<(Vec<u8>, String), VersionError>> {
        ready(Self::figures(&path))
    }

    fn release_notes_skill(
        &self,
        _ctx: &Reporting,
        path: String,
    ) -> impl Future<Output = Result<String, VersionError>> {
        ready(Self::release_notes(&path))
    }
}

/// A handler that panics with the sentence the handler of the document's own sample says.
fn came_apart() -> ! {
    resume_unwind(Box::new("the handler came apart"))
}
