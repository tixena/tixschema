//! The writer: progress, the event-stream answer, cancellation, and the stdio host that answers
//! each line as its own task. Nothing here waits on a clock; each wait ends on the event itself.

use crate::export_mcp::{self, McpCall, McpEndpoint, McpHttpRequest, McpHttpSettings, McpInFlight};
use crate::http_tests;
use crate::support::{Lines, Tasks, assert_result, assert_valid, block_on, call, id, request};
use crate::tests::{Archive, Reporting};
use crate::{DocumentServiceSchema, ExportServiceSchema, billing_mcp, document_mcp};
use serde_json::{Value, json};
use std::sync::Mutex;

const PROGRESS_FIRST: &str = "{\"jsonrpc\":\"2.0\",\"method\":\"notifications/progress\",\"params\":{\"message\":\"exported 1 of 3 files\",\"progress\":1.0,\"progressToken\":\"export-1\",\"total\":3.0}}";

fn reporting(call: &McpCall) -> Reporting {
    let (reporter, watcher) = (call.clone(), call.clone());
    Reporting::new(
        move |done, total, message| reporter.progress(done, total, message),
        move || watcher.cancelled(),
    )
}

fn export(request_id: u64, folder: &str, wanted: Option<&str>) -> Value {
    let mut asked = call(
        &id(request_id),
        "export-folder",
        &json!({ "value": folder }),
    );
    if let Some(progress_token) = wanted {
        asked["params"]["_meta"]["progressToken"] = json!(progress_token);
    }
    assert_valid("CallToolRequest", &asked);
    asked
}

fn answer_export(asked: &Value, lines: &Lines) -> Option<Value> {
    block_on(export_mcp::answer(
        &asked.to_string(),
        &Archive::default(),
        |call| reporting(&call),
        lines,
        &McpInFlight::default(),
    ))
    .map(|text| serde_json::from_str(&text).unwrap())
}

fn post(asked: &Value) -> McpHttpRequest {
    McpHttpRequest {
        body: asked.to_string().into_bytes(),
        headers: vec![
            ("MCP-Protocol-Version".to_owned(), "2026-07-28".to_owned()),
            ("Mcp-Method".to_owned(), "tools/call".to_owned()),
            ("Mcp-Name".to_owned(), "export-folder".to_owned()),
        ],
        method: "POST".to_owned(),
        path: "/mcp".to_owned(),
    }
}

fn respond(asked: &Value) -> export_mcp::McpHttpResponse {
    let settings = McpHttpSettings {
        allowed_origins: &[],
        oauth: None,
    };
    block_on(export_mcp::answer_http(
        &post(asked),
        &settings,
        &Archive::default(),
        async |_request: &McpHttpRequest, call: McpCall| {
            Ok::<_, export_mcp::McpDenied>(reporting(&call))
        },
    ))
}

fn events_of(response: &export_mcp::McpHttpResponse) -> Vec<Value> {
    let body = String::from_utf8(response.body.clone()).unwrap();
    body.split_terminator("\n\n")
        .map(|event| {
            let data = event.strip_prefix("event: message\ndata: ").unwrap();
            assert!(!data.contains('\n'), "{data}");
            serde_json::from_str(data).unwrap()
        })
        .collect()
}

#[test]
fn progress_is_written_ahead_of_the_answer_where_the_request_carries_a_token() {
    let lines = Lines::default();
    let answered = answer_export(&export(1, "docs", Some("export-1")), &lines).unwrap();
    assert_eq!(lines.written(), vec![PROGRESS_FIRST.to_owned()]);
    assert_valid(
        "ProgressNotification",
        &serde_json::from_str(&lines.written()[0]).unwrap(),
    );
    assert_eq!(answered["id"], 1_u64);
    assert_eq!(answered["result"]["isError"], false);
}

