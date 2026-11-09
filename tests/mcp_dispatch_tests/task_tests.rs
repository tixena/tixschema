//! Tasks: the handle a task tool answers to a client that declared the extension, the ordinary
//! success it answers to any other, and `tasks/get`, `tasks/cancel` and `tasks/update` calling the
//! operations the tool names. The store is the stand-in's; nothing here waits on a clock.

mod over_http {
    use super::{
        HEADER_MISMATCH, JobServiceSchema, Jobs, Recorded, Value, assert_result, block_on,
        declared, id, job_mcp, json, through,
    };
    use crate::http_tests::post;
    use crate::job_mcp::{McpCall, McpDenied, McpEndpoint, McpHttpRequest, McpHttpSettings};
    use crate::support::{call, request};

    const SETTINGS: McpHttpSettings<'static> = McpHttpSettings::new(&[], None);

    fn asking(message: &Value, named: Option<&str>) -> McpHttpRequest {
        let from = post(message);
        let mut headers = from.headers().to_vec();
        headers.retain(|(held, _)| !held.eq_ignore_ascii_case("mcp-name"));
        if let Some(written) = named {
            headers.push(("Mcp-Name".to_owned(), written.to_owned()));
        }
        McpHttpRequest::new(
            from.method().to_owned(),
            from.path().to_owned(),
            headers,
            from.body().to_vec(),
        )
    }

    fn started(tokened: bool) -> Value {
        let mut asked = declared(call(
            &id(1),
            "start-export",
            &json!({ "folder": "reports" }),
        ));
        if tokened {
            asked["params"]["_meta"]["progressToken"] = json!("export-1");
        }
        asked
    }

    #[test]
    fn a_task_request_answers_one_object_and_a_tokened_one_answers_an_event_stream() {
        let tasks = request(
            &id(2),
            "tasks/get",
            json!({ "taskId": "start-export:7f3a" }),
        );
        let answered = through(|stream| {
            block_on(job_mcp::answer_http(
                &asking(&tasks, None),
                &SETTINGS,
                stream,
                &Jobs::default(),
                async |_request, _call| Ok(job_mcp::McpAdmitted::new(())),
            ))
        });
        assert_eq!(answered.status(), 200);
        assert_result("Result", &serde_json::from_slice(answered.body()).unwrap());
        let stream = Recorded::default();
        let streamed = block_on(job_mcp::answer_http(
            &asking(&started(true), Some("start-export")),
            &SETTINGS,
            &stream,
            &Jobs::default(),
            async |_request, _call| Ok(job_mcp::McpAdmitted::new(())),
        ));
        assert!(streamed.is_none());
        let body = String::from_utf8(stream.streamed().body).unwrap();
        let data = body
            .strip_prefix("event: message\ndata: ")
            .unwrap()
            .trim_end();
        let result: Value = serde_json::from_str(data).unwrap();
        assert_result("Result", &result);
        assert_eq!(result["result"]["resultType"], "task");
    }

    #[test]
    fn a_task_method_needs_no_name_header_and_a_mismatched_tool_name_is_refused() {
        let tasks = request(
            &id(2),
            "tasks/get",
            json!({ "taskId": "start-export:7f3a" }),
        );
        let mismatched = through(|stream| {
            block_on(job_mcp::answer_http(
                &asking(&started(false), Some("another")),
                &SETTINGS,
                stream,
                &Jobs::default(),
                async |_request, _call| Ok(job_mcp::McpAdmitted::new(())),
            ))
        });
        assert_eq!(mismatched.status(), 400);
        let body: Value = serde_json::from_slice(mismatched.body()).unwrap();
        assert_eq!(body["error"]["code"], HEADER_MISMATCH);
        let matched = through(|stream| {
            block_on(job_mcp::answer_http(
                &asking(&tasks, Some("something else")),
                &SETTINGS,
                stream,
                &Jobs::default(),
                async |_request, _call| Ok(job_mcp::McpAdmitted::new(())),
            ))
        });
        assert_eq!(matched.status(), 200);
    }

