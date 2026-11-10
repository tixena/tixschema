//! A handler that asks the user with a form or a link, and a tool that answers content blocks:
//! every message passes the official schema, and the retried call carries the answer.

mod over_http {
    use super::{Approvals, call_with, form};
    use crate::ApprovalServiceSchema;
    use crate::approval_mcp::{
        self, McpCall, McpDenied, McpEndpoint, McpHttpRequest, McpHttpSettings, McpOAuth,
    };
    use crate::http_tests::post;
    use crate::support::{Lines, assert_result, block_on, through};
    use serde_json::{Value, json};

    const ENDPOINT: McpEndpoint = McpEndpoint::new(
        &[ApprovalServiceSchema::MCP_NAMES],
        &[approval_mcp::catalog],
    );

    const SETTINGS: McpHttpSettings<'static> = McpHttpSettings::new(
        &[],
        Some(McpOAuth::new(
            &["https://auth.example.com"],
            "https://mcp.example.com/mcp",
            "https://mcp.example.com/.well-known/oauth-protected-resource/mcp",
            &[],
        )),
    );

    fn asked(message: &Value) -> McpHttpRequest {
        let from = post(message);
        McpHttpRequest::new(
            from.method().to_owned(),
            from.path().to_owned(),
            from.headers().to_vec(),
            from.body().to_vec(),
        )
    }

    fn message() -> Value {
        call_with(
            &form(),
            "document-preview",
            &json!({ "documentId": "doc-1" }),
            None,
        )
    }

    #[test]
    fn the_service_answers_over_http_and_over_a_stdio_loop() {
        let approvals = Approvals::default();
        for denied in [
            McpDenied::NoValidCredentials { scope: None },
            McpDenied::InsufficientScope {
                scope: "documents:read".to_owned(),
            },
            McpDenied::Forbidden,
        ] {
            let response = through(|stream| {
                block_on(approval_mcp::serve_http(
                    &asked(&message()),
                    &SETTINGS,
                    stream,
                    &approval_mcp::McpServer::new(&ENDPOINT, None, "Approvals"),
                    async |request: &McpHttpRequest, _call: McpCall| {
                        assert_eq!(request.mcp_name().as_deref(), Some("document-preview"));
                        assert_eq!(request.bearer_token(), None);
                        assert_eq!(request.header("content-type"), Some("application/json"));
                        Err::<approval_mcp::McpAdmitted<()>, _>(denied.clone())
                    },
                    &|ctx: &(), part, method: &str, params: &Value| {
                        let held = approvals.clone();
                        Box::pin(async move {
                            match part {
                                0 => approval_mcp::route(method, params, &held, ctx).await,
                                _ => None,
                            }
                        })
                    },
                ))
            });
            assert!(matches!(response.status(), 401 | 403), "{denied:?}");
        }
        let response = through(|stream| {
            block_on(approval_mcp::answer_http(
                &asked(&message()),
                &SETTINGS,
                stream,
                &approvals,
                async |_request, _call| Ok(approval_mcp::McpAdmitted::new(())),
            ))
        });
        assert_eq!(response.status(), 200);
        let sent: Value = serde_json::from_slice(response.body()).unwrap();
        assert_result("CallToolResult", &sent);

        let text = block_on(approval_mcp::serve(
            &message().to_string(),
            "Approvals",
            None,
            &ENDPOINT,
            |_call| (),
            approval_mcp::McpHost::new(&approval_mcp::McpInFlight::default(), &Lines::default()),
            &|ctx: &(), part, method: &str, params: &Value| {
                let held = approvals.clone();
                Box::pin(async move {
                    match part {
                        0 => approval_mcp::route(method, params, &held, ctx).await,
                        _ => None,
                    }
                })
            },
        ))
        .unwrap();
        assert_result("CallToolResult", &serde_json::from_str(&text).unwrap());
    }
}

mod with_progress {
    use super::{Approvals, call_with, form};
    use crate::ApprovalServiceSchema;
    use crate::approval_mcp::{self, McpEndpoint, McpHost, McpInFlight};
    use crate::support::{Lines, assert_result, assert_valid, block_on};
    use serde_json::{Value, json};

    const ENDPOINT: McpEndpoint = McpEndpoint::new(
        &[ApprovalServiceSchema::MCP_NAMES],
        &[approval_mcp::catalog],
    );

    fn delete_with_a_token() -> String {
        let mut asked = call_with(
            &form(),
            "delete-document",
            &json!({ "documentId": "doc-1" }),
            None,
        );
        asked["params"]["_meta"]["progressToken"] = json!("delete-1");
        asked.to_string()
    }