#[test]
fn a_resource_read_writes_its_progress_ahead_of_the_answer() {
    let mut asked = request(
        &id(1),
        "resources/read",
        json!({ "uri": "export://reports/docs" }),
    );
    asked["params"]["_meta"]["progressToken"] = json!("export-1");
    assert_valid("ReadResourceRequest", &asked);
    let lines = Lines::default();
    let answered = answer_export(&asked, &lines).unwrap();
    assert_eq!(lines.written(), vec![PROGRESS_FIRST.to_owned()]);
    assert_eq!(answered["id"], 1_u64);
    assert_result("ReadResourceResult", &answered);
    assert_eq!(answered["result"]["contents"][0]["text"], "report of docs");
}

#[test]
fn progress_does_nothing_where_the_request_carries_no_token() {
    let lines = Lines::default();
    let answered = answer_export(&export(1, "docs", None), &lines).unwrap();
    assert_eq!(lines.written(), Vec::<String>::new());
    assert_eq!(answered["id"], 1_u64);
}

#[test]
fn a_token_may_be_a_number() {
    let mut asked = export(1, "docs", None);
    asked["params"]["_meta"]["progressToken"] = json!(7_u64);
    let lines = Lines::default();
    answer_export(&asked, &lines).unwrap();
    let progress: Value = serde_json::from_str(&lines.written()[0]).unwrap();
    assert_eq!(progress["params"]["progressToken"], 7_u64);
    assert_valid("ProgressNotification", &progress);
}

#[test]
fn progress_is_written_between_two_polls_of_the_handler() {
    let lines = Lines::default();
    let answered = answer_export(&export(1, "steps", Some("export-1")), &lines).unwrap();
    let written = lines.written();
    assert_eq!(written.len(), 2);
    assert_eq!(written[0], PROGRESS_FIRST);
    let second: Value = serde_json::from_str(&written[1]).unwrap();
    assert_eq!(second["params"]["progress"], 2.0_f64);
    assert_eq!(second["params"].get("message"), None);
    assert_valid("ProgressNotification", &second);
    assert_eq!(answered["id"], 1_u64);
}

#[test]
fn a_write_that_fails_cancels_the_call_and_drops_the_answer() {
    let lines = Lines::gone();
    let seen = Mutex::new(None);
    let answered = block_on(export_mcp::answer(
        &export(1, "docs", Some("export-1")).to_string(),
        &Archive::default(),
        |call| {
            *seen.lock().unwrap() = Some(call.clone());
            reporting(&call)
        },
        &lines,
        &McpInFlight::default(),
    ));
    assert_eq!(answered, None);
    assert!(seen.lock().unwrap().as_ref().unwrap().cancelled());
    assert_eq!(lines.written(), Vec::<String>::new());
}

#[test]
fn a_request_with_a_token_answers_an_event_stream_over_http() {
    let response = respond(&export(1, "docs", Some("export-1")));
    assert_eq!(response.status, 200);
    assert_eq!(
        response.headers,
        vec![
            ("Content-Type".to_owned(), "text/event-stream".to_owned()),
            ("X-Accel-Buffering".to_owned(), "no".to_owned()),
        ]
    );
    let events = events_of(&response);
    assert_eq!(events.len(), 2);
    assert_eq!(
        events[0],
        serde_json::from_str::<Value>(PROGRESS_FIRST).unwrap()
    );
    assert_valid("ProgressNotification", &events[0]);
    assert_valid("JSONRPCResultResponse", &events[1]);
    assert_eq!(events[1]["id"], 1_u64);
}

#[test]
fn a_request_without_a_token_answers_one_json_object_over_http() {
    let response = respond(&export(1, "docs", None));
    assert_eq!(response.status, 200);
    assert_eq!(
        response.headers,
        vec![("Content-Type".to_owned(), "application/json".to_owned())]
    );
    let answered: Value = serde_json::from_slice(&response.body).unwrap();
    assert_valid("JSONRPCResultResponse", &answered);
}

