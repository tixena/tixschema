//! Services that ask for `mcp`, and the tools each lists.

#![cfg(feature = "serde")]

use core::future::{Future, ready};
use core::pin::pin;
use core::task::{Context, Poll, Waker};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tixschema::{model_schema, service_schema};

/// The official 2026-07-28 schema of the Model Context Protocol.
const MCP_SCHEMA: &str = include_str!("mcp-schema-2026-07-28.json");

/// Answers every operation of every service with its first answer.
struct Archive;

#[model_schema(enable = ["jsonschema"])]
#[derive(Debug, Deserialize, Serialize)]
pub struct GetVersionRequest {
    /// The document whose version is asked for.
    #[model_schema_prop(minLength = 1)]
    pub document_id: String,
}

#[model_schema(enable = ["jsonschema"])]
#[derive(Debug, Deserialize, Serialize)]
pub struct VersionResponse {
    /// The version the document is at.
    pub version: u32,
}

#[model_schema(enable = ["jsonschema"])]
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case", tag = "errorCode")]
pub enum VersionError {
    NotFound,
}

#[model_schema(enable = ["jsonschema"])]
#[derive(Debug, Deserialize, Serialize)]
#[serde(transparent)]
pub struct DocumentId(String);

/// Reads and renames documents in the caller's workspace.
#[service_schema(transports = ["mcp"])]
pub trait DocumentService<Ctx> {
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
    ) -> Result<(VersionResponse, String), VersionError>;

    /// Answers the current version of one document.
    #[service_schema_op(mcp(title = "Current Document Version"))]
    async fn get_version(
        &self,
        ctx: &Ctx,
        req: GetVersionRequest,
    ) -> Result<VersionResponse, VersionError>;

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

    /// Marks one document as read.
    #[service_schema_op(one_way)]
    async fn touch_document(&self, ctx: &Ctx, document_id: String, reader: String);
}

/// A service that asks for two transports.
#[service_schema(transports = ["mcp", "ws_rpc"])]
pub trait TwoTransportService<Ctx> {
    /// Answers the current version of one document.
    async fn get_version(
        &self,
        ctx: &Ctx,
        req: GetVersionRequest,
    ) -> Result<VersionResponse, VersionError>;
}

/// A service whose tools carry annotations, an icon and a marked argument.
#[service_schema(transports = ["mcp"])]
pub trait AnnotatedService<Ctx> {
    /// Answers the version of one document.
    #[service_schema_op(mcp(destructive = true, open_world = true))]
    async fn get_version(
        &self,
        ctx: &Ctx,
        req: GetVersionRequest,
    ) -> Result<VersionResponse, VersionError>;

    /// Retitles one document.
    #[service_schema_op(mcp(
        read_only,
        destructive = false,
        idempotent,
        open_world = false,
        icon = "https://example.com/rename.png",
        header(document_id = "Document-Id")
    ))]
    async fn retitle_document(
        &self,
        ctx: &Ctx,
        document_id: String,
        title: String,
    ) -> Result<(), VersionError>;
}

impl DocumentService<()> for Archive {
    async fn get_tagged(
        &self,
        _ctx: &(),
        _document_id: String,
        _known_tag: Option<String>,
    ) -> Result<(VersionResponse, String), VersionError> {
        ready(()).await;
        Err(VersionError::NotFound)
    }

    async fn get_version(
        &self,
        _ctx: &(),
        _req: GetVersionRequest,
    ) -> Result<VersionResponse, VersionError> {
        ready(()).await;
        Err(VersionError::NotFound)
    }

    async fn lookup(&self, _ctx: &(), _id: DocumentId) -> Result<VersionResponse, VersionError> {
        ready(()).await;
        Err(VersionError::NotFound)
    }

    async fn rename_document(
        &self,
        _ctx: &(),
        _document_id: String,
        _title: String,
    ) -> Result<(), VersionError> {
        ready(()).await;
        Err(VersionError::NotFound)
    }

    async fn touch_document(&self, _ctx: &(), _document_id: String, _reader: String) {
        ready(()).await;
    }
}

impl AnnotatedService<()> for Archive {
    async fn get_version(
        &self,
        _ctx: &(),
        _req: GetVersionRequest,
    ) -> Result<VersionResponse, VersionError> {
        ready(()).await;
        Err(VersionError::NotFound)
    }

