//! `AskingService`, whose operations ask the user with a form or a link and answer content
//! blocks, the Rust dispatcher that answers it, and the rows the TypeScript, Dart and Kotlin
//! servers are measured against and the clients are read against. Nothing here waits on a clock.

use super::mcp_asking_dispatcher as dispatcher;
use super::mcp_preview_dispatcher as preview_dispatcher;
use crate::mcp_streams::through;
use alloc::collections::BTreeMap;
use alloc::sync::Arc;
use core::future::{Future, ready};
use core::pin::pin;
use core::task::{Context, Poll, Waker};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::sync::Mutex;
use tixschema::{model_schema, service_schema};

/// The official 2026-07-28 schema of the Model Context Protocol.
const MCP_SCHEMA: &str = include_str!("../mcp_declaration_tests/mcp-schema-2026-07-28.json");

/// The link, and the messages, the stand asks with.
pub const LINK: &str = "https://mcp.example.com/connect/drive?request=7f3a";

pub const LINK_MESSAGE: &str = "Connect your Google Drive so this server can import the file.";

pub const CONFIRMATION_MESSAGE: &str = "Delete this document for good?";

pub const SHIPPING_MESSAGE: &str = "How should it ship?";

const NO_CAPABILITIES: &str = "none";

const FORM: &str = "form";

const LINK_CAPABLE: &str = "link";

const URL_ONLY: &str = "url";

/// The answer of a request whose validation fault is compared without its detail, which each
/// language words in its own terms.
const FAULTED_ANSWERS: [&str; 1] = ["form_invalid_answer"];

#[model_schema(enable = ["jsonschema"])]
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct AskingConfirmation {
    pub confirm: bool,
}

#[model_schema(enable = ["jsonschema"])]
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case", tag = "errorCode")]
pub enum AskingError {
    NeedsConfirmation { message: String },
    NotFound,
    Refused,
}

#[model_schema(enable = ["jsonschema"])]
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case", tag = "errorCode")]
pub enum AskingLinkError {
    NeedsConnection { message: String, url: String },
    NotFound,
}

#[model_schema(enable = ["jsonschema"])]
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "errorCode")]
pub enum AskingRenamedError {
    #[serde(rename = "connect-first")]
    NeedsConnection { message: String, url: String },
    #[serde(rename = "gone")]
    NotFound,
}

#[model_schema(enable = ["jsonschema"])]
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum AskingKindError {
    NeedsConnection { message: String, url: String },
    NotFound,
}

#[model_schema(enable = ["jsonschema"])]
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case", tag = "kind", content = "data")]
pub enum AskingAdjacentError {
    NeedsConfirmation { message: String },
    NeedsConnection { message: String, url: String },
    NotFound,
    Refused,
}

#[model_schema(enable = ["jsonschema"])]
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case", tag = "errorCode")]
pub enum AskingMiss {
    NotFound,
}

#[model_schema(enable = ["jsonschema"])]
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct AskingReportRequest {
    pub title: String,
}

#[model_schema(enable = ["jsonschema"])]
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct AskingLinked {
    pub document_id: String,
}

#[model_schema(enable = ["jsonschema"])]
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum AskingPriority {
    High,
    Low,
}

#[model_schema(enable = ["jsonschema"])]
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct AskingShipping {
    pub express: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    pub parcels: u32,
    pub priority: AskingPriority,
    pub tags: Vec<AskingPriority>,
}

/// Asks the user before it changes anything, and builds content for the model.
#[service_schema(transports = ["mcp"])]
pub trait AskingService<Ctx> {
    /// Answers a chart and a link to the full report.
    #[service_schema_op(mcp(content))]
    async fn build_report(
        &self,
        ctx: &Ctx,
        req: AskingReportRequest,
    ) -> Result<Vec<asking_service_schema::McpContent>, AskingMiss>;

    /// Imports one file from a Drive whose error holds its arm under a content key.
    #[service_schema_op(mcp(interview(arm = "NeedsConnection", url)))]
    async fn connect_adjacent(
        &self,
        ctx: &Ctx,
        file_id: String,
    ) -> Result<AskingLinked, AskingAdjacentError>;

    /// Imports one file from a Drive whose error is tagged by a key of its own.
    #[service_schema_op(mcp(interview(arm = "NeedsConnection", url)))]
    async fn connect_by_kind(
        &self,
        ctx: &Ctx,
        file_id: String,
    ) -> Result<AskingLinked, AskingKindError>;