#[test]
fn an_error_answer_closes_the_event_stream_too() {
    let mut asked = call(&id(1), "delete-everything", &json!({}));
    asked["params"]["_meta"]["progressToken"] = json!("export-1");
    let mut unknown = post(&asked);
    unknown.headers[2].1 = "delete-everything".to_owned();
    let settings = McpHttpSettings {
        allowed_origins: &[],
        oauth: None,
    };
    let response = block_on(export_mcp::answer_http(
        &unknown,
        &settings,
        &Archive::default(),
        async |_request: &McpHttpRequest, call: McpCall| {
            Ok::<_, export_mcp::McpDenied>(reporting(&call))
        },
    ));
    let events = events_of(&response);
    assert_eq!(events.len(), 1);
    assert_eq!(events[0]["error"]["code"], -32_602_i64);
    assert_valid("JSONRPCErrorResponse", &events[0]);
}

#[test]
fn the_stdio_host_drops_the_answer_of_an_export_the_client_cancelled() {
    static WORKSPACE: McpEndpoint = McpEndpoint::new(
        &[
            DocumentServiceSchema::MCP_NAMES,
            ExportServiceSchema::MCP_NAMES,
        ],
        &[document_mcp::catalog, export_mcp::catalog],
    );
    let held_archive = Archive::default();
    let held_stdout = Lines::default();
    let held_in_flight = McpInFlight::default();
    let (archive, stdout, in_flight) = (&held_archive, &held_stdout, &held_in_flight);
    let cancelled = json!({
        "jsonrpc": "2.0",
        "method": "notifications/cancelled",
        "params": { "requestId": 1_u64, "reason": "the user stopped it" },
    });
    assert_valid("CancelledNotification", &cancelled);
    let version = call(&id(2), "get-version", &json!({ "document_id": "doc-1" }));
    let mut tasks = Tasks::default();
    let mut seen = Vec::new();
    for line in [export(1, "forever", Some("export-1")), version, cancelled] {
        let text = line.to_string();
        tasks.spawn(async move {
            let answered = export_mcp::serve(
                &text,
                "Workspace",
                None,
                &WORKSPACE,
                |call| reporting(&call),
                export_mcp::McpHost {
                    in_flight,
                    writer: stdout,
                },
                &|ctx: &Reporting, part, method: &str, params: &Value| {
                    let held = archive.clone();
                    Box::pin(async move {
                        match part {
                            0 => document_mcp_route(method, params, &held, ctx).await,
                            1 => export_mcp::route(method, params, &held, ctx).await,
                            _ => None,
                        }
                    })
                },
            )
            .await;
            if let Some(reply) = answered {
                export_mcp::McpEventWriter::write(stdout, reply)
                    .await
                    .unwrap();
            }
        });
        tasks.run_until_idle();
        seen.push((stdout.written().len(), tasks.running()));
    }
    assert_eq!(seen, vec![(1, 1), (2, 1), (2, 0)]);
    let written = stdout.written();
    assert_eq!(written.len(), 2, "{written:?}");
    assert_eq!(written[0], PROGRESS_FIRST);
    let answered: Value = serde_json::from_str(&written[1]).unwrap();
    assert_eq!(answered["id"], 2_u64);
    assert_eq!(
        answered["result"]["structuredContent"],
        json!({ "version": 7_u64 })
    );
    assert_valid("JSONRPCResultResponse", &answered);
}

async fn document_mcp_route(
    method: &str,
    params: &Value,
    archive: &Archive,
    ctx: &Reporting,
) -> Option<Result<Value, Value>> {
    document_mcp::route(method, params, archive, ctx).await
}

#[test]
fn a_cancellation_of_a_request_nobody_runs_changes_nothing() {
    let cancelled = json!({
        "jsonrpc": "2.0",
        "method": "notifications/cancelled",
        "params": { "requestId": "never-sent" },
    });
    let lines = Lines::default();
    let in_flight = McpInFlight::default();
    let answered = block_on(export_mcp::answer(
        &cancelled.to_string(),
        &Archive::default(),
        |call| reporting(&call),
        &lines,
        &in_flight,
    ));
    assert_eq!(answered, None);
    assert_eq!(lines.written(), Vec::<String>::new());
}

