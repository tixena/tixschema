//! Prompts: `prompts/list`, `prompts/get` and the completion of a prompt's argument, driven
//! against a real expansion. Every message passes the official 2026-07-28 schema.

mod over_http {
    use super::{Counsels, ENDPOINT, get};
    use crate::CounselServiceSchema;
    use crate::counsel_mcp::{
        self, McpCall, McpDenied, McpEndpoint, McpHttpRequest, McpHttpSettings,
    };
    use crate::http_tests::post;
    use crate::support::{Recorded, assert_result, block_on, through};
    use serde_json::{Value, json};

    const SETTINGS: McpHttpSettings<'static> = McpHttpSettings::new(&[], None);

    fn asked() -> McpHttpRequest {
        asking(&get("greet", &json!({ "name": "Ana" })))
    }

    fn asking(message: &Value) -> McpHttpRequest {
        let from = post(message);
        let mut headers = from.headers().to_vec();
        headers.push(("Mcp-Name".to_owned(), "greet".to_owned()));
        McpHttpRequest::new(
            from.method().to_owned(),
            from.path().to_owned(),
            headers,
            from.body().to_vec(),
        )
    }

    #[test]
    fn a_prompt_request_with_a_progress_token_answers_an_event_stream() {
        let mut tokened = get("greet", &json!({ "name": "Ana" }));
        tokened["params"]["_meta"]["progressToken"] = json!("greet-1");
        let stream = Recorded::default();
        let answered = block_on(counsel_mcp::answer_http(
            &asking(&tokened),
            &SETTINGS,
            &stream,
            &Counsels::default(),
            async |_request, _call| Ok(counsel_mcp::McpAdmitted::new(())),
        ));
        assert!(answered.is_none());
        let streamed = stream.streamed();
        assert_eq!(streamed.status, 200);
        assert!(
            streamed
                .headers
                .contains(&("Content-Type".to_owned(), "text/event-stream".to_owned())),
            "{:?}",
            streamed.headers
        );
        let body = String::from_utf8(streamed.body).unwrap();
        let data = body
            .strip_prefix("event: message\ndata: ")
            .unwrap()
            .trim_end();
        assert_result("GetPromptResult", &serde_json::from_str(data).unwrap());
    }

    #[test]
    fn a_denied_prompt_request_stops_before_its_handler_and_an_admitted_one_is_answered() {
        let counsels = Counsels::default();
        let endpoint: &McpEndpoint = &ENDPOINT;
        for denied in [
            McpDenied::NoValidCredentials { scope: None },
            McpDenied::InsufficientScope {
                scope: CounselServiceSchema::mcp_scopes("prompts/get", Some("greet"))[0].to_owned(),
            },
            McpDenied::Forbidden,
        ] {
            let response = through(|stream| {
                block_on(counsel_mcp::serve_http(
                    &asked(),
                    &SETTINGS,
                    stream,
                    &counsel_mcp::McpServer::new(endpoint, None, "Counsel"),
                    async |request: &McpHttpRequest, call: McpCall| {
                        assert_eq!(request.mcp_name().as_deref(), Some("greet"));
                        call.progress(1.0_f64, None, None);
                        Err::<counsel_mcp::McpAdmitted<()>, _>(denied.clone())
                    },
                    &|ctx: &(), part, method: &str, params: &Value| {
                        let held = counsels.clone();
                        Box::pin(async move {
                            match part {
                                0 => counsel_mcp::route(method, params, &held, ctx).await,
                                _ => None,
                            }
                        })
                    },
                ))
            });
            assert!(matches!(response.status(), 401 | 403), "{denied:?}");
        }
        assert!(counsels.reached.lock().unwrap().is_empty());
        let response = through(|stream| {
            block_on(counsel_mcp::answer_http(
                &asked(),
                &SETTINGS,
                stream,
                &counsels,
                async |_request, _call| Ok(counsel_mcp::McpAdmitted::new(())),
            ))
        });
        assert_eq!(response.status(), 200);
        assert_result(
            "GetPromptResult",
            &serde_json::from_slice(response.body()).unwrap(),
        );
    }
}