    /// Imports one file from a Drive whose connection arm is renamed on the wire.
    #[service_schema_op(mcp(interview(arm = "NeedsConnection", url)))]
    async fn connect_renamed(
        &self,
        ctx: &Ctx,
        file_id: String,
    ) -> Result<AskingLinked, AskingRenamedError>;

    /// Deletes one document after the user confirms it, in an error that holds its arm under a
    /// content key.
    #[service_schema_op(mcp(interview(arm = "NeedsConfirmation", answers = confirmation)))]
    async fn delete_adjacent(
        &self,
        ctx: &Ctx,
        document_id: String,
        confirmation: Option<AskingConfirmation>,
    ) -> Result<(), AskingAdjacentError>;

    /// Deletes one document after the user confirms it.
    #[service_schema_op(mcp(interview(arm = "NeedsConfirmation", answers = confirmation)))]
    async fn delete_document(
        &self,
        ctx: &Ctx,
        document_id: String,
        confirmation: Option<AskingConfirmation>,
    ) -> Result<(), AskingError>;

    /// Imports one file from the caller's Drive.
    #[service_schema_op(mcp(interview(arm = "NeedsConnection", url)))]
    async fn import_from_drive(
        &self,
        ctx: &Ctx,
        file_id: String,
    ) -> Result<AskingLinked, AskingLinkError>;

    /// Ships one document, asking how first.
    #[service_schema_op(mcp(interview(arm = "NeedsConfirmation", answers = shipping)))]
    async fn ship_document(
        &self,
        ctx: &Ctx,
        document_id: String,
        shipping: Option<AskingShipping>,
    ) -> Result<(), AskingError>;
}

/// Answers a page of a document as the bytes of an image and their content type. A service of its
/// own, since a pair is read into a class the Kotlin accessors each number for themselves and into
/// a tuple Swift does not read.
#[service_schema(transports = ["mcp"])]
pub trait PreviewService<Ctx> {
    /// Answers the first page of one document.
    #[service_schema_op(mcp(content))]
    async fn document_preview(
        &self,
        ctx: &Ctx,
        document_id: String,
    ) -> Result<(Vec<u8>, String), AskingMiss>;
}

/// Answers every operation, and writes down what reached it.
#[derive(Clone, Default)]
pub struct Stand {
    pub reached: Arc<Mutex<Vec<Value>>>,
}

impl Stand {
    fn reach(&self, call: Value) {
        self.reached.lock().unwrap().push(call);
    }
}