    fn serve(asked: &str, lines: &Lines) -> Option<String> {
        block_on(approval_mcp::serve(
            asked,
            "Approvals",
            None,
            &ENDPOINT,
            |call| call.progress(1.0_f64, None, Some("asking")),
            McpHost::new(&McpInFlight::default(), lines),
            &|_ctx: &(), part, method: &str, params: &Value| {
                Box::pin(async move {
                    match part {
                        0 => approval_mcp::route(method, params, &Approvals::default(), &()).await,
                        _ => None,
                    }
                })
            },
        ))
    }

    #[test]
    fn an_interview_called_with_a_token_writes_progress_and_still_asks() {
        let lines = Lines::default();
        let text = serve(&delete_with_a_token(), &lines).unwrap();
        let answered: Value = serde_json::from_str(&text).unwrap();
        assert_result("InputRequiredResult", &answered);
        assert_eq!(answered["result"]["resultType"], "input_required");
        assert!(answered["result"]["inputRequests"]["confirmation"].is_object());
        let written = lines.written();
        assert_eq!(written.len(), 1);
        assert_valid(
            "ProgressNotification",
            &serde_json::from_str(&written[0]).unwrap(),
        );
    }

    #[test]
    fn an_interview_whose_client_is_gone_writes_no_answer() {
        let lines = Lines::gone();
        assert_eq!(serve(&delete_with_a_token(), &lines), None);
        assert_eq!(lines.written(), Vec::<String>::new());
    }
}

use crate::approval::{
    Approvals, DeleteConfirmation, DeleteError, ImportError, ImportedDocument, PreviewError,
    ReportRequest,
};
use crate::approval_client::{ApprovalServiceClient, McpMessage, Transport};
use crate::approval_mcp;
use crate::approval_service_schema::{CallError, McpContent, McpResourceContents};
use crate::support::{Lines, assert_result, assert_valid, block_on, id, meta, server_info};
use core::future::Future;
use serde_json::{Value, json};

use crate::ApprovalServiceSchema;
/// Hands each message to the dispatcher of the same service.
struct Dispatching<'approvals> {
    approvals: &'approvals Approvals,
}

impl Transport for Dispatching<'_> {
    fn send(&self, message: McpMessage) -> impl Future<Output = Result<String, String>> + Send {
        assert_valid(
            "CallToolRequest",
            &serde_json::from_str(message.text()).unwrap(),
        );
        let sent = (message.method(), message.name(), message.protocol_version());
        assert_eq!(sent.0, "tools/call");
        assert!(sent.1.is_some());
        assert_eq!(sent.2, "2026-07-28");
        let approvals = self.approvals;
        async move {
            Ok(approval_mcp::answer(
                message.text(),
                approvals,
                |_call| (),
                &Lines::default(),
                &approval_mcp::McpInFlight::default(),
            )
            .await
            .unwrap())
        }
    }
}

fn capable_of(elicitation: &Value) -> Value {
    let mut carried = meta();
    carried["io.modelcontextprotocol/clientCapabilities"] = json!({ "elicitation": elicitation });
    carried
}

fn call_with(
    capabilities: &Value,
    tool: &str,
    arguments: &Value,
    responses: Option<Value>,
) -> Value {
    let mut params = json!({ "name": tool, "arguments": arguments, "_meta": capabilities });
    if let Some(held) = responses {
        params["inputResponses"] = held;
    }
    let asked = json!({ "jsonrpc": "2.0", "id": id(1), "method": "tools/call", "params": params });
    assert_valid("CallToolRequest", &asked);
    asked
}

fn answer(approvals: &Approvals, asked: &Value) -> Value {
    block_on(approval_mcp::answer(
        &asked.to_string(),
        approvals,
        |_call| (),
        &Lines::default(),
        &approval_mcp::McpInFlight::default(),
    ))
    .map(|text| serde_json::from_str(&text).unwrap())
    .unwrap()
}

fn form() -> Value {
    capable_of(&json!({ "form": {} }))
}

fn link() -> Value {
    capable_of(&json!({ "form": {}, "url": {} }))
}

fn delete(capabilities: &Value, responses: Option<Value>) -> Value {
    answer(
        &Approvals::default(),
        &call_with(
            capabilities,
            "delete-document",
            &json!({ "documentId": "doc-1" }),
            responses,
        ),
    )
}

fn error_text(answered: &Value) -> Value {
    assert_result("CallToolResult", answered);
    assert_eq!(answered["result"]["isError"], true);
    serde_json::from_str(answered["result"]["content"][0]["text"].as_str().unwrap()).unwrap()
}