use crate::CounselServiceSchema;
use crate::counsel::{CounselDraft, CounselErrorDraft, Counsels};
use crate::counsel_client::{CounselServiceClient, McpMessage, Transport};
use crate::counsel_mcp::{
    self, McpEndpoint, McpHost, McpHttpRequest, McpHttpSettings, McpInFlight,
};
use crate::counsel_service_schema::{
    CallError, McpContent, McpPromptMessage, McpResourceContents, McpRole, ServiceFaultKind,
};
use crate::support::{
    HEADER_MISMATCH, INTERNAL_ERROR, INVALID_PARAMS, LISTED_FOR_MS, Lines, assert_error,
    assert_result, assert_valid, block_on, id, request, server_info, through,
};
use core::future::Future;
use serde_json::{Value, json};
use std::sync::Mutex;

const ENDPOINT: McpEndpoint =
    McpEndpoint::new(&[CounselServiceSchema::MCP_NAMES], &[counsel_mcp::catalog]);

/// Hands each message to the dispatcher of the same service, and writes down what it was handed.
struct Dispatching {
    sent: Mutex<Vec<(&'static str, Option<String>, Value)>>,
}

impl Transport for Dispatching {
    fn send(&self, message: McpMessage) -> impl Future<Output = Result<String, String>> + Send {
        let asked: Value = serde_json::from_str(message.text()).unwrap();
        assert_eq!(message.protocol_version(), "2026-07-28");
        self.sent.lock().unwrap().push((
            message.method(),
            message.name().map(str::to_owned),
            asked["params"]["arguments"].clone(),
        ));
        async move {
            Ok(block_on(counsel_mcp::answer(
                message.text(),
                &Counsels::default(),
                |_call| (),
                &Lines::default(),
                &McpInFlight::default(),
            ))
            .unwrap())
        }
    }
}

fn answer(asked: &Value) -> Value {
    block_on(counsel_mcp::answer(
        &asked.to_string(),
        &Counsels::default(),
        |_call| (),
        &Lines::default(),
        &McpInFlight::default(),
    ))
    .map(|text| serde_json::from_str(&text).unwrap())
    .unwrap()
}

fn get(name: &str, arguments: &Value) -> Value {
    let asked = request(
        &id(3),
        "prompts/get",
        json!({ "name": name, "arguments": arguments }),
    );
    assert_valid("GetPromptRequest", &asked);
    asked
}

fn fetched(name: &str, arguments: &Value) -> Value {
    answer(&get(name, arguments))
}

#[test]
fn server_discover_adds_the_prompts_capability() {
    let answered = answer(&request(&id(1), "server/discover", json!({})));
    assert_result("DiscoverResult", &answered);
    assert_eq!(
        answered["result"]["capabilities"],
        json!({ "completions": {}, "prompts": {}, "tools": {} })
    );
}

#[test]
fn prompts_list_lists_each_prompt_and_tools_list_leaves_them_out() {
    let answered = answer(&request(&id(1), "prompts/list", json!({})));
    assert_result("ListPromptsResult", &answered);
    assert_eq!(
        answered,
        json!({
            "jsonrpc": "2.0",
            "id": 1_u64,
            "result": {
                "resultType": "complete",
                "prompts": [
                    { "name": "configure-organization", "title": "Configure Organization",
                      "description": "Guides the model through the configuration of one organization." },
                    { "name": "greet", "title": "A Greeting", "description": "Greets one person.",
                      "icons": [{ "src": "https://example.com/hi.png" }],
                      "arguments": [{ "name": "name", "required": true }] },
                    { "name": "review-document-type", "title": "Review Document Type",
                      "description": "Reviews one document type against the naming guide.",
                      "arguments": [
                          { "name": "documentType", "required": true },
                          { "name": "focus", "required": false }
                      ] },
                ],
                "_meta": server_info("CounselService"),
                "ttlMs": LISTED_FOR_MS,
                "cacheScope": "public",
            },
        })
    );
    let tools = answer(&request(&id(2), "tools/list", json!({})));
    let names: Vec<&str> = tools["result"]["tools"]
        .as_array()
        .unwrap()
        .iter()
        .map(|tool| tool["name"].as_str().unwrap())
        .collect();
    assert_eq!(names, ["get-counsel"]);
}

#[test]
fn a_string_success_answers_one_user_message() {
    let answered = fetched("configure-organization", &json!({}));
    assert_result("GetPromptResult", &answered);
    assert_eq!(
        answered["result"],
        json!({
            "resultType": "complete",
            "description": "Guides the model through the configuration of one organization.",
            "messages": [{ "role": "user", "content": {
                "type": "text", "text": "Apply the entity types in this order." } }],
            "_meta": server_info("CounselService"),
        })
    );
}

#[test]
fn a_request_with_no_arguments_reads_as_an_empty_object() {
    let asked = request(
        &id(3),
        "prompts/get",
        json!({ "name": "configure-organization" }),
    );
    assert_valid("GetPromptRequest", &asked);
    assert_result("GetPromptResult", &answer(&asked));
}

#[test]
fn a_lone_string_argument_is_read_by_its_camel_case_name() {
    let answered = fetched("greet", &json!({ "name": "Ana" }));
    assert_result("GetPromptResult", &answered);
    assert_eq!(
        answered["result"]["messages"][0]["content"]["text"],
        "Say hello to Ana."
    );
}

#[test]
fn a_list_of_messages_answers_each_in_order() {
    let answered = fetched(
        "review-document-type",
        &json!({ "documentType": "ID card" }),
    );
    assert_result("GetPromptResult", &answered);
    assert_eq!(
        answered["result"]["messages"],
        json!([
            { "role": "user", "content": { "type": "text",
                "text": "Review the document type ID card against this guide." } },
            { "role": "user", "content": { "type": "resource", "resource": {
                "uri": "guide://naming", "mimeType": "text/markdown", "text": "# Naming" } } },
            { "role": "assistant", "content": { "type": "image",
                "data": "iVBORw==", "mimeType": "image/png" } },
        ])
    );
}

#[test]
fn an_optional_argument_reaches_the_handler_when_it_is_given() {
    let counsels = Counsels::default();
    let asked = get(
        "review-document-type",
        &json!({ "documentType": "ID card", "focus": "names" }),
    );
    let text = block_on(counsel_mcp::answer(
        &asked.to_string(),
        &counsels,
        |_call| (),
        &Lines::default(),
        &McpInFlight::default(),
    ))
    .unwrap();
    assert_result("GetPromptResult", &serde_json::from_str(&text).unwrap());
    assert_eq!(
        counsels.reached.lock().unwrap().clone(),
        ["review-document-type ID card Some(\"names\")"]
    );
}

fn refused(answered: &Value, code: i64, kind: &str) {
    assert_error("JSONRPCErrorResponse", answered);
    assert_eq!(answered["error"]["code"], code, "{answered:#}");
    assert_eq!(answered["error"]["data"]["isServiceFault"], true);
    assert_eq!(answered["error"]["data"]["fault"]["kind"], kind);
}

#[test]
fn every_failure_is_a_json_rpc_error() {
    refused(
        &fetched("delete-everything", &json!({})),
        INVALID_PARAMS,
        "unknown-operation",
    );
    refused(
        &fetched("review-document-type", &json!({})),
        INVALID_PARAMS,
        "failed-validation",
    );
    let mut numbered = get("review-document-type", &json!({ "documentType": "x" }));
    numbered["params"]["arguments"]["documentType"] = json!(5_u8);
    refused(&answer(&numbered), INVALID_PARAMS, "failed-validation");
    refused(
        &fetched("greet", &json!({})),
        INVALID_PARAMS,
        "failed-validation",
    );
    refused(
        &fetched("greet", &json!({ "name": "boom" })),
        INTERNAL_ERROR,
        "handler-panic",
    );
    let declined = fetched("greet", &json!({ "name": "missing" }));
    assert_error("JSONRPCErrorResponse", &declined);
    assert_eq!(declined["error"]["code"], INVALID_PARAMS);
    assert_eq!(
        declined["error"]["data"],
        json!({ "errorCode": "not-found" })
    );
    let absent = answer(&request(&id(3), "prompts/get", json!({})));
    assert_eq!(absent["error"]["code"], INVALID_PARAMS);
}

#[test]
fn a_prompt_argument_is_completed_by_its_camel_case_name() {
    let asked = request(
        &id(4),
        "completion/complete",
        json!({
            "ref": { "type": "ref/prompt", "name": "review-document-type" },
            "argument": { "name": "documentType", "value": "i" },
            "context": { "arguments": { "focus": "names" } },
        }),
    );
    assert_valid("CompleteRequest", &asked);
    let answered = answer(&asked);
    assert_result("CompleteResult", &answered);
    assert_eq!(
        answered["result"]["completion"],
        json!({ "values": ["ID card", "Invoice"], "hasMore": false })
    );
    let other = request(
        &id(4),
        "completion/complete",
        json!({
            "ref": { "type": "ref/prompt", "name": "review-document-type" },
            "argument": { "name": "focus", "value": "i" },
        }),
    );
    assert_eq!(answer(&other)["error"]["code"], INVALID_PARAMS);
}

#[test]
fn each_prompt_the_schema_lists_passes_the_prompt_definition() {
    let prompts = CounselServiceSchema::mcp_prompts();
    assert_eq!(prompts.len(), 3);
    for prompt in &prompts {
        assert_valid("Prompt", prompt);
    }
}

#[test]
fn the_names_of_the_service_hold_each_prompt() {
    assert_eq!(
        CounselServiceSchema::MCP_NAMES,
        &[
            ("prompt", "configure-organization"),
            ("tool", "get-counsel"),
            ("prompt", "greet"),
            ("prompt", "review-document-type"),
        ]
    );
}

#[test]
fn the_scopes_a_prompt_and_its_completion_declare_are_answered() {
    assert_eq!(
        CounselServiceSchema::mcp_scopes("prompts/get", Some("greet")),
        ["prompts:read"]
    );
    assert_eq!(
        CounselServiceSchema::mcp_scopes("prompts/get", Some("review-document-type")),
        &[] as &[&str]
    );
    assert_eq!(
        CounselServiceSchema::mcp_scopes("completion/complete", Some("review-document-type")),
        ["prompts:read"]
    );
}

#[test]
fn the_mcp_name_header_of_prompts_get_must_match_the_body() {
    let asked = get("greet", &json!({ "name": "Ana" }));
    let headers = |name: &str| {
        vec![
            ("Content-Type".to_owned(), "application/json".to_owned()),
            ("MCP-Protocol-Version".to_owned(), "2026-07-28".to_owned()),
            ("Mcp-Method".to_owned(), "prompts/get".to_owned()),
            ("Mcp-Name".to_owned(), name.to_owned()),
        ]
    };
    let post = |name: &str| {
        McpHttpRequest::new(
            "POST".to_owned(),
            "/mcp".to_owned(),
            headers(name),
            asked.to_string().into_bytes(),
        )
    };
    let settings = McpHttpSettings::new(&[], None);
    let served = |name: &str| {
        through(|stream| {
            block_on(counsel_mcp::answer_http(
                &post(name),
                &settings,
                stream,
                &Counsels::default(),
                async |_request, _call| Ok(counsel_mcp::McpAdmitted::new(())),
            ))
        })
    };
    let mismatched = served("another");
    assert_eq!(mismatched.status(), 400);
    let body: Value = serde_json::from_slice(mismatched.body()).unwrap();
    assert_eq!(body["error"]["code"], HEADER_MISMATCH);
    let matched = served("greet");
    assert_eq!(matched.status(), 200);
    assert_result(
        "GetPromptResult",
        &serde_json::from_slice(matched.body()).unwrap(),
    );
}

#[test]
fn an_endpoint_lists_and_routes_prompts() {
    let served = |asked: &Value| {
        block_on(counsel_mcp::serve(
            &asked.to_string(),
            "Counsel",
            None,
            &ENDPOINT,
            |_call| (),
            McpHost::new(&McpInFlight::default(), &Lines::default()),
            &|_ctx: &(), part, method: &str, params: &Value| {
                Box::pin(async move {
                    match part {
                        0 => counsel_mcp::route(method, params, &Counsels::default(), &()).await,
                        _ => None,
                    }
                })
            },
        ))
        .map(|text| serde_json::from_str::<Value>(&text).unwrap())
        .unwrap()
    };
    let listed = served(&request(&id(1), "prompts/list", json!({})));
    assert_eq!(listed["result"]["prompts"].as_array().unwrap().len(), 3);
    let got = served(&get("greet", &json!({ "name": "Ana" })));
    assert_result("GetPromptResult", &got);
    let unknown = served(&get("nothing", &json!({})));
    assert_eq!(unknown["error"]["message"], "Unknown prompt: nothing");
}

#[test]
fn the_client_has_one_method_per_prompt() {
    let seam = Dispatching {
        sent: Mutex::new(Vec::new()),
    };
    let client = CounselServiceClient::new(seam);
    assert_eq!(
        block_on(client.get_counsel()).unwrap(),
        CounselDraft { version: 3 }.validate().unwrap()
    );
    let text = block_on(client.configure_organization()).unwrap();
    assert_eq!(text, "Apply the entity types in this order.");
    let greeting = block_on(client.greet("Ana".to_owned())).unwrap();
    assert_eq!(greeting, "Say hello to Ana.");
    let messages = block_on(client.review_document_type("ID card".to_owned(), None)).unwrap();
    assert_eq!(messages.len(), 3);
    assert_eq!(messages[0].role, McpRole::User);
    assert_eq!(
        messages[1],
        McpPromptMessage {
            role: McpRole::User,
            content: McpContent::Resource {
                uri: "guide://naming".to_owned(),
                mime_type: Some("text/markdown".to_owned()),
                contents: McpResourceContents::Text("# Naming".to_owned()),
            },
        }
    );
    assert_eq!(messages[2].role, McpRole::Assistant);
    let sent = client.transport().sent.lock().unwrap();
    assert_eq!(
        sent.iter()
            .map(|(method, name, arguments)| (*method, name.clone(), arguments.clone()))
            .collect::<Vec<_>>(),
        vec![
            ("tools/call", Some("get-counsel".to_owned()), json!({})),
            (
                "prompts/get",
                Some("configure-organization".to_owned()),
                json!({})
            ),
            (
                "prompts/get",
                Some("greet".to_owned()),
                json!({ "name": "Ana" })
            ),
            (
                "prompts/get",
                Some("review-document-type".to_owned()),
                json!({ "documentType": "ID card" })
            ),
        ]
    );
}

#[test]
fn the_client_reads_a_declared_error_and_a_fault() {
    let client = CounselServiceClient::new(Dispatching {
        sent: Mutex::new(Vec::new()),
    });
    assert!(
        matches!(block_on(client.greet("missing".to_owned())), Err(CallError::Operation(error)) if matches!(&*error, CounselErrorDraft::NotFound))
    );
    assert!(matches!(
        block_on(client.greet("boom".to_owned())),
        Err(CallError::Fault(fault)) if fault.kind() == ServiceFaultKind::HandlerPanic
    ));
}