// Every method below is synchronous work wrapped in an already-ready `Future`: nothing here waits
// on anything.
impl AskingService<()> for Stand {
    fn build_report(
        &self,
        _ctx: &(),
        req: AskingReportRequest,
    ) -> impl Future<Output = Result<Vec<asking_service_schema::McpContent>, AskingMiss>> {
        use asking_service_schema::{McpContent, McpResourceContents};
        self.reach(json!(["build-report", req.title]));
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

    fn connect_adjacent(
        &self,
        _ctx: &(),
        file_id: String,
    ) -> impl Future<Output = Result<AskingLinked, AskingAdjacentError>> {
        self.reach(json!(["connect-adjacent", file_id]));
        ready(Err(AskingAdjacentErrorDraft::NeedsConnection {
            url: LINK.to_owned(),
            message: LINK_MESSAGE.to_owned(),
        }))
    }

    fn connect_by_kind(
        &self,
        _ctx: &(),
        file_id: String,
    ) -> impl Future<Output = Result<AskingLinked, AskingKindError>> {
        self.reach(json!(["connect-by-kind", file_id]));
        ready(Err(AskingKindErrorDraft::NeedsConnection {
            url: LINK.to_owned(),
            message: LINK_MESSAGE.to_owned(),
        }))
    }

    fn connect_renamed(
        &self,
        _ctx: &(),
        file_id: String,
    ) -> impl Future<Output = Result<AskingLinked, AskingRenamedError>> {
        self.reach(json!(["connect-renamed", file_id]));
        ready(Err(AskingRenamedErrorDraft::NeedsConnection {
            url: LINK.to_owned(),
            message: LINK_MESSAGE.to_owned(),
        }))
    }

    fn delete_adjacent(
        &self,
        _ctx: &(),
        document_id: String,
        confirmation: Option<AskingConfirmation>,
    ) -> impl Future<Output = Result<(), AskingAdjacentError>> {
        self.reach(json!(["delete-adjacent", document_id, confirmation]));
        ready(match (document_id.as_str(), confirmation) {
            ("missing", _) => Err(AskingAdjacentErrorDraft::NotFound),
            (_, None) => Err(AskingAdjacentErrorDraft::NeedsConfirmation {
                message: CONFIRMATION_MESSAGE.to_owned(),
            }),
            (_, Some(AskingConfirmation { confirm: true })) => Ok(()),
            (_, Some(AskingConfirmation { confirm: false })) => {
                Err(AskingAdjacentErrorDraft::Refused)
            }
        })
    }

    fn delete_document(
        &self,
        _ctx: &(),
        document_id: String,
        confirmation: Option<AskingConfirmation>,
    ) -> impl Future<Output = Result<(), AskingError>> {
        self.reach(json!(["delete-document", document_id, confirmation]));
        ready(match (document_id.as_str(), confirmation) {
            ("missing", _) => Err(AskingErrorDraft::NotFound),
            (_, None) => Err(AskingErrorDraft::NeedsConfirmation {
                message: CONFIRMATION_MESSAGE.to_owned(),
            }),
            (_, Some(AskingConfirmation { confirm: true })) => Ok(()),
            (_, Some(AskingConfirmation { confirm: false })) => Err(AskingErrorDraft::Refused),
        })
    }

    fn import_from_drive(
        &self,
        _ctx: &(),
        file_id: String,
    ) -> impl Future<Output = Result<AskingLinked, AskingLinkError>> {
        self.reach(json!(["import-from-drive", file_id]));
        ready(if file_id == "connected" {
            Ok(AskingLinkedDraft {
                document_id: format!("doc-{file_id}"),
            })
        } else {
            Err(AskingLinkErrorDraft::NeedsConnection {
                url: LINK.to_owned(),
                message: LINK_MESSAGE.to_owned(),
            })
        })
    }

    fn ship_document(
        &self,
        _ctx: &(),
        document_id: String,
        shipping: Option<AskingShipping>,
    ) -> impl Future<Output = Result<(), AskingError>> {
        self.reach(json!(["ship-document", document_id, shipping]));
        ready(shipping.map_or_else(
            || {
                Err(AskingErrorDraft::NeedsConfirmation {
                    message: SHIPPING_MESSAGE.to_owned(),
                })
            },
            |_form| Ok(()),
        ))
    }
}

impl PreviewService<()> for Stand {
    fn document_preview(
        &self,
        _ctx: &(),
        document_id: String,
    ) -> impl Future<Output = Result<(Vec<u8>, String), AskingMiss>> {
        self.reach(json!(["document-preview", document_id]));
        ready(match document_id.as_str() {
            "doc-1" => Ok((vec![0x89, b'P', b'N', b'G'], "image/png".to_owned())),
            "voice" => Ok((vec![1, 2, 3, 4, 5], "audio/mpeg".to_owned())),
            "plain" => Ok((b"%PDF-".to_vec(), "application/pdf".to_owned())),
            _ => Err(AskingMissDraft::NotFound),
        })
    }
}

/// A write that is never made: these runs compare the answer alone.
struct Nowhere;

impl dispatcher::McpEventWriter for Nowhere {
    fn write(
        &self,
        _message: String,
    ) -> impl Future<Output = Result<(), dispatcher::McpClientGone>> {
        ready(Ok(()))
    }
}

/// The same, for the preview service.
struct PreviewNowhere;

impl preview_dispatcher::McpEventWriter for PreviewNowhere {
    fn write(
        &self,
        _message: String,
    ) -> impl Future<Output = Result<(), preview_dispatcher::McpClientGone>> {
        ready(Ok(()))
    }
}

/// One call a client makes: the answer of the Rust dispatcher it is handed, the tool it must ask
/// for with the arguments it must write, and the outcome it must read.
pub struct ClientCall {
    pub answer: &'static str,
    pub arguments: Value,
    pub expected: Value,
    pub label: &'static str,
    pub tool: &'static str,
}

fn capabilities(named: &str) -> Value {
    match named {
        FORM => json!({ "elicitation": { "form": {} } }),
        LINK_CAPABLE => json!({ "elicitation": { "form": {}, "url": {} } }),
        URL_ONLY => json!({ "elicitation": { "url": {} } }),
        _ => json!({}),
    }
}

/// One `tools/call` of a client that declared `capabilities`.
fn tool_call(tool: &str, arguments: &Value, named: &str, responses: Option<Value>) -> String {
    let mut params = json!({
        "name": tool,
        "arguments": arguments,
        "_meta": {
            "io.modelcontextprotocol/protocolVersion": "2026-07-28",
            "io.modelcontextprotocol/clientCapabilities": capabilities(named),
        },
    });
    if let Some(held) = responses {
        params["inputResponses"] = held;
    }
    json!({ "jsonrpc": "2.0", "id": 1, "method": "tools/call", "params": params }).to_string()
}

fn tools_list() -> String {
    json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/list",
        "params": { "_meta": {
            "io.modelcontextprotocol/protocolVersion": "2026-07-28",
            "io.modelcontextprotocol/clientCapabilities": {},
        } },
    })
    .to_string()
}