#[test]
fn the_answers_argument_is_left_out_of_the_tool_s_input() {
    let tools = ApprovalServiceSchema::mcp_tools();
    for (name, left_out) in [
        ("delete-document", "confirmation"),
        ("ship-document", "shipping"),
    ] {
        let tool = tools.iter().find(|held| held["name"] == name).unwrap();
        assert_eq!(
            tool["inputSchema"],
            json!({
                "additionalProperties": false,
                "properties": { "documentId": { "type": "string" } },
                "required": ["documentId"],
                "type": "object"
            }),
            "{name} left {left_out} out"
        );
    }
}

#[test]
fn a_handler_that_asks_makes_the_call_answer_a_form() {
    let answered = delete(&form(), None);
    assert_eq!(
        answered,
        json!({
            "jsonrpc": "2.0",
            "id": 1_u64,
            "result": {
                "resultType": "input_required",
                "inputRequests": { "confirmation": { "method": "elicitation/create", "params": {
                    "mode": "form",
                    "message": "Delete this document for good?",
                    "requestedSchema": {
                        "additionalProperties": false,
                        "properties": { "confirm": { "type": "boolean" } },
                        "required": ["confirm"],
                        "type": "object"
                    },
                } } },
                "_meta": server_info("ApprovalService"),
            },
        })
    );
    assert_result("InputRequiredResult", &answered);
}

#[test]
fn a_form_without_a_message_of_its_own_asks_with_the_rustdoc() {
    let answered = answer(
        &Approvals::default(),
        &call_with(
            &form(),
            "ship-document",
            &json!({ "documentId": "doc-1" }),
            None,
        ),
    );
    assert_result("InputRequiredResult", &answered);
    let asked = &answered["result"]["inputRequests"]["shipping"]["params"];
    assert_eq!(asked["message"], "How should it ship?");
    assert_eq!(asked["mode"], "form");
}

#[test]
fn the_retried_call_runs_the_handler_with_the_answer() {
    let approvals = Approvals::default();
    let asked = call_with(
        &form(),
        "delete-document",
        &json!({ "documentId": "doc-1" }),
        Some(json!({ "confirmation": { "action": "accept", "content": { "confirm": true } } })),
    );
    let answered = answer(&approvals, &asked);
    assert_result("CallToolResult", &answered);
    assert_eq!(answered["result"]["isError"], false);
    assert_eq!(
        *approvals.reached.lock().unwrap(),
        ["delete-document doc-1 Some(DeleteConfirmation { confirm: true })"]
    );
}

#[test]
fn a_declined_or_cancelled_form_runs_the_handler_without_an_answer_and_answers_the_arm() {
    for action in ["decline", "cancel"] {
        let approvals = Approvals::default();
        let asked = call_with(
            &form(),
            "delete-document",
            &json!({ "documentId": "doc-1" }),
            Some(json!({ "confirmation": { "action": action } })),
        );
        let answered = answer(&approvals, &asked);
        assert_eq!(
            error_text(&answered),
            json!({ "errorCode": "needs-confirmation", "message": "Delete this document for good?" }),
            "{action}"
        );
        assert_eq!(
            *approvals.reached.lock().unwrap(),
            ["delete-document doc-1 None"]
        );
    }
}

#[test]
fn a_client_that_declared_no_elicitation_receives_the_arm_as_the_declared_error() {
    for capabilities in [meta(), capable_of(&json!({ "url": {} }))] {
        let answered = delete(&capabilities, None);
        assert_eq!(
            error_text(&answered),
            json!({ "errorCode": "needs-confirmation", "message": "Delete this document for good?" })
        );
    }
}

#[test]
fn an_answer_that_does_not_read_answers_the_validation_fault() {
    let asked = call_with(
        &form(),
        "delete-document",
        &json!({ "documentId": "doc-1" }),
        Some(json!({ "confirmation": { "action": "accept", "content": { "confirm": "yes" } } })),
    );
    let answered = answer(&Approvals::default(), &asked);
    let fault = error_text(&answered);
    assert_eq!(fault["isServiceFault"], true);
    assert_eq!(fault["fault"]["kind"], "failed-validation");
    assert_eq!(fault["fault"]["field"], "confirmation");
}

#[test]
fn a_call_that_carries_the_answer_among_its_arguments_reaches_the_handler_with_it() {
    let approvals = Approvals::default();
    let asked = call_with(
        &meta(),
        "delete-document",
        &json!({ "documentId": "doc-1", "confirmation": { "confirm": false } }),
        None,
    );
    let answered = answer(&approvals, &asked);
    assert_eq!(error_text(&answered), json!({ "errorCode": "refused" }));
}