    async fn retitle_document(
        &self,
        _ctx: &(),
        _document_id: String,
        _title: String,
    ) -> Result<(), VersionError> {
        ready(()).await;
        Ok(())
    }
}

impl TwoTransportService<()> for Archive {
    async fn get_version(
        &self,
        _ctx: &(),
        _req: GetVersionRequest,
    ) -> Result<VersionResponse, VersionError> {
        ready(()).await;
        Err(VersionError::NotFound)
    }
}

/// Every error the vendored `Tool` definition finds in `tool`, each with where it is.
fn tool_errors(tool: &Value) -> Vec<String> {
    let schema: Value = serde_json::from_str(MCP_SCHEMA).unwrap();
    let rooted = json!({ "$ref": "#/$defs/Tool", "$defs": schema["$defs"] });
    let validator = jsonschema::validator_for(&rooted).unwrap();
    validator
        .iter_errors(tool)
        .map(|error| format!("{error} at {}", error.instance_path()))
        .collect()
}

fn assert_tool(tool: &Value) {
    let errors = tool_errors(tool);
    assert!(errors.is_empty(), "{errors:#?}\n{tool:#}");
}

fn tool_named<'tools>(tools: &'tools [Value], name: &str) -> &'tools Value {
    tools.iter().find(|tool| tool["name"] == name).unwrap()
}

#[test]
fn the_vendored_schema_compiles_and_refuses_a_tool_with_a_string_input() {
    assert_tool(&json!({ "name": "ok", "inputSchema": { "type": "object" } }));
    assert_eq!(
        tool_errors(&json!({ "name": "no", "inputSchema": { "type": "string" } })).len(),
        1
    );
}

#[test]
fn a_service_lists_one_tool_per_operation_in_declaration_order() {
    let tools = crate::DocumentServiceSchema::mcp_tools();
    let names: Vec<&Value> = tools.iter().map(|tool| &tool["name"]).collect();
    assert_eq!(
        names,
        [
            "get-tagged",
            "get-version",
            "lookup-document",
            "rename-document",
            "touch-document"
        ]
    );
    assert_eq!(
        crate::DocumentServiceSchema::mcp_tools()[0]["name"],
        "get-tagged"
    );
}

#[test]
fn every_tool_passes_the_tool_definition_of_the_official_schema() {
    for tool in crate::DocumentServiceSchema::mcp_tools() {
        assert_tool(&tool);
    }
    for tool in crate::AnnotatedServiceSchema::mcp_tools() {
        assert_tool(&tool);
    }
    for tool in crate::TwoTransportServiceSchema::mcp_tools() {
        assert_tool(&tool);
    }
}

#[test]
fn the_title_is_the_one_written_or_the_method_in_title_case() {
    let tools = crate::DocumentServiceSchema::mcp_tools();
    let titles: Vec<&Value> = tools.iter().map(|tool| &tool["title"]).collect();
    assert_eq!(
        titles,
        [
            "Get Tagged",
            "Current Document Version",
            "Lookup",
            "Rename Document",
            "Touch Document"
        ]
    );
    assert_eq!(
        crate::TwoTransportServiceSchema::mcp_tools()[0]["title"],
        "Get Version"
    );
}

#[test]
fn the_description_is_the_rustdoc_of_the_method() {
    let tools = crate::DocumentServiceSchema::mcp_tools();
    assert_eq!(
        tool_named(&tools, "get-version")["description"],
        "Answers the current version of one document."
    );
    assert_eq!(
        tools[0]["description"],
        "Answers the current version of one document, unless it still carries the tag the caller\n\
         knows."
    );
}

#[test]
fn a_message_the_operation_names_is_the_arguments_object_itself() {
    let tools = crate::DocumentServiceSchema::mcp_tools();
    let version = tool_named(&tools, "get-version");
    assert_eq!(
        version["inputSchema"],
        json!({
            "additionalProperties": false,
            "properties": { "document_id": {
                "description": "The document whose version is asked for.",
                "minLength": 1_u32,
                "type": "string"
            } },
            "required": ["document_id"],
            "type": "object"
        })
    );
    assert_eq!(
        version["outputSchema"],
        json!({
            "additionalProperties": false,
            "properties": { "version": {
                "description": "The version the document is at.",
                "type": "integer"
            } },
            "required": ["version"],
            "type": "object"
        })
    );
}