/// The requests of the forms, by name.
fn form_requests() -> Vec<(&'static str, String)> {
    let delete = json!({ "documentId": "doc-1" });
    let accepted =
        json!({ "confirmation": { "action": "accept", "content": { "confirm": true } } });
    vec![
        ("tools_list", tools_list()),
        (
            "form_first",
            tool_call("delete-document", &delete, FORM, None),
        ),
        (
            "form_accepted",
            tool_call("delete-document", &delete, FORM, Some(accepted)),
        ),
        (
            "form_declined",
            tool_call(
                "delete-document",
                &delete,
                FORM,
                Some(json!({ "confirmation": { "action": "decline" } })),
            ),
        ),
        (
            "form_cancelled",
            tool_call(
                "delete-document",
                &delete,
                FORM,
                Some(json!({ "confirmation": { "action": "cancel" } })),
            ),
        ),
        (
            "form_without_elicitation",
            tool_call("delete-document", &delete, NO_CAPABILITIES, None),
        ),
        (
            "form_with_url_only",
            tool_call("delete-document", &delete, URL_ONLY, None),
        ),
        (
            "form_invalid_answer",
            tool_call(
                "delete-document",
                &delete,
                FORM,
                Some(
                    json!({ "confirmation": { "action": "accept", "content": { "confirm": "yes" } } }),
                ),
            ),
        ),
        (
            "form_answer_in_arguments",
            tool_call(
                "delete-document",
                &json!({ "documentId": "doc-1", "confirmation": { "confirm": true } }),
                NO_CAPABILITIES,
                None,
            ),
        ),
        (
            "form_refused_in_arguments",
            tool_call(
                "delete-document",
                &json!({ "documentId": "doc-1", "confirmation": { "confirm": false } }),
                NO_CAPABILITIES,
                None,
            ),
        ),
        (
            "form_missing_document",
            tool_call(
                "delete-document",
                &json!({ "documentId": "missing" }),
                FORM,
                None,
            ),
        ),
    ]
}

/// The requests that ask how to ship, by name.
fn ship_requests() -> Vec<(&'static str, String)> {
    let ship = json!({ "documentId": "doc-1" });
    vec![
        ("ship_first", tool_call("ship-document", &ship, FORM, None)),
        (
            "ship_accepted",
            tool_call(
                "ship-document",
                &ship,
                FORM,
                Some(json!({ "shipping": { "action": "accept", "content": {
                    "express": true, "parcels": 2_u8, "priority": "High", "tags": ["Low", "High"]
                } } })),
            ),
        ),
    ]
}

/// The requests of the links, by name.
fn link_requests() -> Vec<(&'static str, String)> {
    let link = json!({ "value": "1A2b3C" });
    let accepted_link = json!({ "needs-connection": { "action": "accept" } });
    vec![
        (
            "link_first",
            tool_call("import-from-drive", &link, LINK_CAPABLE, None),
        ),
        (
            "link_again",
            tool_call(
                "import-from-drive",
                &link,
                LINK_CAPABLE,
                Some(accepted_link),
            ),
        ),
        (
            "link_connected",
            tool_call(
                "import-from-drive",
                &json!({ "value": "connected" }),
                LINK_CAPABLE,
                Some(json!({ "needs-connection": { "action": "accept" } })),
            ),
        ),
        (
            "link_declined",
            tool_call(
                "import-from-drive",
                &link,
                LINK_CAPABLE,
                Some(json!({ "needs-connection": { "action": "decline" } })),
            ),
        ),
        (
            "link_without_url_mode",
            tool_call("import-from-drive", &link, FORM, None),
        ),
        (
            "link_without_elicitation",
            tool_call("import-from-drive", &link, NO_CAPABILITIES, None),
        ),
    ]
}

