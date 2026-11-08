//! `JobService`, whose operations start a task and answer where it stands, cancel it and answer
//! its questions, the Rust dispatcher that answers it, and the rows the TypeScript, Dart and
//! Kotlin servers are measured against and the clients are read against. Nothing here waits on a
//! clock.

use super::mcp_task_dispatcher as dispatcher;
use crate::mcp_streams::through;
use alloc::collections::{BTreeMap, VecDeque};
use alloc::sync::Arc;
use core::future::{Future, ready};
use core::pin::pin;
use core::task::{Context, Poll, Waker};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::panic::resume_unwind;
use std::sync::Mutex;
use tixschema::{model_schema, service_schema};

/// The official 2026-07-28 schema of the Model Context Protocol.
const MCP_SCHEMA: &str = include_str!("../mcp_declaration_tests/mcp-schema-2026-07-28.json");

/// The official schema of the tasks extension.
const TASKS_SCHEMA: &str =
    include_str!("../mcp_declaration_tests/mcp-tasks-schema-2026-07-28.json");

const EVENT_PREFIX: &str = "event: message\ndata: ";
const STARTED: &str = "2026-11-01T20:00:00Z";
const FINISHED: &str = "2026-11-01T20:04:10Z";

/// The requests whose validation fault is compared without its detail, which each language words
/// in its own terms.
const FAULTED_ANSWERS: [&str; 0] = [];

/// The method and name of each scope query every server answers.
pub const QUERIES: [(&str, Option<&str>); 8] = [
    ("tasks/get", Some("start-export:7f3a")),
    ("tasks/get", Some("start-export")),
    ("tasks/cancel", Some("start-export:7f3a")),
    ("tasks/update", Some("start-export:7f3a")),
    ("tasks/get", Some("other:1")),
    ("tools/call", Some("start-export")),
    ("tools/call", Some("export-status")),
    ("tools/list", None),
];

#[model_schema(enable = ["jsonschema"])]
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case", tag = "errorCode")]
pub enum JobError {
    NotFound,
}

#[model_schema(enable = ["jsonschema"])]
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ExportRequest {
    pub folder: String,
}

#[model_schema(enable = ["jsonschema"])]
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ExportReport {
    pub files: u32,
}

/// Runs the long exports of a workspace.
#[service_schema(transports = ["mcp"])]
pub trait JobService<Ctx> {
    /// Answers a question an export asked.
    #[service_schema_op(mcp(scopes = ["jobs:answer"]))]
    async fn answer_export(
        &self,
        ctx: &Ctx,
        task_id: String,
        key: String,
        action: String,
        content: Option<String>,
    ) -> Result<(), JobError>;

    /// Asks one export to stop.
    #[service_schema_op(mcp(scopes = ["jobs:stop"]))]
    async fn cancel_export(&self, ctx: &Ctx, task_id: String) -> Result<(), JobError>;

    /// Answers where one export stands.
    #[service_schema_op(mcp(scopes = ["jobs:read"]))]
    async fn export_status(
        &self,
        ctx: &Ctx,
        task_id: String,
    ) -> Result<job_service_schema::McpTaskState<ExportReport>, JobError>;

    /// Starts the export of one folder.
    #[service_schema_op(mcp(task(get = "export_status", cancel = "cancel_export", update = "answer_export"), scopes = ["jobs:run"]))]
    async fn start_export(
        &self,
        ctx: &Ctx,
        req: ExportRequest,
    ) -> Result<job_service_schema::McpTask, JobError>;
}

/// Answers every operation, and writes down what reached it. A task is what its id says:
/// `7f3a` works, `done` completed, `needs` waits for the user, `broken` failed and `stopped` was
/// cancelled.
#[derive(Clone, Default)]
pub struct Stand {
    pub reached: Arc<Mutex<Vec<Value>>>,
}

impl Stand {
    fn reach(&self, call: Value) {
        self.reached.lock().unwrap().push(call);
    }

    fn task(id: &str) -> job_service_schema::McpTask {
        job_service_schema::McpTask {
            task_id: id.to_owned(),
            created_at: STARTED.to_owned(),
            last_updated_at: STARTED.to_owned(),
            ttl_ms: Some(3_600_000),
            poll_interval_ms: Some(2000),
        }
    }
}

