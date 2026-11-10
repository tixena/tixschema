//! `MobileDocumentService` and `MobileBillingService`, both asking for `mcp`, and the stand-in that answers
//! them: the Rust twins the Kotlin server and the Kotlin and Swift clients are measured against.

use core::future::{Future, ready};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::panic::resume_unwind;
use tixschema::{model_schema, service_schema};

macro_rules! sinks_to {
    ($($module:ident),*) => {$(
        impl super::$module::McpEventWriter for Sink {
            fn write(
                &self,
                _message: String,
            ) -> impl Future<Output = Result<(), super::$module::McpClientGone>> {
                ready(Ok(()))
            }
        }
    )*};
}

/// Where progress is written: nowhere, since these runs compare the answer alone.
pub struct Sink;

sinks_to!(mcp_mobile_document, mcp_mobile_billing);

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
pub struct Reference {
    pub name: String,
}

#[model_schema(enable = ["jsonschema"])]
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
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
pub trait MobileBillingService<Ctx> {
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
pub trait MobileDocumentService<Ctx> {
    /// Archives one document, with an argument named after a local the generated code writes.
    #[service_schema_op(http(
        method = "POST",
        path = "/archive-document",
        header_in("x-exchange" = exchange)
    ))]
    async fn archive_document(
        &self,
        ctx: &Ctx,
        document_id: String,
        reason: String,
        exchange: String,
    ) -> Result<(), VersionError>;

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

    /// Answers the current version of one document, unless it still carries the tag the caller knows.
    #[service_schema_op(http(
        method = "POST",
        path = "/get-tagged",
        header_in("if-none-match" = known_tag),
        header_out("etag"),
        error_header_out("x-reason")
    ))]
    async fn get_tagged(
        &self,
        ctx: &Ctx,
        id: String,
        known_tag: Option<String>,
    ) -> Result<(VersionResponse, String), (VersionError, Option<String>)>;

    /// Answers the current version of one document.
    #[service_schema_op(mcp(title = "Current Document Version", scopes = ["documents:read"]))]
    async fn get_version(
        &self,
        ctx: &Ctx,
        req: GetVersionRequest,
    ) -> Result<VersionResponse, VersionError>;

    /// Labels one document, with more arguments named after the locals the generated code writes.
    #[service_schema_op(http(
        method = "POST",
        path = "/label-document",
        header_in("x-sent" = sent),
        header_in("x-next-id" = next_id),
        header_in("x-mcp-source" = mcp_source),
        header_in("x-carried" = carried)
    ))]
    async fn label_document(
        &self,
        ctx: &Ctx,
        label: String,
        sent: String,
        next_id: String,
        mcp_source: String,
        carried: String,
    ) -> Result<VersionResponse, VersionError>;

    /// Answers the current version of the document the identifier names.
    #[service_schema_op(mcp(name = "lookup-document"))]
    async fn lookup(&self, ctx: &Ctx, id: DocumentId) -> Result<VersionResponse, VersionError>;

    /// Renames one document.
    #[service_schema_op(mcp(scopes = ["documents:write"]))]
    async fn rename_document(
        &self,
        ctx: &Ctx,
        document_id: String,
        title: String,
    ) -> Result<(), VersionError>;

    /// Tags one document, with arguments named after the locals the generated code writes.
    #[service_schema_op(http(
        method = "POST",
        path = "/tag-document",
        header_in("x-arguments" = arguments),
        header_in("x-message" = message),
        header_in("x-outcome" = outcome),
        header_in("x-members" = members)
    ))]
    async fn tag_document(
        &self,
        ctx: &Ctx,
        tag: String,
        arguments: String,
        message: String,
        outcome: String,
        members: String,
    ) -> Result<VersionResponse, VersionError>;

    /// Marks one document as read.
    #[service_schema_op(one_way, mcp(scopes = ["documents:read", "documents:touch"]))]
    async fn touch_document(&self, ctx: &Ctx, document_id: String, reader: String);
}