/// The requests that ask by a link whose arm is renamed, or by an error tagged by a key of its own
/// or holding its arm under a content key.
fn renamed_requests() -> Vec<(&'static str, String)> {
    let link = json!({ "value": "1A2b3C" });
    let delete = json!({ "documentId": "doc-1" });
    vec![
        (
            "renamed_first",
            tool_call("connect-renamed", &link, LINK_CAPABLE, None),
        ),
        (
            "renamed_dismissed",
            tool_call(
                "connect-renamed",
                &link,
                LINK_CAPABLE,
                Some(json!({ "connect-first": { "action": "decline" } })),
            ),
        ),
        (
            "adjacent_link_first",
            tool_call("connect-adjacent", &link, LINK_CAPABLE, None),
        ),
        (
            "adjacent_link_dismissed",
            tool_call(
                "connect-adjacent",
                &link,
                LINK_CAPABLE,
                Some(json!({ "needs-connection": { "action": "decline" } })),
            ),
        ),
        (
            "adjacent_link_without_url_mode",
            tool_call("connect-adjacent", &link, FORM, None),
        ),
        (
            "adjacent_form_first",
            tool_call("delete-adjacent", &delete, FORM, None),
        ),
        (
            "adjacent_form_accepted",
            tool_call(
                "delete-adjacent",
                &delete,
                FORM,
                Some(
                    json!({ "confirmation": { "action": "accept", "content": { "confirm": true } } }),
                ),
            ),
        ),
        (
            "adjacent_form_declined",
            tool_call(
                "delete-adjacent",
                &delete,
                FORM,
                Some(json!({ "confirmation": { "action": "decline" } })),
            ),
        ),
        (
            "adjacent_form_without_elicitation",
            tool_call("delete-adjacent", &delete, NO_CAPABILITIES, None),
        ),
        (
            "adjacent_form_missing_document",
            tool_call(
                "delete-adjacent",
                &json!({ "documentId": "missing" }),
                FORM,
                None,
            ),
        ),
        (
            "kind_first",
            tool_call("connect-by-kind", &link, LINK_CAPABLE, None),
        ),
        (
            "kind_dismissed",
            tool_call(
                "connect-by-kind",
                &link,
                LINK_CAPABLE,
                Some(json!({ "needs_connection": { "action": "cancel" } })),
            ),
        ),
    ]
}

/// The requests of the content tools, by name.
fn content_requests() -> Vec<(&'static str, String)> {
    vec![
        (
            "preview_image",
            tool_call(
                "document-preview",
                &json!({ "value": "doc-1" }),
                NO_CAPABILITIES,
                None,
            ),
        ),
        (
            "preview_audio",
            tool_call(
                "document-preview",
                &json!({ "value": "voice" }),
                NO_CAPABILITIES,
                None,
            ),
        ),
        (
            "preview_plain",
            tool_call(
                "document-preview",
                &json!({ "value": "plain" }),
                NO_CAPABILITIES,
                None,
            ),
        ),
        (
            "preview_missing",
            tool_call(
                "document-preview",
                &json!({ "value": "nothing" }),
                NO_CAPABILITIES,
                None,
            ),
        ),
        (
            "report",
            tool_call(
                "build-report",
                &json!({ "title": "Q3" }),
                NO_CAPABILITIES,
                None,
            ),
        ),
    ]
}

/// The requests every server is sent, by name, in the order they are answered.
pub fn requests() -> Vec<(&'static str, String)> {
    [
        form_requests(),
        ship_requests(),
        link_requests(),
        renamed_requests(),
        content_requests(),
    ]
    .concat()
}

/// Whether the request asks for the tool of the preview service.
fn is_preview(text: &str) -> bool {
    let asked: Value = serde_json::from_str(text).unwrap();
    asked["params"]["name"] == "document-preview"
}

/// What the Rust dispatcher answers a request, and what reached its handler.
fn answered(text: &str) -> (String, Vec<Value>) {
    let stand = Stand::default();
    let reply = if is_preview(text) {
        ready_output(preview_dispatcher::answer(
            text,
            &stand,
            |_call| (),
            &PreviewNowhere,
            &preview_dispatcher::McpInFlight::default(),
        ))
    } else {
        ready_output(dispatcher::answer(
            text,
            &stand,
            |_call| (),
            &Nowhere,
            &dispatcher::McpInFlight::default(),
        ))
    }
    .unwrap();
    let reached = stand.reached.lock().unwrap().clone();
    (reply, reached)
}

/// What the Rust dispatcher answers every request, and the calls that reached its handlers.
pub fn rust_report() -> Value {
    let mut answers = BTreeMap::new();
    let mut reached = BTreeMap::new();
    for (name, text) in requests() {
        let (reply, calls) = answered(&text);
        answers.insert(name, reply);
        reached.insert(name, calls);
    }
    json!({ "answers": answers, "reached": reached })
}

/// What the Rust side hands each language's program.
pub fn input() -> Value {
    let requests: BTreeMap<_, _> = requests().into_iter().collect();
    let previews: Vec<&str> = requests
        .iter()
        .filter(|(_, text)| is_preview(text))
        .map(|(name, _)| *name)
        .collect();
    json!({ "requests": requests, "previews": previews })
}