#[test]
fn a_generated_message_is_the_arguments_object_and_a_unit_success_has_no_output_schema() {
    let tools = crate::DocumentServiceSchema::mcp_tools();
    let rename = tool_named(&tools, "rename-document");
    assert_eq!(
        rename["inputSchema"],
        json!({
            "additionalProperties": false,
            "properties": { "documentId": { "type": "string" }, "title": { "type": "string" } },
            "required": ["documentId", "title"],
            "type": "object"
        })
    );
    assert!(rename.get("outputSchema").is_none());
}

#[test]
fn a_one_way_operation_lists_its_generated_message_and_no_output_schema() {
    let tools = crate::DocumentServiceSchema::mcp_tools();
    let touch = tool_named(&tools, "touch-document");
    assert_eq!(
        touch["inputSchema"],
        json!({
            "additionalProperties": false,
            "properties": { "documentId": { "type": "string" }, "reader": { "type": "string" } },
            "required": ["documentId", "reader"],
            "type": "object"
        })
    );
    assert!(touch.get("outputSchema").is_none());
}

#[test]
fn a_message_serde_writes_as_a_string_is_wrapped_under_value() {
    let tools = crate::DocumentServiceSchema::mcp_tools();
    let lookup = tool_named(&tools, "lookup-document");
    assert_eq!(
        lookup["inputSchema"],
        json!({
            "additionalProperties": false,
            "properties": { "value": { "type": "string" } },
            "required": ["value"],
            "type": "object"
        })
    );
    assert!(crate::DocumentServiceSchema::wrapped_lookup());
    assert!(!crate::DocumentServiceSchema::wrapped_get_version());
}

#[test]
fn a_header_in_is_a_property_beside_the_wrapped_message_and_a_header_out_beside_the_success() {
    let tools = crate::DocumentServiceSchema::mcp_tools();
    let tagged = tool_named(&tools, "get-tagged");
    assert_eq!(
        tagged["inputSchema"],
        json!({
            "additionalProperties": false,
            "properties": {
                "if-none-match": { "anyOf": [{ "type": "string" }, { "type": "null" }] },
                "value": { "type": "string" }
            },
            "required": ["value"],
            "type": "object"
        })
    );
    assert_eq!(
        tagged["outputSchema"],
        json!({
            "additionalProperties": false,
            "properties": {
                "etag": { "type": "string" },
                "value": {
                    "additionalProperties": false,
                    "properties": { "version": {
                        "description": "The version the document is at.",
                        "type": "integer"
                    } },
                    "required": ["version"],
                    "type": "object"
                }
            },
            "required": ["value", "etag"],
            "type": "object"
        })
    );
    assert!(crate::DocumentServiceSchema::wrapped_get_tagged());
}

#[test]
fn the_names_list_holds_every_tool() {
    assert_eq!(
        crate::DocumentServiceSchema::MCP_NAMES,
        [
            ("tool", "get-tagged"),
            ("tool", "get-version"),
            ("tool", "lookup-document"),
            ("tool", "rename-document"),
            ("tool", "touch-document")
        ]
    );
}

#[test]
fn the_trait_s_rustdoc_is_the_instructions() {
    assert_eq!(
        crate::DocumentServiceSchema::MCP_INSTRUCTIONS,
        Some("Reads and renames documents in the caller's workspace.")
    );
    assert_eq!(
        crate::TwoTransportServiceSchema::MCP_INSTRUCTIONS,
        Some("A service that asks for two transports.")
    );
}

#[test]
fn annotations_an_icon_and_a_marked_argument_are_listed() {
    let tools = crate::AnnotatedServiceSchema::mcp_tools();
    let rename = tool_named(&tools, "retitle-document");
    assert_eq!(
        rename["annotations"],
        json!({
            "readOnlyHint": true,
            "destructiveHint": false,
            "idempotentHint": true,
            "openWorldHint": false
        })
    );
    assert_eq!(
        rename["icons"],
        json!([{ "src": "https://example.com/rename.png" }])
    );
    assert_eq!(
        rename["inputSchema"]["properties"]["documentId"],
        json!({ "type": "string", "x-mcp-header": "Document-Id" })
    );
    assert!(
        rename["inputSchema"]["properties"]["title"]
            .get("x-mcp-header")
            .is_none()
    );
    let version = tool_named(&tools, "get-version");
    assert_eq!(
        version["annotations"],
        json!({ "destructiveHint": true, "openWorldHint": true })
    );
    assert!(version.get("icons").is_none());
}