// Every method below is synchronous work wrapped in an already-ready `Future`: nothing here waits
// on anything.
impl JobService<()> for Stand {
    fn answer_export(
        &self,
        _ctx: &(),
        task_id: String,
        key: String,
        action: String,
        content: Option<String>,
    ) -> impl Future<Output = Result<(), JobError>> {
        self.reach(json!(["answer-export", task_id, key, action, content]));
        ready(if task_id == "gone" {
            Err(JobError::NotFound)
        } else {
            Ok(())
        })
    }

    fn cancel_export(
        &self,
        _ctx: &(),
        task_id: String,
    ) -> impl Future<Output = Result<(), JobError>> {
        self.reach(json!(["cancel-export", task_id]));
        ready(if task_id == "gone" {
            Err(JobError::NotFound)
        } else {
            Ok(())
        })
    }

    fn export_status(
        &self,
        _ctx: &(),
        task_id: String,
    ) -> impl Future<Output = Result<job_service_schema::McpTaskState<ExportReport>, JobError>>
    {
        use job_service_schema::McpTaskState;
        self.reach(json!(["export-status", task_id]));
        if task_id == "boom" {
            resume_unwind(Box::new("the handler came apart"));
        }
        ready(match task_id.as_str() {
            "7f3a" => Ok(McpTaskState::Working {
                task: Self::task(&task_id),
            }),
            "done" => {
                let mut task = Self::task(&task_id);
                FINISHED.clone_into(&mut task.last_updated_at);
                Ok(McpTaskState::Completed {
                    task,
                    result: ExportReport { files: 3 },
                })
            }
            "needs" => Ok(McpTaskState::InputRequired {
                task: Self::task(&task_id),
                input_requests: [(
                    "overwrite".to_owned(),
                    json!({
                        "method": "elicitation/create",
                        "params": {
                            "mode": "form",
                            "message": "Overwrite the folder?",
                            "requestedSchema": { "type": "object", "properties": {} },
                        },
                    }),
                )]
                .into(),
            }),
            "broken" => Ok(McpTaskState::Failed {
                task: Self::task(&task_id),
                error: job_service_schema::McpTaskError {
                    code: -32603,
                    message: "the folder vanished".to_owned(),
                    data: None,
                },
            }),
            "stopped" => Ok(McpTaskState::Cancelled {
                task: Self::task(&task_id),
            }),
            _ => Err(JobError::NotFound),
        })
    }