/// Answers every operation of both services.
#[derive(Default)]
pub struct Archive;

impl Archive {
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
impl MobileBillingService<()> for Archive {
    fn get_balance(
        &self,
        _ctx: &(),
        account_id: String,
        _ledger: String,
    ) -> impl Future<Output = Result<BalanceResponse, BalanceError>> {
        ready(if account_id == "acct-1" {
            Ok(BalanceResponse { cents: 1250 })
        } else {
            Err(BalanceError::UnknownAccount)
        })
    }
}

impl MobileDocumentService<()> for Archive {
    fn archive_document(
        &self,
        _ctx: &(),
        document_id: String,
        _reason: String,
        _exchange: String,
    ) -> impl Future<Output = Result<(), VersionError>> {
        ready(Self::version_of(&document_id).map(|_found| ()))
    }

    fn check_drafts(
        &self,
        _ctx: &(),
        req: DraftBatch,
    ) -> impl Future<Output = Result<VersionResponse, VersionError>> {
        ready(Ok(VersionResponse {
            version: u32::try_from(req.drafts.len()).unwrap_or(u32::MAX),
        }))
    }

    fn check_many(
        &self,
        _ctx: &(),
        req: Vec<GetVersionRequest>,
    ) -> impl Future<Output = Result<VersionResponse, VersionError>> {
        ready(req.first().map_or(Err(VersionError::NotFound), |first| {
            Self::version_of(&first.document_id)
        }))
    }

    fn check_maybe(
        &self,
        _ctx: &(),
        req: Option<GetVersionRequest>,
    ) -> impl Future<Output = Result<VersionResponse, VersionError>> {
        ready(req.map_or(Err(VersionError::NotFound), |found| {
            Self::version_of(&found.document_id)
        }))
    }

    fn check_named(
        &self,
        _ctx: &(),
        req: HashMap<String, GetVersionRequest>,
    ) -> impl Future<Output = Result<VersionResponse, VersionError>> {
        ready(
            req.get("first")
                .map_or(Err(VersionError::NotFound), |first| {
                    Self::version_of(&first.document_id)
                }),
        )
    }

    fn get_tagged(
        &self,
        _ctx: &(),
        id: String,
        _known_tag: Option<String>,
    ) -> impl Future<Output = Result<(VersionResponse, String), (VersionError, Option<String>)>>
    {
        ready(
            Self::version_of(&id)
                .map(|found| (found, "\"v7\"".to_owned()))
                .map_err(|refused| (refused, Some("no such document".to_owned()))),
        )
    }

    fn get_version(
        &self,
        _ctx: &(),
        req: GetVersionRequest,
    ) -> impl Future<Output = Result<VersionResponse, VersionError>> {
        ready(Self::version_of(&req.document_id))
    }

    fn label_document(
        &self,
        _ctx: &(),
        _label: String,
        _sent: String,
        _next_id: String,
        _mcp_source: String,
        _carried: String,
    ) -> impl Future<Output = Result<VersionResponse, VersionError>> {
        ready(Ok(VersionResponse { version: 10 }))
    }

    fn lookup(
        &self,
        _ctx: &(),
        id: DocumentId,
    ) -> impl Future<Output = Result<VersionResponse, VersionError>> {
        ready(Self::version_of(&id.0))
    }

    fn rename_document(
        &self,
        _ctx: &(),
        document_id: String,
        _title: String,
    ) -> impl Future<Output = Result<(), VersionError>> {
        ready(Self::version_of(&document_id).map(|_found| ()))
    }

    fn tag_document(
        &self,
        _ctx: &(),
        _tag: String,
        _arguments: String,
        _message: String,
        _outcome: String,
        _members: String,
    ) -> impl Future<Output = Result<VersionResponse, VersionError>> {
        ready(Ok(VersionResponse { version: 9 }))
    }

    fn touch_document(
        &self,
        _ctx: &(),
        _document_id: String,
        reader: String,
    ) -> impl Future<Output = ()> {
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