#[test]
fn a_tool_without_a_flag_lists_no_annotations() {
    for tool in crate::DocumentServiceSchema::mcp_tools() {
        assert!(tool.get("annotations").is_none(), "{tool:#}");
        assert!(tool.get("icons").is_none(), "{tool:#}");
    }
}

#[test]
fn a_service_asking_for_mcp_and_ws_rpc_lists_its_tools() {
    assert_eq!(crate::TwoTransportServiceSchema::mcp_tools().len(), 1);
    assert_eq!(
        crate::TwoTransportServiceSchema::MCP_NAMES,
        [("tool", "get-version")]
    );
}

#[test]
fn a_generated_message_lists_jsonschema() {
    let schema = RenameDocumentRequest::json_schema();
    assert_eq!(schema["type"], "object");
    let fault = document_service_schema::ServiceFault::json_schema();
    assert!(fault.is_object());
    assert!(annotated_service_schema::ServiceFault::json_schema().is_object());
    assert!(two_transport_service_schema::ServiceFault::json_schema().is_object());
}

/// What `answering` answers on its first poll, which is every answer `Archive` gives.
fn answered<Answering>(answering: Answering) -> Answering::Output
where
    Answering: Future,
{
    let mut pinned = pin!(answering);
    let reply = match pinned
        .as_mut()
        .poll(&mut Context::from_waker(Waker::noop()))
    {
        Poll::Ready(output) => Some(output),
        Poll::Pending => None,
    };
    reply.unwrap()
}

#[test]
fn the_stand_in_serves_every_operation_of_every_service() {
    let ctx = ();
    let request = || GetVersionRequest {
        document_id: "doc-1".to_owned(),
    };
    answered(DocumentService::get_version(&Archive, &ctx, request())).unwrap_err();
    answered(DocumentService::rename_document(
        &Archive,
        &ctx,
        "doc-1".to_owned(),
        "Title".to_owned(),
    ))
    .unwrap_err();
    answered(DocumentService::lookup(
        &Archive,
        &ctx,
        DocumentId("doc-1".to_owned()),
    ))
    .unwrap_err();
    answered(DocumentService::touch_document(
        &Archive,
        &ctx,
        "doc-1".to_owned(),
        "reader".to_owned(),
    ));
    answered(DocumentService::get_tagged(
        &Archive,
        &ctx,
        "doc-1".to_owned(),
        None,
    ))
    .unwrap_err();
    answered(AnnotatedService::retitle_document(
        &Archive,
        &ctx,
        "doc-1".to_owned(),
        "Title".to_owned(),
    ))
    .unwrap();
    answered(AnnotatedService::get_version(&Archive, &ctx, request())).unwrap_err();
    answered(TwoTransportService::get_version(&Archive, &ctx, request())).unwrap_err();
}

#[test]
fn a_value_each_message_writes_passes_the_schema_the_tool_lists() {
    let tools = crate::DocumentServiceSchema::mcp_tools();
    let message = serde_json::to_value(GetVersionRequest {
        document_id: "doc-1".to_owned(),
    })
    .unwrap();
    let answer = serde_json::to_value(VersionResponse { version: 7 }).unwrap();
    let identifier = serde_json::to_value(DocumentId("doc-1".to_owned())).unwrap();
    let error = serde_json::to_value(VersionError::NotFound).unwrap();
    assert_eq!(error, json!({ "errorCode": "not-found" }));
    for (tool, instance, field) in [
        ("get-version", &message, "inputSchema"),
        ("get-version", &answer, "outputSchema"),
        (
            "lookup-document",
            &json!({ "value": identifier }),
            "inputSchema",
        ),
        ("lookup-document", &answer, "outputSchema"),
    ] {
        let schema = &tool_named(&tools, tool)[field];
        let validator = jsonschema::validator_for(schema).unwrap();
        let errors: Vec<String> = validator
            .iter_errors(instance)
            .map(|e| e.to_string())
            .collect();
        assert!(errors.is_empty(), "{tool} {field}: {errors:?}");
    }
}