    #[test]
    fn a_denied_task_request_stops_before_its_handler() {
        let jobs = Jobs::default();
        let endpoint: &McpEndpoint = &super::ENDPOINT;
        let tasks = request(
            &id(2),
            "tasks/get",
            json!({ "taskId": "start-export:7f3a" }),
        );
        for denied in [
            McpDenied::NoValidCredentials { scope: None },
            McpDenied::InsufficientScope {
                scope: JobServiceSchema::mcp_scopes("tasks/get", Some("start-export:7f3a"))[0]
                    .to_owned(),
            },
            McpDenied::Forbidden,
        ] {
            let response = through(|stream| {
                block_on(job_mcp::serve_http(
                    &asking(&tasks, Some("start-export")),
                    &SETTINGS,
                    stream,
                    &job_mcp::McpServer::new(endpoint, None, "Jobs"),
                    async |request: &job_mcp::McpHttpRequest, call: McpCall| {
                        assert_eq!(request.mcp_name().as_deref(), Some("start-export"));
                        call.progress(1.0_f64, None, None);
                        Err::<job_mcp::McpAdmitted<()>, _>(denied.clone())
                    },
                    &|ctx: &(), part, method: &str, params: &Value| {
                        let held = jobs.clone();
                        Box::pin(async move {
                            match part {
                                0 => job_mcp::route(method, params, &held, ctx).await,
                                _ => None,
                            }
                        })
                    },
                ))
            });
            assert!(matches!(response.status(), 401 | 403), "{denied:?}");
        }
        assert!(jobs.reached.lock().unwrap().is_empty());
    }
}

use crate::JobServiceSchema;
use crate::counsel::Counsels;
use crate::counsel_mcp;
use crate::http_tests::post;
use crate::job::{ExportRequest, FINISHED, JobError, Jobs, STARTED};
use crate::job_client::{JobServiceClient, McpMessage, Transport};
use crate::job_mcp::{self, McpEndpoint, McpHost, McpInFlight};
use crate::job_service_schema::CallError;
use crate::support::{
    HEADER_MISMATCH, INVALID_PARAMS, Lines, METHOD_NOT_FOUND, Recorded, assert_error,
    assert_result, assert_task, assert_valid, block_on, call, id, request, server_info, through,
};
use core::future::Future;
use serde_json::{Value, json};
use std::sync::Mutex;

const TASKS: &str = "io.modelcontextprotocol/tasks";
const ENDPOINT: McpEndpoint = McpEndpoint::new(&[JobServiceSchema::MCP_NAMES], &[job_mcp::catalog]);

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
        let jobs = JOBS.with(Clone::clone);
        async move {
            Ok(block_on(job_mcp::answer(
                message.text(),
                &jobs,
                |_call| (),
                &Lines::default(),
                &McpInFlight::default(),
            ))
            .unwrap())
        }
    }
}

std::thread_local! {
    static JOBS: Jobs = Jobs::default();
}

fn declared(mut asked: Value) -> Value {
    asked["params"]["_meta"]["io.modelcontextprotocol/clientCapabilities"] =
        json!({ "extensions": { TASKS: {} } });
    asked
}

fn answer_with(jobs: &Jobs, asked: &Value) -> Value {
    block_on(job_mcp::answer(
        &asked.to_string(),
        jobs,
        |_call| (),
        &Lines::default(),
        &McpInFlight::default(),
    ))
    .map(|text| serde_json::from_str(&text).unwrap())
    .unwrap()
}

fn started(declaring: bool) -> Value {
    let asked = call(&id(1), "start-export", &json!({ "folder": "reports" }));
    answer_with(
        &Jobs::default(),
        &if declaring { declared(asked) } else { asked },
    )
}