#[test]
fn a_flat_form_of_every_kind_of_field_is_asked_and_answered() {
    let asked = call_with(
        &form(),
        "ship-document",
        &json!({ "documentId": "doc-1" }),
        None,
    );
    let answered = answer(&Approvals::default(), &asked);
    assert_result("InputRequiredResult", &answered);
    let schema = &answered["result"]["inputRequests"]["shipping"]["params"]["requestedSchema"];
    assert_eq!(schema["type"], "object");
    assert_eq!(
        schema["properties"]["express"],
        json!({ "type": "boolean" })
    );

    let approvals = Approvals::default();
    let retried = call_with(
        &form(),
        "ship-document",
        &json!({ "documentId": "doc-1" }),
        Some(json!({ "shipping": { "action": "accept", "content": {
            "express": true, "parcels": 2_u8, "priority": "High", "tags": ["Low", "High"]
        } } })),
    );
    let done = answer(&approvals, &retried);
    assert_result("CallToolResult", &done);
    assert_eq!(done["result"]["isError"], false);
}

#[test]
fn a_link_is_asked_in_url_mode_keyed_by_the_arm_s_wire_tag() {
    let answered = answer(
        &Approvals::default(),
        &call_with(
            &link(),
            "import-from-drive",
            &json!({ "value": "1A2b3C" }),
            None,
        ),
    );
    assert_eq!(
        answered,
        json!({
            "jsonrpc": "2.0",
            "id": 1_u64,
            "result": {
                "resultType": "input_required",
                "inputRequests": { "needs-drive-connection": { "method": "elicitation/create", "params": {
                    "mode": "url",
                    "url": "https://mcp.example.com/connect/drive?request=7f3a",
                    "message": "Connect your Google Drive so this server can import the file.",
                } } },
                "_meta": server_info("ApprovalService"),
            },
        })
    );
    assert_result("InputRequiredResult", &answered);
}

#[test]
fn the_retried_link_call_runs_the_handler_again_and_asks_again_until_it_is_done() {
    let approvals = Approvals::default();
    let accepted = json!({ "needs-drive-connection": { "action": "accept" } });
    let again = answer(
        &approvals,
        &call_with(
            &link(),
            "import-from-drive",
            &json!({ "value": "1A2b3C" }),
            Some(accepted.clone()),
        ),
    );
    assert_result("InputRequiredResult", &again);

    approvals.connect();
    let done = answer(
        &approvals,
        &call_with(
            &link(),
            "import-from-drive",
            &json!({ "value": "1A2b3C" }),
            Some(accepted),
        ),
    );
    assert_result("CallToolResult", &done);
    assert_eq!(
        done["result"]["structuredContent"],
        json!({ "document_id": "doc-1A2b3C" })
    );
    assert_eq!(approvals.reached.lock().unwrap().len(), 2);
}

#[test]
fn a_declined_link_answers_the_arm_as_a_tool_execution_error() {
    let answered = answer(
        &Approvals::default(),
        &call_with(
            &link(),
            "import-from-drive",
            &json!({ "value": "1A2b3C" }),
            Some(json!({ "needs-drive-connection": { "action": "decline" } })),
        ),
    );
    assert_eq!(error_text(&answered)["errorCode"], "needs-drive-connection");
}

#[test]
fn a_client_without_the_url_mode_receives_the_link_inside_the_declared_error() {
    let answered = answer(
        &Approvals::default(),
        &call_with(
            &form(),
            "import-from-drive",
            &json!({ "value": "1A2b3C" }),
            None,
        ),
    );
    assert_eq!(
        error_text(&answered),
        json!({
            "errorCode": "needs-drive-connection",
            "url": "https://mcp.example.com/connect/drive?request=7f3a",
            "message": "Connect your Google Drive so this server can import the file.",
        })
    );
}

fn preview(document_id: &str) -> Value {
    let answered = answer(
        &Approvals::default(),
        &call_with(
            &meta(),
            "document-preview",
            &json!({ "value": document_id }),
            None,
        ),
    );
    assert_result("CallToolResult", &answered);
    answered
}

#[test]
fn an_image_or_audio_pair_answers_one_block_and_no_structured_content() {
    let image = preview("doc-1");
    assert_eq!(
        image["result"],
        json!({
            "resultType": "complete",
            "isError": false,
            "content": [{ "type": "image", "data": "iVBORw==", "mimeType": "image/png" }],
            "_meta": server_info("ApprovalService"),
        })
    );
    let audio = preview("voice");
    assert_eq!(
        audio["result"]["content"],
        json!([{ "type": "audio", "data": "AQIDBAU=", "mimeType": "audio/mpeg" }])
    );
    assert!(audio["result"].get("structuredContent").is_none());
}

