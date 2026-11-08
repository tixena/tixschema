//! `ApprovalService`: operations that ask the user with a form or a link, and operations that
//! answer content blocks, and the stand-in that answers them.

use core::future::{Future, ready};
use core::sync::atomic::{AtomicBool, Ordering};
use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use tixschema::{model_schema, service_schema};

#[model_schema(enable = ["jsonschema"])]
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DeleteConfirmation {
    pub confirm: bool,
}

#[model_schema(enable = ["jsonschema"])]
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case", tag = "errorCode")]
pub enum DeleteError {
    NeedsConfirmation { message: String },
    NotFound,
    Refused,
}

#[model_schema(enable = ["jsonschema"])]
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case", tag = "errorCode")]
pub enum ImportError {
    NeedsDriveConnection { message: String, url: String },
    NotFound,
}

#[model_schema(enable = ["jsonschema"])]
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case", tag = "errorCode")]
pub enum PreviewError {
    NotFound,
}

#[model_schema(enable = ["jsonschema"])]
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ReportRequest {
    pub title: String,
}

#[model_schema(enable = ["jsonschema"])]
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ImportedDocument {
    pub document_id: String,
}

#[model_schema(enable = ["jsonschema"])]
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum Priority {
    High,
    Low,
}

#[model_schema(enable = ["jsonschema"])]
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ShippingForm {
    pub express: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    pub parcels: u32,
    pub priority: Priority,
    pub tags: Vec<Priority>,
}

/// Asks the user before it changes anything, and builds content for the model.
#[service_schema(transports = ["mcp"])]
pub trait ApprovalService<Ctx> {
    /// Answers a chart and a link to the full report.
    #[service_schema_op(mcp(content))]
    async fn build_report(
        &self,
        ctx: &Ctx,
        req: ReportRequest,
    ) -> Result<Vec<approval_service_schema::McpContent>, PreviewError>;

    /// Deletes one document after the user confirms it.
    #[service_schema_op(mcp(interview(arm = "NeedsConfirmation", answers = confirmation)))]
    async fn delete_document(
        &self,
        ctx: &Ctx,
        document_id: String,
        confirmation: Option<DeleteConfirmation>,
    ) -> Result<(), DeleteError>;

    /// Answers the first page of one document.
    #[service_schema_op(mcp(content))]
    async fn document_preview(
        &self,
        ctx: &Ctx,
        document_id: String,
    ) -> Result<(Vec<u8>, String), PreviewError>;

    /// Imports one file from the caller's Drive.
    #[service_schema_op(mcp(interview(arm = "NeedsDriveConnection", url)))]
    async fn import_from_drive(
        &self,
        ctx: &Ctx,
        file_id: String,
    ) -> Result<ImportedDocument, ImportError>;

    /// Ships one document, asking how first.
    #[service_schema_op(mcp(interview(arm = "NeedsConfirmation", answers = shipping)))]
    async fn ship_document(
        &self,
        ctx: &Ctx,
        document_id: String,
        shipping: Option<ShippingForm>,
    ) -> Result<(), DeleteError>;
}

/// Answers every operation, and writes down what reached it.
#[derive(Default)]
pub struct Approvals {
    pub connected: AtomicBool,
    pub reached: Mutex<Vec<String>>,
}

impl Approvals {
    pub fn connect(&self) {
        self.connected.store(true, Ordering::Relaxed);
    }

    fn reach(&self, call: String) {
        self.reached.lock().unwrap().push(call);
    }
}

// Every method below is synchronous work wrapped in an already-ready `Future`: nothing here waits
// on anything.
impl ApprovalService<()> for Approvals {
    fn build_report(
        &self,
        _ctx: &(),
        req: ReportRequest,
    ) -> impl Future<Output = Result<Vec<approval_service_schema::McpContent>, PreviewError>> {
        use approval_service_schema::{McpContent, McpResourceContents};
        self.reach(format!("build-report {}", req.title));
        ready(Ok(vec![
            McpContent::Text { text: req.title },
            McpContent::Image {
                data: vec![0x89, b'P', b'N', b'G'],
                mime_type: "image/png".to_owned(),
            },
            McpContent::Audio {
                data: vec![1, 2, 3, 4, 5],
                mime_type: "audio/mpeg".to_owned(),
            },
            McpContent::ResourceLink {
                uri: "workspace://documents/doc-1".to_owned(),
                name: "id-card.pdf".to_owned(),
                title: Some("Id card".to_owned()),
                description: None,
                mime_type: Some("application/pdf".to_owned()),
            },
            McpContent::Resource {
                uri: "workspace://documents/doc-2".to_owned(),
                mime_type: Some("text/plain".to_owned()),
                contents: McpResourceContents::Text("passport".to_owned()),
            },
            McpContent::Resource {
                uri: "workspace://documents/doc-3".to_owned(),
                mime_type: None,
                contents: McpResourceContents::Blob(vec![9, 8, 7]),
            },
        ]))
    }

    fn delete_document(
        &self,
        _ctx: &(),
        document_id: String,
        confirmation: Option<DeleteConfirmation>,
    ) -> impl Future<Output = Result<(), DeleteError>> {
        self.reach(format!("delete-document {document_id} {confirmation:?}"));
        ready(match (document_id.as_str(), confirmation) {
            ("missing", _) => Err(DeleteError::NotFound),
            (_, None) => Err(DeleteError::NeedsConfirmation {
                message: "Delete this document for good?".to_owned(),
            }),
            (_, Some(DeleteConfirmation { confirm: true })) => Ok(()),
            (_, Some(DeleteConfirmation { confirm: false })) => Err(DeleteError::Refused),
        })
    }

    fn document_preview(
        &self,
        _ctx: &(),
        document_id: String,
    ) -> impl Future<Output = Result<(Vec<u8>, String), PreviewError>> {
        self.reach(format!("document-preview {document_id}"));
        ready(match document_id.as_str() {
            "doc-1" => Ok((vec![0x89, b'P', b'N', b'G'], "image/png".to_owned())),
            "voice" => Ok((vec![1, 2, 3, 4, 5], "audio/mpeg".to_owned())),
            "plain" => Ok((b"%PDF-".to_vec(), "application/pdf".to_owned())),
            _ => Err(PreviewError::NotFound),
        })
    }

    fn import_from_drive(
        &self,
        _ctx: &(),
        file_id: String,
    ) -> impl Future<Output = Result<ImportedDocument, ImportError>> {
        self.reach(format!("import-from-drive {file_id}"));
        ready(if self.connected.load(Ordering::Relaxed) {
            Ok(ImportedDocument {
                document_id: format!("doc-{file_id}"),
            })
        } else {
            Err(ImportError::NeedsDriveConnection {
                url: "https://mcp.example.com/connect/drive?request=7f3a".to_owned(),
                message: "Connect your Google Drive so this server can import the file.".to_owned(),
            })
        })
    }

    fn ship_document(
        &self,
        _ctx: &(),
        document_id: String,
        shipping: Option<ShippingForm>,
    ) -> impl Future<Output = Result<(), DeleteError>> {
        self.reach(format!("ship-document {document_id} {shipping:?}"));
        ready(shipping.map_or_else(
            || {
                Err(DeleteError::NeedsConfirmation {
                    message: "How should it ship?".to_owned(),
                })
            },
            |_form| Ok(()),
        ))
    }
}