/// Holds `message` to the definition of the official schema named `definition`.
fn held_to(definition: &str, message: &Value) {
    let schema: Value = serde_json::from_str(MCP_SCHEMA).unwrap();
    let rooted = json!({ "$ref": format!("#/$defs/{definition}"), "$defs": schema["$defs"] });
    let validator = jsonschema::validator_for(&rooted).unwrap();
    let errors: Vec<String> = validator
        .iter_errors(message)
        .map(|error| format!("{error} at {}", error.instance_path()))
        .collect();
    assert!(
        errors.is_empty(),
        "{definition}: {errors:#?}\nin {message:#}"
    );
}

/// `value` with every number read as a float, since a language that writes `1` for `1.0` writes
/// the same JSON.
fn floats(value: &Value) -> Value {
    match value {
        Value::Number(number) => json!(number.as_f64().unwrap()),
        Value::Array(items) => Value::Array(items.iter().map(floats).collect()),
        Value::Object(members) => Value::Object(
            members
                .iter()
                .map(|(key, held)| (key.clone(), floats(held)))
                .collect(),
        ),
        Value::Null | Value::Bool(_) | Value::String(_) => value.clone(),
    }
}

fn read(text: &str) -> Value {
    floats(&serde_json::from_str(text).unwrap())
}

/// The definition of the official schema an answer to the request `name` is held to.
fn definition_of(name: &str, answer: &Value) -> &'static str {
    match name {
        "tools_list" => "JSONRPCResultResponse",
        _ if answer["result"]["resultType"] == "input_required" => "InputRequiredResult",
        _ => "CallToolResult",
    }
}

fn without_detail(answer: &Value) -> Value {
    let mut stripped = answer.clone();
    let text = stripped["result"]["content"][0]["text"].as_str().unwrap();
    let mut body: Value = serde_json::from_str(text).unwrap();
    assert_eq!(body["isServiceFault"], true, "{answer:#}");
    body["fault"].as_object_mut().unwrap().remove("detail");
    stripped["result"]["content"][0]["text"] = json!(body.to_string());
    stripped
}

/// Holds the report of a language's program to what the Rust dispatcher answered, and every
/// answer to the official schema.
pub fn assert_matches(report: &Value) {
    assert_matches_except(report, &[]);
}

/// [`assert_matches`] for a server that answers the requests but the `skipped` ones.
pub fn assert_matches_except(report: &Value, skipped: &[&str]) {
    let rust = rust_report();
    for (name, _) in requests() {
        if skipped.contains(&name) {
            continue;
        }
        let found = report["answers"][name].as_str();
        assert!(found.is_some(), "{name} was not answered: {report:#}");
        let answered = found.unwrap();
        let expected = rust["answers"][name].as_str().unwrap();
        let (got, wanted) = (read(answered), read(expected));
        if FAULTED_ANSWERS.contains(&name) {
            assert_eq!(without_detail(&got), without_detail(&wanted), "{name}");
        } else {
            assert_eq!(got, wanted, "{name}");
        }
        let message: Value = serde_json::from_str(answered).unwrap();
        held_to("JSONRPCResultResponse", &message);
        if name != "tools_list" {
            held_to(definition_of(name, &message), &message["result"]);
        }
        assert_eq!(
            report["reached"][name], rust["reached"][name],
            "{name} reached"
        );
    }
}

fn report_blocks() -> Value {
    read(&answered(&requests().iter().find(|row| row.0 == "report").unwrap().1).0)["result"]
        ["content"]
        .clone()
}