fn task_request(method: &str, task_id: &str) -> Value {
    let asked = request(&id(2), method, json!({ "taskId": task_id }));
    match method {
        "tasks/get" => assert_task("GetTaskRequest", &asked),
        "tasks/cancel" => assert_task("CancelTaskRequest", &asked),
        _ => {}
    }
    asked
}

fn tasks_get(jobs: &Jobs, task_id: &str) -> Value {
    answer_with(jobs, &task_request("tasks/get", task_id))
}

#[test]
fn server_discover_adds_the_tasks_extension() {
    let answered = answer_with(
        &Jobs::default(),
        &request(&id(1), "server/discover", json!({})),
    );
    assert_result("DiscoverResult", &answered);
    assert_eq!(
        answered["result"]["capabilities"]["extensions"],
        json!({ TASKS: {} })
    );
    assert_task(
        "TasksExtensionCapability",
        &answered["result"]["capabilities"]["extensions"][TASKS],
    );
}

#[test]
fn a_client_that_declared_the_extension_receives_the_task_with_the_tool_s_name_in_its_id() {
    let answered = started(true);
    assert_result("Result", &answered);
    assert_task("CreateTaskResult", &answered["result"]);
    assert_eq!(
        answered,
        json!({
            "jsonrpc": "2.0",
            "id": 1_u64,
            "result": {
                "resultType": "task",
                "taskId": "start-export:7f3a",
                "status": "working",
                "createdAt": STARTED,
                "lastUpdatedAt": STARTED,
                "ttlMs": 3_600_000_u64,
                "pollIntervalMs": 2000_u64,
                "_meta": server_info("JobService"),
            },
        })
    );
}

#[test]
fn any_other_client_receives_the_handle_as_an_ordinary_success() {
    let answered = started(false);
    assert_result("CallToolResult", &answered);
    let handle = json!({
        "taskId": "7f3a",
        "status": "working",
        "createdAt": STARTED,
        "lastUpdatedAt": STARTED,
        "ttlMs": 3_600_000_u64,
        "pollIntervalMs": 2000_u64,
    });
    assert_eq!(answered["result"]["resultType"], "complete");
    assert_eq!(answered["result"]["structuredContent"], handle);
    assert_eq!(answered["result"]["isError"], false);
}

#[test]
fn the_status_and_cancel_operations_are_tools_too() {
    let jobs = Jobs::default();
    let status = answer_with(
        &jobs,
        &call(&id(3), "export-status", &json!({ "value": "7f3a" })),
    );
    assert_result("CallToolResult", &status);
    assert_eq!(status["result"]["structuredContent"]["status"], "working");
    assert_eq!(status["result"]["structuredContent"]["taskId"], "7f3a");
    let cancelled = answer_with(
        &jobs,
        &call(&id(4), "cancel-export", &json!({ "value": "7f3a" })),
    );
    assert_result("CallToolResult", &cancelled);
    assert_eq!(cancelled["result"]["isError"], false);
    let listed = answer_with(&jobs, &request(&id(5), "tools/list", json!({})));
    let names: Vec<&str> = listed["result"]["tools"]
        .as_array()
        .unwrap()
        .iter()
        .map(|tool| tool["name"].as_str().unwrap())
        .collect();
    assert_eq!(
        names,
        [
            "answer-export",
            "cancel-export",
            "count-exports",
            "export-status",
            "start-export"
        ]
    );
    for tool in listed["result"]["tools"].as_array().unwrap() {
        let schema = tool.get("outputSchema").is_some();
        assert_eq!(schema, tool["name"] == "count-exports", "{tool:#}");
    }
}

