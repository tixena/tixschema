//! `DocumentService` and `BillingService`, both asking for `mcp`, and the stand-in that answers
//! them.

#![cfg(feature = "serde")]

use core::future::{Future, poll_fn, ready};
use core::task::Poll;
use serde::{Deserialize, Serialize};
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
}

/// Reads and renames documents in the caller's workspace.
#[service_schema(transports = ["mcp"])]
pub trait DocumentService<Ctx> {
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

    /// Marks one document as read.
    #[service_schema_op(one_way, mcp(scopes = ["documents:read", "documents:touch"]))]
    async fn touch_document(&self, ctx: &Ctx, document_id: String, reader: String);
}

/// Exports folders.
#[service_schema(transports = ["mcp"])]
pub trait ExportService<Ctx> {
    /// Exports one folder, reporting one step of three.
    async fn export_folder(&self, ctx: &Ctx, folder: String) -> Result<(), VersionError>;
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
}

impl<Ctx> DocumentService<Ctx> for Archive {
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

    fn lookup(
        &self,
        _ctx: &Ctx,
        id: DocumentId,
    ) -> impl Future<Output = Result<VersionResponse, VersionError>> {
        self.reach(format!("lookup-document {}", id.0));
        ready(Self::version_of(&id.0))
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
}

/// A handler that panics with the sentence the handler of the document's own sample says.
fn came_apart() -> ! {
    resume_unwind(Box::new("the handler came apart"))
}