fn client_calls() -> Vec<ClientCall> {
    vec![
        ClientCall {
            answer: "form_without_elicitation",
            arguments: json!({ "documentId": "doc-1" }),
            expected: json!({ "declared": {
                "errorCode": "needs-confirmation",
                "message": CONFIRMATION_MESSAGE,
            } }),
            label: "declared",
            tool: "delete-document",
        },
        ClientCall {
            answer: "form_answer_in_arguments",
            arguments: json!({ "documentId": "doc-1", "confirmation": { "confirm": true } }),
            expected: json!({ "ok": null }),
            label: "confirmed",
            tool: "delete-document",
        },
        ClientCall {
            answer: "preview_image",
            arguments: json!({ "value": "doc-1" }),
            expected: json!({ "ok": { "bytes": [137, 80, 78, 71], "contentType": "image/png" } }),
            label: "image",
            tool: "document-preview",
        },
        ClientCall {
            answer: "preview_audio",
            arguments: json!({ "value": "voice" }),
            expected: json!({ "ok": { "bytes": [1, 2, 3, 4, 5], "contentType": "audio/mpeg" } }),
            label: "audio",
            tool: "document-preview",
        },
        ClientCall {
            answer: "preview_plain",
            arguments: json!({ "value": "plain" }),
            expected: json!({ "ok": { "bytes": [37, 80, 68, 70, 45], "contentType": "application/pdf" } }),
            label: "plain",
            tool: "document-preview",
        },
        ClientCall {
            answer: "preview_missing",
            arguments: json!({ "value": "nothing" }),
            expected: json!({ "declared": { "errorCode": "not-found" } }),
            label: "missing",
            tool: "document-preview",
        },
        ClientCall {
            answer: "report",
            arguments: json!({ "title": "Q3" }),
            expected: json!({ "ok": report_blocks() }),
            label: "report",
            tool: "build-report",
        },
        ClientCall {
            answer: "link_without_elicitation",
            arguments: json!({ "value": "1A2b3C" }),
            expected: json!({ "declared": {
                "errorCode": "needs-connection",
                "message": LINK_MESSAGE,
                "url": LINK,
            } }),
            label: "link",
            tool: "import-from-drive",
        },
        ClientCall {
            answer: "link_connected",
            arguments: json!({ "value": "connected" }),
            expected: json!({ "ok": { "document_id": "doc-connected" } }),
            label: "linked",
            tool: "import-from-drive",
        },
    ]
}

/// What the Rust side hands each language's client program: the answers it is handed, in order.
pub fn client_input() -> Value {
    let report = rust_report();
    json!({
        "calls": client_calls().iter().map(|call| json!({
            "label": call.label,
            "answer": report["answers"][call.answer],
        })).collect::<Vec<_>>(),
    })
}

/// `value` without the members that are `null`, which a language may write for an absent option.
fn without_nulls(value: &Value) -> Value {
    match value {
        Value::Object(members) => Value::Object(
            members
                .iter()
                .filter(|(_, held)| !held.is_null())
                .map(|(key, held)| (key.clone(), without_nulls(held)))
                .collect(),
        ),
        Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_) | Value::Array(_) => {
            value.clone()
        }
    }
}

/// Holds the outcomes a client read, and the requests it sent, to the calls above.
pub fn assert_client(report: &Value) {
    let calls = client_calls();
    let sent = report["seen"].as_array().unwrap();
    assert_eq!(sent.len(), calls.len(), "{report:#}");
    for (call, request) in calls.iter().zip(sent) {
        held_to("CallToolRequest", request);
        assert_eq!(request["params"]["name"], call.tool, "{}", call.label);
        assert_eq!(
            without_nulls(&request["params"]["arguments"]),
            call.arguments,
            "{}",
            call.label
        );
        assert_eq!(
            request["params"]["_meta"]["io.modelcontextprotocol/clientCapabilities"],
            json!({}),
            "{}",
            call.label
        );
        assert_eq!(
            floats(&report["outcomes"][call.label]),
            floats(&call.expected),
            "{}",
            call.label
        );
    }
}

#[test]
fn the_rust_dispatcher_answers_alone_beside_an_endpoint_and_over_http_alike() {
    let (alone, served, status) = rust_serves_alike!(
        dispatcher,
        AskingServiceSchema,
        Stand::default(),
        ready_output
    );
    assert_eq!(alone, served);
    assert!(alone.is_some());
    assert_eq!(status, 200);
}

#[test]
fn the_rust_dispatcher_listens_alike_through_every_entry_point() {
    let (alone, served) = rust_listens_alike!(
        dispatcher,
        AskingServiceSchema,
        Stand::default(),
        ready_output
    );
    assert_eq!(alone, served);
    assert!(alone.is_some());
}

#[test]
fn an_admit_that_denies_stops_the_request_before_any_handler_runs() {
    let settings = dispatcher::McpHttpSettings::new(&[], None);
    let (_, text) = requests().remove(1);
    let stand = Stand::default();
    let asked = dispatcher::McpHttpRequest::new(
        "POST".to_owned(),
        "/mcp".to_owned(),
        vec![
            ("MCP-Protocol-Version".to_owned(), "2026-07-28".to_owned()),
            ("Mcp-Method".to_owned(), "tools/call".to_owned()),
            ("Mcp-Name".to_owned(), "delete-document".to_owned()),
        ],
        text.into_bytes(),
    );
    for (denied, status) in [
        (dispatcher::McpDenied::Forbidden, 403_u16),
        (
            dispatcher::McpDenied::InsufficientScope {
                scope: "documents:write".to_owned(),
            },
            403,
        ),
        (
            dispatcher::McpDenied::NoValidCredentials { scope: None },
            401,
        ),
    ] {
        let response = through(|stream| {
            ready_output(dispatcher::answer_http(
                &asked,
                &settings,
                stream,
                &stand,
                async |request: &dispatcher::McpHttpRequest, _call: dispatcher::McpCall| {
                    assert_eq!(request.mcp_name().as_deref(), Some("delete-document"));
                    assert_eq!(request.bearer_token(), None);
                    assert_eq!(request.header("mcp-method"), Some("tools/call"));
                    Err::<dispatcher::McpAdmitted<()>, _>(denied.clone())
                },
            ))
        });
        assert_eq!(response.status(), status);
    }
    assert!(stand.reached.lock().unwrap().is_empty());
}