#[test]
fn tasks_get_works_then_completes_with_the_result_the_tool_would_have_answered() {
    let jobs = Jobs::default();
    let working = tasks_get(&jobs, "start-export:7f3a");
    assert_result("Result", &working);
    assert_task("GetTaskResult", &working["result"]);
    assert_eq!(
        working["result"],
        json!({
            "resultType": "complete",
            "taskId": "start-export:7f3a",
            "status": "working",
            "createdAt": STARTED,
            "lastUpdatedAt": STARTED,
            "ttlMs": 3_600_000_u64,
            "pollIntervalMs": 2000_u64,
            "_meta": server_info("JobService"),
        })
    );
    let completed = tasks_get(&jobs, "start-export:7f3a");
    assert_result("Result", &completed);
    assert_task("GetTaskResult", &completed["result"]);
    assert_valid("CallToolResult", &completed["result"]["result"]);
    assert_eq!(
        completed["result"],
        json!({
            "resultType": "complete",
            "taskId": "start-export:7f3a",
            "status": "completed",
            "createdAt": STARTED,
            "lastUpdatedAt": FINISHED,
            "ttlMs": 3_600_000_u64,
            "pollIntervalMs": 2000_u64,
            "result": {
                "resultType": "complete",
                "content": [{ "type": "text", "text": "{\"files\":3}" }],
                "structuredContent": { "files": 3_u32 },
                "isError": false,
            },
            "_meta": server_info("JobService"),
        })
    );
    assert_eq!(
        jobs.reached.lock().unwrap().clone(),
        ["export-status 7f3a", "export-status 7f3a"]
    );
}

#[test]
fn a_task_that_waits_failed_or_stopped_answers_its_status() {
    let jobs = Jobs::default();
    let waiting = tasks_get(&jobs, "start-export:needs");
    assert_result("Result", &waiting);
    assert_task("GetTaskResult", &waiting["result"]);
    assert_eq!(waiting["result"]["status"], "input_required");
    assert_eq!(
        waiting["result"]["inputRequests"]["overwrite"]["method"],
        "elicitation/create"
    );
    let broken = tasks_get(&jobs, "start-export:broken");
    assert_result("Result", &broken);
    assert_task("GetTaskResult", &broken["result"]);
    assert_eq!(broken["result"]["status"], "failed");
    assert_eq!(
        broken["result"]["error"],
        json!({ "code": -32603_i64, "message": "the folder vanished" })
    );
    let stopped = tasks_get(&jobs, "start-export:stopped");
    assert_result("Result", &stopped);
    assert_task("GetTaskResult", &stopped["result"]);
    assert_eq!(stopped["result"]["status"], "cancelled");
}

#[test]
fn an_id_a_task_tool_does_not_own_is_unknown_and_the_declared_error_is_the_data() {
    let jobs = Jobs::default();
    for task_id in ["no-such-tool:1", "7f3a"] {
        let answered = tasks_get(&jobs, task_id);
        assert_error("JSONRPCErrorResponse", &answered);
        assert_eq!(answered["error"]["code"], INVALID_PARAMS);
        assert_eq!(
            answered["error"]["data"]["fault"]["kind"],
            "unknown-operation"
        );
    }
    let gone = tasks_get(&jobs, "start-export:gone");
    assert_error("JSONRPCErrorResponse", &gone);
    assert_eq!(gone["error"]["code"], INVALID_PARAMS);
    assert_eq!(gone["error"]["data"], json!({ "errorCode": "not-found" }));
    let nameless = answer_with(&jobs, &request(&id(2), "tasks/get", json!({})));
    assert_error("JSONRPCErrorResponse", &nameless);
    assert_eq!(nameless["error"]["code"], INVALID_PARAMS);
}

#[test]
fn tasks_cancel_calls_the_cancel_operation() {
    let jobs = Jobs::default();
    let answered = answer_with(&jobs, &task_request("tasks/cancel", "start-export:7f3a"));
    assert_result("Result", &answered);
    assert_task("CancelTaskResult", &answered["result"]);
    assert_eq!(answered["result"]["resultType"], "complete");
    let refused = answer_with(&jobs, &task_request("tasks/cancel", "start-export:gone"));
    assert_error("JSONRPCErrorResponse", &refused);
    assert_eq!(
        refused["error"]["data"],
        json!({ "errorCode": "not-found" })
    );
    assert_eq!(
        jobs.reached.lock().unwrap().clone(),
        ["cancel-export 7f3a", "cancel-export gone"]
    );
}