#[test]
fn a_call_without_a_token_ignores_progress_in_every_dispatcher() {
    let archive = Archive::default();
    let settings = document_mcp::McpHttpSettings {
        allowed_origins: &[],
        oauth: None,
    };
    let response = block_on(document_mcp::answer_http(
        &http_tests::post(&http_tests::get_version()),
        &settings,
        &archive,
        async |_request: &document_mcp::McpHttpRequest, call: document_mcp::McpCall| {
            call.progress(1.0_f64, None, None);
            assert!(!call.cancelled());
            Ok::<_, document_mcp::McpDenied>(())
        },
    ));
    assert_eq!(response.status, 200);
    let billing_settings = billing_mcp::McpHttpSettings {
        allowed_origins: &[],
        oauth: None,
    };
    let denied = block_on(billing_mcp::answer_http(
        &billing_post(),
        &billing_settings,
        &archive,
        async |_request: &billing_mcp::McpHttpRequest, call: billing_mcp::McpCall| {
            call.progress(1.0_f64, Some(2.0_f64), Some("half"));
            assert!(!call.cancelled());
            Err::<(), _>(billing_mcp::McpDenied::Forbidden)
        },
    ));
    assert_eq!(denied.status, 403);
}

fn billing_post() -> billing_mcp::McpHttpRequest {
    let asked = call(
        &id(1),
        "get-balance",
        &json!({ "value": "acct-1", "x-ledger": "main" }),
    );
    billing_mcp::McpHttpRequest {
        body: asked.to_string().into_bytes(),
        headers: vec![
            ("MCP-Protocol-Version".to_owned(), "2026-07-28".to_owned()),
            ("Mcp-Method".to_owned(), "tools/call".to_owned()),
            ("Mcp-Name".to_owned(), "get-balance".to_owned()),
        ],
        method: "POST".to_owned(),
        path: "/mcp".to_owned(),
    }
}

#[test]
fn serve_http_streams_for_an_endpoint_and_its_admit_may_deny_by_name() {
    static ONLY_EXPORT: McpEndpoint =
        McpEndpoint::new(&[ExportServiceSchema::MCP_NAMES], &[export_mcp::catalog]);
    let settings = McpHttpSettings {
        allowed_origins: &[],
        oauth: None,
    };
    let archive = Archive::default();
    let serve = |asked: &Value, name: &str| {
        let mut posted = post(asked);
        posted.headers[2].1 = name.to_owned();
        block_on(export_mcp::serve_http(
            &posted,
            &settings,
            "Exports",
            None,
            &ONLY_EXPORT,
            async |request: &McpHttpRequest, call: McpCall| match request.mcp_name().as_deref() {
                Some("forbidden") => Err(export_mcp::McpDenied::Forbidden),
                Some("scoped") => Err(export_mcp::McpDenied::InsufficientScope {
                    scope: "exports:write".to_owned(),
                }),
                Some("anonymous") => Err(export_mcp::McpDenied::NoValidCredentials { scope: None }),
                _ => Ok(reporting(&call)),
            },
            &|ctx: &Reporting, part, method: &str, params: &Value| {
                let held = archive.clone();
                Box::pin(async move {
                    match part {
                        0 => export_mcp::route(method, params, &held, ctx).await,
                        _ => None,
                    }
                })
            },
        ))
    };
    let streamed = serve(&export(1, "docs", Some("export-1")), "export-folder");
    assert_eq!(streamed.status, 200);
    assert_eq!(events_of(&streamed).len(), 2);
    for (name, status) in [("forbidden", 403), ("scoped", 403), ("anonymous", 401)] {
        assert_eq!(
            serve(&export(1, "docs", None), name).status,
            status,
            "{name}"
        );
    }
}