#[test]
fn the_rust_preview_dispatcher_answers_alone_beside_an_endpoint_and_over_http_alike() {
    let (alone, served, status) = rust_serves_alike!(
        preview_dispatcher,
        PreviewServiceSchema,
        Stand::default(),
        ready_output
    );
    assert_eq!(alone, served);
    assert!(alone.is_some());
    assert_eq!(status, 200);
}

#[test]
fn the_rust_preview_dispatcher_listens_alike_through_every_entry_point() {
    let (alone, served) = rust_listens_alike!(
        preview_dispatcher,
        PreviewServiceSchema,
        Stand::default(),
        ready_output
    );
    assert_eq!(alone, served);
    assert!(alone.is_some());
}

#[test]
fn an_admit_that_denies_stops_a_preview_before_its_handler_runs() {
    let settings = preview_dispatcher::McpHttpSettings::new(&[], None);
    let (_, text) = requests()
        .into_iter()
        .find(|(name, _)| *name == "preview_image")
        .unwrap();
    let stand = Stand::default();
    let asked = preview_dispatcher::McpHttpRequest::new(
        "POST".to_owned(),
        "/mcp".to_owned(),
        vec![
            ("MCP-Protocol-Version".to_owned(), "2026-07-28".to_owned()),
            ("Mcp-Method".to_owned(), "tools/call".to_owned()),
            ("Mcp-Name".to_owned(), "document-preview".to_owned()),
        ],
        text.into_bytes(),
    );
    for (denied, status) in [
        (preview_dispatcher::McpDenied::Forbidden, 403_u16),
        (
            preview_dispatcher::McpDenied::InsufficientScope {
                scope: "documents:read".to_owned(),
            },
            403,
        ),
        (
            preview_dispatcher::McpDenied::NoValidCredentials { scope: None },
            401,
        ),
    ] {
        let response = through(|stream| {
            ready_output(preview_dispatcher::answer_http(
                &asked,
                &settings,
                stream,
                &stand,
                async |request: &preview_dispatcher::McpHttpRequest,
                       _call: preview_dispatcher::McpCall| {
                    assert_eq!(request.mcp_name().as_deref(), Some("document-preview"));
                    assert_eq!(request.bearer_token(), None);
                    assert_eq!(request.header("mcp-method"), Some("tools/call"));
                    Err::<preview_dispatcher::McpAdmitted<()>, _>(denied.clone())
                },
            ))
        });
        assert_eq!(response.status(), status);
    }
    assert!(stand.reached.lock().unwrap().is_empty());
}

/// What `answering` answers on its first poll.
fn ready_output<Answering>(answering: Answering) -> Answering::Output
where
    Answering: Future,
{
    let mut pinned = pin!(answering);
    match pinned
        .as_mut()
        .poll(&mut Context::from_waker(Waker::noop()))
    {
        Poll::Ready(output) => Some(output),
        Poll::Pending => None,
    }
    .unwrap()
}

#[test]
fn an_arm_asks_under_the_name_and_the_tag_serde_gives_it() {
    let report = rust_report();
    let answer = |name: &str| -> Value {
        serde_json::from_str(report["answers"][name].as_str().unwrap()).unwrap()
    };
    for (request, key) in [
        ("renamed_first", "connect-first"),
        ("kind_first", "needs_connection"),
        ("adjacent_link_first", "needs-connection"),
        ("adjacent_form_first", "confirmation"),
    ] {
        let asked = answer(request);
        assert_eq!(asked["result"]["resultType"], "input_required", "{asked:#}");
        let keys: Vec<&String> = asked["result"]["inputRequests"]
            .as_object()
            .unwrap()
            .keys()
            .collect();
        assert_eq!(keys, [key], "{asked:#}");
    }
    for request in [
        "renamed_dismissed",
        "kind_dismissed",
        "adjacent_link_dismissed",
        "adjacent_form_declined",
    ] {
        let declined = answer(request);
        assert_eq!(declined["result"]["isError"], true, "{declined:#}");
    }
}