#[test]
fn tasks_update_hands_each_answer_to_the_update_operation() {
    let jobs = Jobs::default();
    let asked = request(
        &id(2),
        "tasks/update",
        json!({
            "taskId": "start-export:needs",
            "inputResponses": {
                "overwrite": { "action": "accept", "content": { "yes": true } },
                "other": { "action": "decline" },
            },
        }),
    );
    assert_task("UpdateTaskRequest", &asked);
    let answered = answer_with(&jobs, &asked);
    assert_result("Result", &answered);
    assert_task("UpdateTaskResult", &answered["result"]);
    assert_eq!(
        jobs.reached.lock().unwrap().clone(),
        [
            "answer-export needs other decline None",
            "answer-export needs overwrite accept Some(\"{\\\"yes\\\":true}\")",
        ]
    );
    let bare = answer_with(&jobs, &task_request("tasks/update", "start-export:needs"));
    assert_error("JSONRPCErrorResponse", &bare);
    assert_eq!(bare["error"]["code"], INVALID_PARAMS);
}

#[test]
fn over_http_a_task_method_of_a_service_with_no_task_is_a_method_not_found() {
    let asked = task_request("tasks/get", "start-export:7f3a");
    let posted = post(&asked);
    let request = counsel_mcp::McpHttpRequest::new(
        posted.method().to_owned(),
        posted.path().to_owned(),
        vec![
            ("MCP-Protocol-Version".to_owned(), "2026-07-28".to_owned()),
            ("Mcp-Method".to_owned(), "tasks/get".to_owned()),
        ],
        posted.body().to_vec(),
    );
    let response = through(|stream| {
        block_on(counsel_mcp::answer_http(
            &request,
            &counsel_mcp::McpHttpSettings::new(&[], None),
            stream,
            &Counsels::default(),
            async |_request, _call| Ok(counsel_mcp::McpAdmitted::new(())),
        ))
    });
    assert_eq!(response.status(), 404);
    let body: Value = serde_json::from_slice(response.body()).unwrap();
    assert_eq!(body["error"]["code"], METHOD_NOT_FOUND);
}

#[test]
fn a_service_with_no_task_does_not_answer_the_task_methods() {
    let asked = task_request("tasks/get", "start-export:7f3a");
    let answered = block_on(counsel_mcp::answer(
        &asked.to_string(),
        &Counsels::default(),
        |_call| (),
        &Lines::default(),
        &counsel_mcp::McpInFlight::default(),
    ))
    .map(|text| serde_json::from_str::<Value>(&text).unwrap())
    .unwrap();
    assert_error("JSONRPCErrorResponse", &answered);
    assert_eq!(answered["error"]["code"], METHOD_NOT_FOUND);
}

#[test]
fn the_scopes_of_the_task_methods_are_those_of_the_operations_they_call() {
    for (method, scopes) in [
        ("tasks/get", ["jobs:read"]),
        ("tasks/cancel", ["jobs:stop"]),
        ("tasks/update", ["jobs:answer"]),
    ] {
        assert_eq!(
            JobServiceSchema::mcp_scopes(method, Some("start-export:7f3a")),
            scopes
        );
        assert_eq!(
            JobServiceSchema::mcp_scopes(method, Some("start-export")),
            scopes
        );
        assert_eq!(
            JobServiceSchema::mcp_scopes(method, Some("other:1")),
            &[] as &[&str]
        );
    }
    assert_eq!(
        JobServiceSchema::mcp_scopes("tools/call", Some("start-export")),
        ["jobs:run"]
    );
}