#[test]
fn a_pair_of_any_other_content_type_answers_as_an_operation_with_no_flag_does() {
    let plain = preview("plain");
    assert_eq!(
        plain["result"]["structuredContent"],
        json!([[37_u8, 80_u8, 68_u8, 70_u8, 45_u8], "application/pdf"])
    );
}

#[test]
fn a_list_of_blocks_answers_its_blocks_in_order() {
    let answered = answer(
        &Approvals::default(),
        &call_with(&meta(), "build-report", &json!({ "title": "Q3" }), None),
    );
    assert_result("CallToolResult", &answered);
    assert_eq!(
        answered["result"]["content"],
        json!([
            { "type": "text", "text": "Q3" },
            { "type": "image", "data": "iVBORw==", "mimeType": "image/png" },
            { "type": "audio", "data": "AQIDBAU=", "mimeType": "audio/mpeg" },
            { "type": "resource_link", "uri": "workspace://documents/doc-1", "name": "id-card.pdf",
              "title": "Id card", "mimeType": "application/pdf" },
            { "type": "resource", "resource": { "uri": "workspace://documents/doc-2",
              "mimeType": "text/plain", "text": "passport" } },
            { "type": "resource", "resource": { "uri": "workspace://documents/doc-3", "blob": "CQgH" } },
        ])
    );
    assert!(answered["result"].get("structuredContent").is_none());
}

#[test]
fn a_content_tool_declares_no_output_schema() {
    let tools = ApprovalServiceSchema::mcp_tools();
    for name in ["document-preview", "build-report"] {
        let tool = tools.iter().find(|held| held["name"] == name).unwrap();
        assert!(tool.get("outputSchema").is_none(), "{name}");
    }
}

#[test]
fn a_client_reads_the_blocks_back_into_the_pair_and_the_list() {
    let approvals = Approvals::default();
    let client = ApprovalServiceClient::new(Dispatching {
        approvals: &approvals,
    });
    assert_eq!(
        block_on(client.document_preview("doc-1".to_owned())).unwrap(),
        (vec![0x89, b'P', b'N', b'G'], "image/png".to_owned())
    );
    assert_eq!(
        block_on(client.document_preview("plain".to_owned())).unwrap(),
        (b"%PDF-".to_vec(), "application/pdf".to_owned())
    );
    assert!(matches!(
        block_on(client.document_preview("nothing".to_owned())),
        Err(CallError::Operation(PreviewError::NotFound))
    ));
    let blocks = block_on(client.build_report(ReportRequest {
        title: "Q3".to_owned(),
    }))
    .unwrap();
    assert_eq!(blocks.len(), 6);
    assert_eq!(
        blocks[0],
        McpContent::Text {
            text: "Q3".to_owned()
        }
    );
    assert_eq!(
        blocks[5],
        McpContent::Resource {
            uri: "workspace://documents/doc-3".to_owned(),
            mime_type: None,
            contents: McpResourceContents::Blob(vec![9, 8, 7]),
        }
    );
}

#[test]
fn a_client_declares_no_capabilities_so_it_receives_the_arm_and_calls_again_with_the_answer() {
    let approvals = Approvals::default();
    let client = ApprovalServiceClient::new(Dispatching {
        approvals: &approvals,
    });
    assert!(
        client
            .transport()
            .approvals
            .reached
            .lock()
            .unwrap()
            .is_empty()
    );
    let first = block_on(client.delete_document("doc-1".to_owned(), None));
    assert!(matches!(
        first,
        Err(CallError::Operation(DeleteError::NeedsConfirmation { message })) if !message.is_empty()
    ));
    block_on(client.delete_document(
        "doc-1".to_owned(),
        Some(DeleteConfirmation { confirm: true }),
    ))
    .unwrap();
    block_on(client.ship_document("doc-1".to_owned(), None)).unwrap_err();
    let link = block_on(client.import_from_drive("1A2b3C".to_owned()));
    assert!(matches!(
        link,
        Err(CallError::Operation(ImportError::NeedsDriveConnection { url, message })) if !url.is_empty() && !message.is_empty()
    ));
    approvals.connect();
    assert_eq!(
        block_on(client.import_from_drive("1A2b3C".to_owned())).unwrap(),
        ImportedDocument {
            document_id: "doc-1A2b3C".to_owned()
        }
    );
}