    fn start_export(
        &self,
        _ctx: &(),
        req: ExportRequest,
    ) -> impl Future<Output = Result<job_service_schema::McpTask, JobError>> {
        self.reach(json!(["start-export", req.folder]));
        ready(if req.folder == "missing" {
            Err(JobError::NotFound)
        } else {
            Ok(Self::task("7f3a"))
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

/// One HTTP request every server is sent: its name, its headers and its body.
pub struct HttpRow {
    pub body: String,
    pub headers: Vec<(String, String)>,
    pub name: &'static str,
}

/// One call a client makes: the answer of the Rust dispatcher it is handed, and the outcome it
/// must read.
pub struct ClientCall {
    pub answer: &'static str,
    pub arguments: Value,
    pub expected: Value,
    pub label: &'static str,
    pub tool: &'static str,
}

/// One `subscriptions/listen` request and the changes its source is handed.
pub struct Listen {
    pub request: String,
    pub script: Vec<Value>,
}

/// Where a stdio host writes: every message kept.
#[derive(Default)]
struct Recorder {
    written: Mutex<Vec<String>>,
}

impl dispatcher::McpEventWriter for Recorder {
    fn write(
        &self,
        message: String,
    ) -> impl Future<Output = Result<(), dispatcher::McpClientGone>> {
        self.written.lock().unwrap().push(message);
        ready(Ok(()))
    }
}

fn request(id: u64, method: &str, params: Value, declared: bool) -> String {
    let mut carried = params;
    carried["_meta"] = json!({
        "io.modelcontextprotocol/protocolVersion": "2026-07-28",
        "io.modelcontextprotocol/clientCapabilities": if declared {
            json!({ "extensions": { "io.modelcontextprotocol/tasks": {} } })
        } else {
            json!({})
        },
    });
    json!({ "jsonrpc": "2.0", "id": id, "method": method, "params": carried }).to_string()
}

fn tool(name: &str, arguments: &Value, declared: bool) -> String {
    request(
        5,
        "tools/call",
        json!({ "name": name, "arguments": arguments }),
        declared,
    )
}

fn task(method: &str, task_id: &str) -> String {
    request(6, method, json!({ "taskId": task_id }), false)
}

/// The requests every server is sent, by name, in the order they are answered.
pub fn requests() -> Vec<(&'static str, String)> {
    let folder = json!({ "folder": "reports" });
    let update = request(
        7,
        "tasks/update",
        json!({
            "taskId": "start-export:needs",
            "inputResponses": {
                "overwrite": { "action": "accept", "content": { "yes": true } },
                "other": { "action": "decline" },
            },
        }),
        false,
    );
    vec![
        ("discover", request(1, "server/discover", json!({}), false)),
        ("tools_list", request(2, "tools/list", json!({}), false)),
        ("start_task", tool("start-export", &folder, true)),
        ("start_plain", tool("start-export", &folder, false)),
        (
            "start_declared_error",
            tool("start-export", &json!({ "folder": "missing" }), true),
        ),
        (
            "status_working",
            tool("export-status", &json!({ "value": "7f3a" }), false),
        ),
        (
            "status_completed",
            tool("export-status", &json!({ "value": "done" }), false),
        ),
        (
            "status_needs",
            tool("export-status", &json!({ "value": "needs" }), false),
        ),
        (
            "status_broken",
            tool("export-status", &json!({ "value": "broken" }), false),
        ),
        (
            "status_stopped",
            tool("export-status", &json!({ "value": "stopped" }), false),
        ),
        (
            "status_gone",
            tool("export-status", &json!({ "value": "gone" }), false),
        ),
        (
            "status_boom",
            tool("export-status", &json!({ "value": "boom" }), false),
        ),
        (
            "cancel_tool",
            tool("cancel-export", &json!({ "value": "7f3a" }), false),
        ),
        (
            "answer_tool",
            tool(
                "answer-export",
                &json!({ "taskId": "needs", "key": "overwrite", "action": "accept", "content": "{}" }),
                false,
            ),
        ),
        ("get_working", task("tasks/get", "start-export:7f3a")),
        ("get_completed", task("tasks/get", "start-export:done")),
        ("get_needs", task("tasks/get", "start-export:needs")),
        ("get_broken", task("tasks/get", "start-export:broken")),
        ("get_stopped", task("tasks/get", "start-export:stopped")),
        ("get_declared", task("tasks/get", "start-export:gone")),
        ("get_other_tool", task("tasks/get", "other:1")),
        ("get_no_colon", task("tasks/get", "7f3a")),
        ("get_nameless", request(6, "tasks/get", json!({}), false)),
        ("get_panic", task("tasks/get", "start-export:boom")),
        ("cancel_ok", task("tasks/cancel", "start-export:7f3a")),
        ("cancel_declared", task("tasks/cancel", "start-export:gone")),
        ("update_ok", update),
        ("update_bare", task("tasks/update", "start-export:needs")),
    ]
}

/// The HTTP requests every server is sent: `tasks/get` with no `Mcp-Name`, the same with a progress
/// token, a started task with a token, and a start whose `Mcp-Name` is wrong.
pub fn http_rows() -> Vec<HttpRow> {
    let headers = |method: &str, named: Option<&str>| {
        let mut held = vec![
            ("Content-Type".to_owned(), "application/json".to_owned()),
            ("MCP-Protocol-Version".to_owned(), "2026-07-28".to_owned()),
            ("Mcp-Method".to_owned(), method.to_owned()),
        ];
        if let Some(written) = named {
            held.push(("Mcp-Name".to_owned(), written.to_owned()));
        }
        held
    };
    let getting = task("tasks/get", "start-export:done");
    let mut tokened: Value = serde_json::from_str(&getting).unwrap();
    tokened["params"]["_meta"]["progressToken"] = json!("export-1");
    let starting = tool("start-export", &json!({ "folder": "reports" }), true);
    let mut started: Value = serde_json::from_str(&starting).unwrap();
    started["params"]["_meta"]["progressToken"] = json!("export-2");
    vec![
        HttpRow {
            body: getting,
            headers: headers("tasks/get", None),
            name: "http_get",
        },
        HttpRow {
            body: tokened.to_string(),
            headers: headers("tasks/get", None),
            name: "http_get_streamed",
        },
        HttpRow {
            body: started.to_string(),
            headers: headers("tools/call", Some("start-export")),
            name: "http_start_streamed",
        },
        HttpRow {
            body: starting,
            headers: headers("tools/call", Some("another")),
            name: "http_start_mismatched",
        },
    ]
}

/// The `subscriptions/listen` of a client that follows one task, and what its source hands over.
pub fn listens() -> Vec<Listen> {
    let following = request(
        1,
        "subscriptions/listen",
        json!({ "notifications": { "taskIds": ["start-export:7f3a"] } }),
        false,
    );
    vec![Listen {
        request: following,
        script: vec![
            json!({ "kind": "task", "taskId": "start-export:other", "state": moved_state("other") }),
            json!({ "kind": "task", "taskId": "start-export:7f3a", "state": moved_state("7f3a") }),
            json!({ "kind": "keepAlive" }),
        ],
    }]
}

/// What the Rust dispatcher answers a request, and what reached its handler.
fn answered(text: &str) -> (String, Vec<Value>) {
    let stand = Stand::default();
    let reply = ready_output(dispatcher::answer(
        text,
        &stand,
        |_call| (),
        &Nowhere,
        &dispatcher::McpInFlight::default(),
    ))
    .unwrap();
    let reached = stand.reached.lock().unwrap().clone();
    (reply, reached)
}

fn http_answered(headers: &[(String, String)], body: &str) -> Value {
    let settings = dispatcher::McpHttpSettings {
        allowed_origins: &[],
        oauth: None,
    };
    let asked = dispatcher::McpHttpRequest {
        body: body.as_bytes().to_vec(),
        headers: headers.to_vec(),
        method: "POST".to_owned(),
        path: "/mcp".to_owned(),
    };
    let response = through(|stream| {
        ready_output(dispatcher::answer_http(
            &asked,
            &settings,
            stream,
            &Stand::default(),
            async |_request, _call| Ok(()),
        ))
    });
    json!({ "status": response.status, "body": String::from_utf8(response.body).unwrap() })
}

fn change_of(spec: &Value) -> dispatcher::McpChange {
    match spec["kind"].as_str().unwrap() {
        "task" => dispatcher::McpChange::Task {
            task_id: spec["taskId"].as_str().unwrap().to_owned(),
            state: spec["state"].clone(),
        },
        _ => dispatcher::McpChange::KeepAlive,
    }
}

fn listened(listen: &Listen) -> Value {
    let recorder = Recorder::default();
    let mut queue: VecDeque<_> = listen.script.iter().map(change_of).collect();
    let mut source = async || queue.pop_front();
    let reply = ready_output(dispatcher::answer_listening(
        &listen.request,
        &Stand::default(),
        |_call| (),
        &recorder,
        &dispatcher::McpInFlight::default(),
        &mut source,
    ));
    json!({ "reply": reply, "written": recorder.written.lock().unwrap().clone() })
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
    let http: BTreeMap<_, _> = http_rows()
        .into_iter()
        .map(|row| (row.name, http_answered(&row.headers, &row.body)))
        .collect();
    let scopes: Vec<Vec<&str>> = QUERIES
        .iter()
        .map(|(method, name)| JobServiceSchema::mcp_scopes(method, *name).to_vec())
        .collect();
    let listening: Vec<Value> = listens().iter().map(listened).collect();
    json!({ "answers": answers, "reached": reached, "http": http, "scopes": scopes, "listens": listening })
}

/// What the Rust side hands each language's server program.
pub fn input() -> Value {
    let requests: BTreeMap<_, _> = requests().into_iter().collect();
    let http: Vec<Value> = http_rows()
        .into_iter()
        .map(|row| {
            json!({
                "name": row.name,
                "headers": row.headers.iter().map(|(key, value)| json!([key, value])).collect::<Vec<_>>(),
                "body": row.body,
            })
        })
        .collect();
    let queries: Vec<Value> = QUERIES
        .iter()
        .map(|(method, name)| json!([method, name]))
        .collect();
    let listening: Vec<Value> = listens()
        .into_iter()
        .map(|listen| json!({ "request": listen.request, "script": listen.script }))
        .collect();
    json!({ "requests": requests, "http": http, "queries": queries, "listens": listening })
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

/// Holds `message` to the definition of the schema of the tasks extension named `definition`.
fn held_to_tasks(definition: &str, message: &Value) {
    let schema: Value = serde_json::from_str(TASKS_SCHEMA).unwrap();
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

/// The definition of the tasks schema the result of the request `name` is held to.
fn task_result_of(name: &str) -> Option<&'static str> {
    match name {
        "start_task" => Some("CreateTaskResult"),
        "get_working" | "get_completed" | "get_needs" | "get_broken" | "get_stopped" => {
            Some("GetTaskResult")
        }
        "cancel_ok" => Some("CancelTaskResult"),
        "update_ok" => Some("UpdateTaskResult"),
        _ => None,
    }
}

/// The definition of the tasks schema the request `name` is held to.
fn task_request_of(name: &str) -> Option<&'static str> {
    match name {
        "get_working" | "get_completed" | "get_needs" | "get_broken" | "get_stopped"
        | "get_declared" | "get_other_tool" | "get_no_colon" | "get_panic" => {
            Some("GetTaskRequest")
        }
        "cancel_ok" | "cancel_declared" => Some("CancelTaskRequest"),
        "update_ok" => Some("UpdateTaskRequest"),
        _ => None,
    }
}

/// The state of a completed task a source of changes hands over.
fn moved_state(id: &str) -> Value {
    json!({
        "status": "completed",
        "taskId": id,
        "createdAt": STARTED,
        "lastUpdatedAt": FINISHED,
        "ttlMs": 3_600_000_u64,
        "pollIntervalMs": 2000_u64,
        "result": { "files": 3_u32 },
    })
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

/// The JSON text an answer carries, whether it is the body or the one event of a stream.
fn event_data(body: &str) -> &str {
    body.strip_prefix(EVENT_PREFIX).unwrap_or(body).trim_end()
}

fn read(text: &str) -> Value {
    floats(&serde_json::from_str(text).unwrap())
}

/// The definition of the official schema an answer to the request `name` is held to.
fn definition_of(name: &str) -> Option<&'static str> {
    match name {
        "discover" => Some("DiscoverResult"),
        "tools_list" => Some("ListToolsResult"),
        "start_task" | "get_working" | "get_completed" | "get_needs" | "get_broken"
        | "get_stopped" | "cancel_ok" | "update_ok" => Some("Result"),
        _ if name.starts_with("start_")
            || name.starts_with("status_")
            || name.ends_with("_tool") =>
        {
            Some("CallToolResult")
        }
        _ => None,
    }
}

fn without_detail(answer: &Value) -> Value {
    let mut stripped = answer.clone();
    let fault = stripped["error"]["data"]["fault"].as_object_mut().unwrap();
    fault.remove("detail");
    stripped["error"]["message"] = Value::Null;
    stripped
}

/// Holds the HTTP answers a language's server program reported to the Rust dispatcher's.
fn assert_http(report: &Value, rust: &Value) {
    for row in http_rows() {
        let name = row.name;
        assert_eq!(
            report["http"][name]["status"], rust["http"][name]["status"],
            "{name}"
        );
        assert_eq!(
            read(event_data(report["http"][name]["body"].as_str().unwrap())),
            read(event_data(rust["http"][name]["body"].as_str().unwrap())),
            "{name}"
        );
    }
    let streamed = rust["http"]["http_get_streamed"]["body"].as_str().unwrap();
    assert!(streamed.starts_with(EVENT_PREFIX), "{streamed}");
    let started = rust["http"]["http_start_streamed"]["body"]
        .as_str()
        .unwrap();
    assert!(started.starts_with(EVENT_PREFIX), "{started}");
    assert!(
        !rust["http"]["http_get"]["body"]
            .as_str()
            .unwrap()
            .starts_with(EVENT_PREFIX)
    );
    assert_eq!(rust["http"]["http_get"]["status"], json!(200_u16));
    assert_eq!(
        rust["http"]["http_start_mismatched"]["status"],
        json!(400_u16)
    );
}

/// Holds the report of a language's server program to what the Rust dispatcher answered, and
/// every answer to the official schema.
pub fn assert_matches(report: &Value) {
    let rust = rust_report();
    for (name, text) in requests() {
        if let Some(definition) = task_request_of(name) {
            held_to_tasks(definition, &serde_json::from_str(&text).unwrap());
        }
    }
    for (name, _) in requests() {
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
        if message.get("error").is_some() {
            held_to("JSONRPCErrorResponse", &message);
        } else {
            held_to("JSONRPCResultResponse", &message);
            if let Some(definition) = definition_of(name) {
                held_to(definition, &message["result"]);
            }
            if name == "get_completed" {
                held_to("CallToolResult", &message["result"]["result"]);
            }
            if let Some(definition) = task_result_of(name) {
                held_to_tasks(definition, &message["result"]);
            }
            if name == "discover" {
                held_to_tasks(
                    "TasksExtensionCapability",
                    &message["result"]["capabilities"]["extensions"]["io.modelcontextprotocol/tasks"],
                );
            }
        }
        assert_eq!(
            report["reached"][name], rust["reached"][name],
            "{name} reached"
        );
    }
    assert_http(report, &rust);
    assert_eq!(report["scopes"], rust["scopes"], "scopes");
    assert_eq!(rust["scopes"][0], json!(["jobs:read"]));
    assert_eq!(rust["scopes"][1], json!(["jobs:read"]));
    assert_eq!(rust["scopes"][2], json!(["jobs:stop"]));
    assert_eq!(rust["scopes"][3], json!(["jobs:answer"]));
    assert_eq!(rust["scopes"][4], json!([]));
    assert_eq!(rust["scopes"][5], json!(["jobs:run"]));
    assert_eq!(report["listens"], rust["listens"], "listens");
    let written = rust["listens"][0]["written"].as_array().unwrap();
    assert_eq!(written.len(), 2, "{written:?}");
    let acknowledged: Value = serde_json::from_str(written[0].as_str().unwrap()).unwrap();
    held_to_tasks(
        "TaskSubscriptionAcknowledgedNotifications",
        &acknowledged["params"]["notifications"],
    );
    let notification: Value = serde_json::from_str(written[1].as_str().unwrap()).unwrap();
    held_to_tasks("TaskStatusNotification", &notification);
    assert_eq!(notification["method"], "notifications/tasks");
    assert_eq!(notification["params"]["taskId"], "start-export:7f3a");
}

fn handle() -> Value {
    json!({
        "taskId": "7f3a",
        "createdAt": STARTED,
        "lastUpdatedAt": STARTED,
        "ttlMs": 3_600_000_u64,
        "pollIntervalMs": 2000_u64,
    })
}

fn state(status: &str, id: &str, extra: &Value) -> Value {
    let mut held = handle();
    held["taskId"] = json!(id);
    held["status"] = json!(status);
    if let Value::Object(more) = extra {
        for (key, value) in more {
            held[key] = value.clone();
        }
    }
    held
}

fn client_calls() -> Vec<ClientCall> {
    let needs = state(
        "input_required",
        "needs",
        &json!({ "inputRequests": { "overwrite": { "method": "elicitation/create", "params": {
            "mode": "form", "message": "Overwrite the folder?",
            "requestedSchema": { "type": "object", "properties": {} } } } } }),
    );
    let mut done = state(
        "completed",
        "done",
        &json!({ "result": { "files": 3_u32 } }),
    );
    done["lastUpdatedAt"] = json!(FINISHED);
    vec![
        ClientCall {
            answer: "start_plain",
            arguments: json!({ "folder": "reports" }),
            expected: json!({ "ok": handle() }),
            label: "started",
            tool: "start-export",
        },
        ClientCall {
            answer: "status_working",
            arguments: json!({ "value": "7f3a" }),
            expected: json!({ "ok": state("working", "7f3a", &json!({})) }),
            label: "working",
            tool: "export-status",
        },
        ClientCall {
            answer: "status_completed",
            arguments: json!({ "value": "done" }),
            expected: json!({ "ok": done }),
            label: "completed",
            tool: "export-status",
        },
        ClientCall {
            answer: "status_needs",
            arguments: json!({ "value": "needs" }),
            expected: json!({ "ok": needs }),
            label: "waiting",
            tool: "export-status",
        },
        ClientCall {
            answer: "status_broken",
            arguments: json!({ "value": "broken" }),
            expected: json!({ "ok": state("failed", "broken", &json!({ "error": { "code": -32603_i64, "message": "the folder vanished" } })) }),
            label: "failed",
            tool: "export-status",
        },
        ClientCall {
            answer: "status_stopped",
            arguments: json!({ "value": "stopped" }),
            expected: json!({ "ok": state("cancelled", "stopped", &json!({})) }),
            label: "cancelled",
            tool: "export-status",
        },
        ClientCall {
            answer: "cancel_tool",
            arguments: json!({ "value": "7f3a" }),
            expected: json!({ "ok": null }),
            label: "cancel",
            tool: "cancel-export",
        },
        ClientCall {
            answer: "answer_tool",
            arguments: json!({ "taskId": "needs", "key": "overwrite", "action": "accept", "content": "{}" }),
            expected: json!({ "ok": null }),
            label: "answer",
            tool: "answer-export",
        },
        ClientCall {
            answer: "status_gone",
            arguments: json!({ "value": "gone" }),
            expected: json!({ "declared": { "errorCode": "not-found" } }),
            label: "declared",
            tool: "export-status",
        },
        ClientCall {
            answer: "status_boom",
            arguments: json!({ "value": "boom" }),
            expected: json!({ "fault": "handler-panic" }),
            label: "panicked",
            tool: "export-status",
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
        Value::Array(items) => Value::Array(items.iter().map(without_nulls).collect()),
        Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_) => value.clone(),
    }
}

/// Holds the outcomes a client read, and the requests it sent, to the calls above. A client never
/// declares the extension, so it reads each handle as an ordinary success.
pub fn assert_client(report: &Value) {
    let calls = client_calls();
    let sent = report["seen"].as_array().unwrap();
    assert_eq!(sent.len(), calls.len(), "{report:#}");
    for (call, request) in calls.iter().zip(sent) {
        held_to("CallToolRequest", request);
        assert_eq!(request["method"], "tools/call", "{}", call.label);
        assert_eq!(request["params"]["name"], call.tool, "{}", call.label);
        assert_eq!(
            without_nulls(&request["params"]["arguments"]),
            call.arguments,
            "{}",
            call.label
        );
        assert_eq!(
            without_nulls(
                &request["params"]["_meta"]["io.modelcontextprotocol/clientCapabilities"]
            ),
            json!({}),
            "{}",
            call.label
        );
        assert_eq!(
            floats(&without_nulls(&report["outcomes"][call.label])),
            floats(&without_nulls(&call.expected)),
            "{}",
            call.label
        );
    }
}

#[test]
fn the_rust_dispatcher_answers_alone_beside_an_endpoint_and_over_http_alike() {
    let (alone, served, status) =
        rust_serves_alike!(dispatcher, JobServiceSchema, Stand::default(), ready_output);
    assert_eq!(alone, served);
    assert!(alone.is_some());
    assert_eq!(status, 200);
}

#[test]
fn the_rust_dispatcher_listens_alike_through_every_entry_point() {
    let (alone, served) =
        rust_listens_alike!(dispatcher, JobServiceSchema, Stand::default(), ready_output);
    assert_eq!(alone, served);
    assert!(alone.is_some());
}

#[test]
fn an_admit_that_denies_stops_a_task_method_before_its_handler_runs() {
    let settings = dispatcher::McpHttpSettings {
        allowed_origins: &[],
        oauth: None,
    };
    let row = http_rows().remove(0);
    let stand = Stand::default();
    let asked = dispatcher::McpHttpRequest {
        body: row.body.into_bytes(),
        headers: row.headers,
        method: "POST".to_owned(),
        path: "/mcp".to_owned(),
    };
    for (denied, status) in [
        (dispatcher::McpDenied::Forbidden, 403_u16),
        (
            dispatcher::McpDenied::InsufficientScope {
                scope: "jobs:read".to_owned(),
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
                    assert_eq!(request.mcp_name(), None);
                    assert_eq!(request.bearer_token(), None);
                    assert_eq!(request.header("mcp-method"), Some("tasks/get"));
                    Err::<(), _>(denied.clone())
                },
            ))
        });
        assert_eq!(response.status, status);
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