#[test]
fn an_endpoint_routes_the_task_methods_and_refuses_them_where_no_service_offers_the_extension() {
    let served = |asked: &Value| {
        block_on(job_mcp::serve(
            &asked.to_string(),
            "Jobs",
            None,
            &ENDPOINT,
            |_call| (),
            McpHost::new(&McpInFlight::default(), &Lines::default()),
            &|_ctx: &(), part, method: &str, params: &Value| {
                Box::pin(async move {
                    match part {
                        0 => job_mcp::route(method, params, &Jobs::default(), &()).await,
                        _ => None,
                    }
                })
            },
        ))
        .map(|text| serde_json::from_str::<Value>(&text).unwrap())
        .unwrap()
    };
    let got = served(&task_request("tasks/get", "start-export:7f3a"));
    assert_result("Result", &got);
    assert_eq!(got["result"]["status"], "working");
    let unknown = served(&task_request("tasks/get", "other:1"));
    assert_eq!(unknown["error"]["message"], "Unknown task: other:1");
}

#[test]
fn the_client_reads_the_handle_and_the_state_as_ordinary_successes() {
    let client = JobServiceClient::new(Dispatching {
        sent: Mutex::new(Vec::new()),
    });
    let handle = block_on(client.start_export(ExportRequest {
        folder: "reports".to_owned(),
    }))
    .unwrap();
    assert_eq!(handle.task_id, "7f3a");
    assert_eq!(handle.created_at, STARTED);
    assert_eq!(handle.ttl_ms, Some(3_600_000));
    let first = block_on(client.export_status(handle.task_id.clone())).unwrap();
    assert_eq!(serde_json::to_value(&first).unwrap()["status"], "working");
    let second = block_on(client.export_status(handle.task_id.clone())).unwrap();
    assert_eq!(second.task().last_updated_at, FINISHED);
    let completed = serde_json::to_value(&second).unwrap();
    assert_eq!(completed["status"], "completed");
    assert_eq!(completed["result"], json!({ "files": 3_u32 }));
    block_on(client.cancel_export(handle.task_id)).unwrap();
    block_on(client.answer_export(
        "needs".to_owned(),
        "overwrite".to_owned(),
        "accept".to_owned(),
        None,
    ))
    .unwrap();
    assert_eq!(block_on(client.count_exports()).unwrap().files, 1);
    let waiting = block_on(client.export_status("needs".to_owned())).unwrap();
    assert_eq!(
        serde_json::to_value(&waiting).unwrap()["status"],
        "input_required"
    );
    let sent = client.transport().sent.lock().unwrap();
    assert_eq!(sent[0].0, "tools/call");
    assert_eq!(sent[0].1.as_deref(), Some("start-export"));
}

#[test]
fn the_client_reads_a_declared_error_and_a_fault() {
    let client = JobServiceClient::new(Dispatching {
        sent: Mutex::new(Vec::new()),
    });
    assert!(matches!(
        block_on(client.export_status("gone".to_owned())),
        Err(CallError::Operation(JobError::NotFound))
    ));
    let missing = block_on(client.start_export(ExportRequest {
        folder: "missing".to_owned(),
    }));
    assert!(matches!(
        missing,
        Err(CallError::Operation(JobError::NotFound))
    ));
    assert!(matches!(
        block_on(client.export_status(String::new())),
        Err(CallError::Operation(JobError::NotFound))
    ));
    let faulted = block_on(client.cancel_export("gone".to_owned()));
    assert!(matches!(
        faulted,
        Err(CallError::Operation(JobError::NotFound))
    ));
}

#[test]
fn each_tool_of_a_task_service_passes_the_tool_definition_and_the_names_hold_them() {
    let tools = JobServiceSchema::mcp_tools();
    assert_eq!(tools.len(), 5);
    for tool in &tools {
        assert_valid("Tool", tool);
    }
    assert_eq!(
        JobServiceSchema::MCP_NAMES,
        &[
            ("tool", "answer-export"),
            ("tool", "cancel-export"),
            ("tool", "count-exports"),
            ("tool", "export-status"),
            ("tool", "start-export"),
        ]
    );
}
