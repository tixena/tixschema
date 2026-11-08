//! `subscriptions/listen` as the TypeScript, Dart and Kotlin `mcp` servers are held to it: the
//! requests and the changes each is handed, what the Rust dispatcher of `LibraryService` writes
//! and answers to them, and the check of every message against the official 2026-07-28 schema.
//! Nothing here waits on a clock; a stream ends on the changes it is handed.

use super::mcp_bare_dispatcher;
use super::mcp_depot_dispatcher;
use super::mcp_library_dispatcher::{
    self, McpCall, McpChange, McpClientGone, McpEndpoint, McpEventWriter, McpHost, McpHttpRequest,
    McpHttpSettings, McpInFlight, McpServer,
};
use super::mcp_library_service::Stand;
use super::mcp_writer_rows::{Tasks, ready_output};
use super::{BareServiceSchema, DepotServiceSchema, LibraryServiceSchema};
use alloc::collections::VecDeque;
use core::future::{Future, pending, ready};
use serde_json::{Value, json};
use std::sync::Mutex;

/// The official 2026-07-28 schema of the Model Context Protocol.
const MCP_SCHEMA: &str = include_str!("../mcp_declaration_tests/mcp-schema-2026-07-28.json");

const VERSION: &str = "2026-07-28";
const WATCHED: &str = "library://covers/book-1";
const OTHER: &str = "library://covers/book-2";

const SETTINGS: McpHttpSettings<'static> = McpHttpSettings {
    allowed_origins: &[],
    oauth: None,
};

/// One request to `LibraryService` and the changes its source is handed.
pub struct Listen {
    pub name: &'static str,
    pub request: String,
    pub script: Vec<Value>,
}

/// One plain HTTP request.
pub struct ListenRow {
    pub body: String,
    pub headers: Vec<(String, String)>,
    pub method: &'static str,
    pub path: &'static str,
}

/// Where a stdio host writes: every message kept, or every write refused once the client is gone.
#[derive(Default)]
struct Recorder {
    gone: bool,
    written: Mutex<Vec<String>>,
}

impl Recorder {
    fn written(&self) -> Vec<String> {
        self.written.lock().unwrap().clone()
    }
}

impl McpEventWriter for Recorder {
    fn write(&self, message: String) -> impl Future<Output = Result<(), McpClientGone>> + Send {
        let kept = !self.gone;
        if kept {
            self.written.lock().unwrap().push(message);
        }
        ready(kept.then_some(()).ok_or(McpClientGone))
    }
}

fn meta() -> Value {
    json!({
        "io.modelcontextprotocol/protocolVersion": VERSION,
        "io.modelcontextprotocol/clientCapabilities": {},
    })
}

fn asked(method: &str, params: &Value) -> Value {
    let mut carried = params.clone();
    carried["_meta"] = meta();
    json!({ "jsonrpc": "2.0", "id": 1, "method": method, "params": carried })
}

fn listen_request(filter: &Value) -> Value {
    asked("subscriptions/listen", &json!({ "notifications": filter }))
}

fn headers_of(method: &str) -> Vec<(String, String)> {
    vec![
        ("MCP-Protocol-Version".to_owned(), VERSION.to_owned()),
        ("Mcp-Method".to_owned(), method.to_owned()),
    ]
}

fn updated(uri: &str) -> Value {
    json!({ "kind": "resourceUpdated", "uri": uri })
}

fn listed() -> Value {
    json!({ "kind": "resourcesListChanged" })
}

fn kept_alive() -> Value {
    json!({ "kind": "keepAlive" })
}

fn moved() -> Value {
    json!({ "kind": "task", "taskId": "export:1" })
}

/// The requests of a stdio host, each with the changes its source is handed.
pub fn listens() -> Vec<Listen> {
    let both = json!({ "resourcesListChanged": true, "resourceSubscriptions": [WATCHED] });
    let mut not_boolean = listen_request(&json!({ "resourcesListChanged": "yes" }));
    not_boolean["id"] = json!("listen-2");
    vec![
        Listen {
            name: "both kinds asked",
            request: listen_request(&both).to_string(),
            script: vec![
                listed(),
                updated(OTHER),
                updated(WATCHED),
                kept_alive(),
                moved(),
            ],
        },
        Listen {
            name: "only the resources named",
            request: listen_request(&json!({ "resourceSubscriptions": [WATCHED, OTHER] }))
                .to_string(),
            script: vec![updated(WATCHED), updated(OTHER), listed()],
        },
        Listen {
            name: "kinds the server never acknowledges",
            request: listen_request(
                &json!({ "toolsListChanged": true, "promptsListChanged": true }),
            )
            .to_string(),
            script: vec![listed(), updated(WATCHED)],
        },
        Listen {
            name: "a source with nothing to say",
            request: listen_request(&json!({ "resourcesListChanged": true })).to_string(),
            script: Vec::new(),
        },
        Listen {
            name: "no notifications named",
            request: asked("subscriptions/listen", &json!({})).to_string(),
            script: Vec::new(),
        },
        Listen {
            name: "a flag that is not a boolean",
            request: not_boolean.to_string(),
            script: Vec::new(),
        },
        Listen {
            name: "a subscription that is not a list of strings",
            request: listen_request(&json!({ "resourceSubscriptions": [7] })).to_string(),
            script: Vec::new(),
        },
    ]
}

/// The request the cancellation stops, and the cancellation.
pub fn cancel_lines() -> [String; 2] {
    [
        listen_request(&json!({ "resourcesListChanged": true })).to_string(),
        json!({
            "jsonrpc": "2.0",
            "method": "notifications/cancelled",
            "params": { "requestId": 1 },
        })
        .to_string(),
    ]
}

/// The request whose every write fails, and the changes its source is handed.
pub fn gone_line() -> (String, Vec<Value>) {
    (
        listen_request(&json!({ "resourcesListChanged": true })).to_string(),
        vec![listed()],
    )
}

pub fn discover_line() -> String {
    asked("server/discover", &json!({})).to_string()
}

fn http_row(request: &str) -> ListenRow {
    ListenRow {
        body: request.to_owned(),
        headers: headers_of("subscriptions/listen"),
        method: "POST",
        path: "/mcp",
    }
}

/// The requests over plain HTTP terms, each with the changes its source is handed.
pub fn http_listens() -> Vec<(ListenRow, Vec<Value>)> {
    let all = listens();
    [0_usize, 1, 4]
        .into_iter()
        .map(|at| (http_row(&all[at].request), all[at].script.clone()))
        .collect()
}

/// The request every dispatcher is asked to listen on, whatever the service declares.
pub fn alike_row() -> ListenRow {
    http_row(&listens()[0].request)
}

/// What the Rust side hands each language's program.
pub fn input() -> Value {
    let (gone_request, gone_script) = gone_line();
    json!({
        "listens": listens().iter().map(|held| json!({
            "name": held.name,
            "request": held.request,
            "script": held.script,
        })).collect::<Vec<_>>(),
        "http": http_listens().iter().map(|(row, script)| json!({
            "body": row.body,
            "headers": row.headers.iter().map(|(name, value)| json!([name, value])).collect::<Vec<_>>(),
            "method": row.method,
            "path": row.path,
            "script": script,
        })).collect::<Vec<_>>(),
        "cancel": cancel_lines(),
        "gone": { "request": gone_request, "script": gone_script },
        "discover": discover_line(),
    })
}

fn change_of(spec: &Value) -> McpChange {
    match spec["kind"].as_str().unwrap() {
        "resourcesListChanged" => McpChange::ResourcesListChanged,
        "resourceUpdated" => McpChange::ResourceUpdated {
            uri: spec["uri"].as_str().unwrap().to_owned(),
        },
        "task" => McpChange::Task {
            task_id: spec["taskId"].as_str().unwrap().to_owned(),
        },
        _ => McpChange::KeepAlive,
    }
}

fn queue_of(script: &[Value]) -> VecDeque<McpChange> {
    script.iter().map(change_of).collect()
}

fn stdio(request: &str, script: &[Value]) -> Value {
    let lines = Recorder::default();
    let mut queue = queue_of(script);
    let mut source = async || queue.pop_front();
    let reply = ready_output(mcp_library_dispatcher::answer_listening(
        request,
        &Stand,
        |_call| (),
        &lines,
        &McpInFlight::default(),
        &mut source,
    ));
    json!({ "reply": reply, "written": lines.written() })
}

fn workspace(request: &str, script: &[Value]) -> Value {
    let endpoint = McpEndpoint::new(
        &[
            LibraryServiceSchema::MCP_NAMES,
            DepotServiceSchema::MCP_NAMES,
        ],
        &[
            mcp_library_dispatcher::catalog,
            mcp_depot_dispatcher::catalog,
        ],
    );
    let server = McpServer {
        endpoint: &endpoint,
        instructions: None,
        name: "Workspace",
    };
    let lines = Recorder::default();
    let mut queue = queue_of(script);
    let mut source = async || queue.pop_front();
    let reply = ready_output(mcp_library_dispatcher::serve_listening(
        request,
        &server,
        |_call| (),
        McpHost {
            in_flight: &McpInFlight::default(),
            writer: &lines,
        },
        &|ctx: &(), part, method: &str, params: &Value| {
            Box::pin(async move {
                match part {
                    0 => mcp_library_dispatcher::route(method, params, &Stand, ctx).await,
                    1 => mcp_depot_dispatcher::route(method, params, &Stand, ctx).await,
                    _ => None,
                }
            })
        },
        &mut source,
    ));
    json!({ "reply": reply, "written": lines.written() })
}

fn http_terms(row: &ListenRow) -> McpHttpRequest {
    McpHttpRequest {
        body: row.body.clone().into_bytes(),
        headers: row.headers.clone(),
        method: row.method.to_owned(),
        path: row.path.to_owned(),
    }
}

fn http_json(response: mcp_library_dispatcher::McpHttpResponse) -> Value {
    json!({
        "body": String::from_utf8(response.body).unwrap(),
        "headers": response.headers.iter().map(|(name, value)| json!([name, value])).collect::<Vec<_>>(),
        "status": response.status,
    })
}

fn http(row: &ListenRow, script: &[Value]) -> Value {
    let mut queue = queue_of(script);
    let mut source = async || queue.pop_front();
    http_json(ready_output(mcp_library_dispatcher::answer_http_listening(
        &http_terms(row),
        &SETTINGS,
        &Stand,
        async |_request: &McpHttpRequest, _call: McpCall| {
            Ok::<_, mcp_library_dispatcher::McpDenied>(())
        },
        &mut source,
    )))
}

fn plain_http(row: &ListenRow) -> Value {
    http_json(ready_output(mcp_library_dispatcher::answer_http(
        &http_terms(row),
        &SETTINGS,
        &Stand,
        async |_request: &McpHttpRequest, _call: McpCall| {
            Ok::<_, mcp_library_dispatcher::McpDenied>(())
        },
    )))
}

fn plain_stdio(request: &str) -> Option<String> {
    ready_output(mcp_library_dispatcher::answer(
        request,
        &Stand,
        |_call| (),
        &Recorder::default(),
        &McpInFlight::default(),
    ))
}

fn discovered(with_changes: bool) -> Option<String> {
    let request = discover_line();
    if !with_changes {
        return plain_stdio(&request);
    }
    let mut queue = VecDeque::<McpChange>::new();
    let mut source = async || queue.pop_front();
    ready_output(mcp_library_dispatcher::answer_listening(
        &request,
        &Stand,
        |_call| (),
        &Recorder::default(),
        &McpInFlight::default(),
        &mut source,
    ))
}

fn cancelled() -> Value {
    let [listening, cancelling] = cancel_lines();
    let lines = Recorder::default();
    let in_flight = McpInFlight::default();
    let mut source = async || pending::<Option<McpChange>>().await;
    let replies = Mutex::new(Vec::new());
    let mut tasks = Tasks::default();
    tasks.spawn(async {
        let reply = mcp_library_dispatcher::answer_listening(
            &listening,
            &Stand,
            |_call| (),
            &lines,
            &in_flight,
            &mut source,
        )
        .await;
        replies.lock().unwrap().push(reply);
    });
    tasks.run_until_idle();
    let before = lines.written().len();
    tasks.spawn(async {
        let reply = mcp_library_dispatcher::answer(
            &cancelling,
            &Stand,
            |_call| (),
            &Recorder::default(),
            &in_flight,
        )
        .await;
        assert_eq!(reply, None);
    });
    tasks.run_until_idle();
    json!({
        "reply": replies.lock().unwrap().first().cloned().flatten(),
        "running": tasks.running(),
        "start": before,
        "written": lines.written(),
    })
}

fn gone() -> Value {
    let (request, script) = gone_line();
    let lines = Recorder {
        gone: true,
        written: Mutex::new(Vec::new()),
    };
    let mut queue = queue_of(&script);
    let mut source = async || queue.pop_front();
    let reply = ready_output(mcp_library_dispatcher::answer_listening(
        &request,
        &Stand,
        |_call| (),
        &lines,
        &McpInFlight::default(),
        &mut source,
    ));
    json!({ "reply": reply, "written": lines.written() })
}

/// What the Rust dispatcher writes and answers for the requests above, in the shape each
/// language's program reports.
pub fn rust_report() -> Value {
    let all = listens();
    let (plain_row, _) = &http_listens()[0];
    json!({
        "cancelled": cancelled(),
        "discover": { "listening": discovered(true), "plain": discovered(false) },
        "gone": gone(),
        "http": http_listens().iter().map(|(row, script)| http(row, script)).collect::<Vec<_>>(),
        "plain": { "http": plain_http(plain_row), "stdio": plain_stdio(&all[0].request) },
        "stdio": all.iter().map(|held| stdio(&held.request, &held.script)).collect::<Vec<_>>(),
        "workspace": workspace(&all[0].request, &all[0].script),
    })
}

fn schema_errors(definition: &str, instance: &Value) -> Vec<String> {
    let schema: Value = serde_json::from_str(MCP_SCHEMA).unwrap();
    let rooted = json!({ "$ref": format!("#/$defs/{definition}"), "$defs": schema["$defs"] });
    let validator = jsonschema::validator_for(&rooted).unwrap();
    validator
        .iter_errors(instance)
        .map(|error| format!("{error} at {}", error.instance_path()))
        .collect()
}

fn held_to(definition: &str, instance: &Value) {
    assert_eq!(
        schema_errors(definition, instance),
        Vec::<String>::new(),
        "{definition}: {instance:#}"
    );
}

/// Holds one message of a subscription stream to the definition of the official schema it is.
fn assert_message_valid(text: &str) {
    let message: Value = serde_json::from_str(text).unwrap();
    if message.get("error").is_some() {
        held_to("JSONRPCErrorResponse", &message);
        return;
    }
    if message.get("result").is_some() {
        held_to("SubscriptionsListenResultResponse", &message);
        return;
    }
    let method = message["method"].as_str().unwrap();
    let definition = [
        (
            "notifications/subscriptions/acknowledged",
            "SubscriptionsAcknowledgedNotification",
        ),
        (
            "notifications/resources/updated",
            "ResourceUpdatedNotification",
        ),
        (
            "notifications/resources/list_changed",
            "ResourceListChangedNotification",
        ),
    ]
    .into_iter()
    .find(|(written, _)| *written == method)
    .map(|(_, definition)| definition)
    .unwrap();
    held_to(definition, &message);
}

fn event_texts(body: &str) -> Vec<&str> {
    body.split_terminator("\n\n")
        .map(|event| {
            event
                .strip_prefix("event: message\ndata: ")
                .unwrap_or(event)
        })
        .collect()
}

/// Holds the report of a language's program to what the Rust dispatcher wrote and answered, and
/// every message to the official schema.
pub fn assert_matches(report: &Value) {
    let rust = rust_report();
    assert_eq!(report["stdio"], rust["stdio"], "{report:#}");
    assert_eq!(report["workspace"], rust["workspace"], "{report:#}");
    assert_eq!(report["http"], rust["http"], "{report:#}");
    assert_eq!(report["plain"], rust["plain"], "{report:#}");
    assert_eq!(report["discover"], rust["discover"], "{report:#}");
    assert_eq!(report["gone"], rust["gone"], "{report:#}");
    assert_eq!(report["cancelled"], rust["cancelled"], "{report:#}");
    for held in report["stdio"].as_array().unwrap().iter().chain([
        &report["workspace"],
        &report["gone"],
        &report["cancelled"],
    ]) {
        for text in held["written"].as_array().unwrap() {
            assert_message_valid(text.as_str().unwrap());
        }
        if let Some(reply) = held["reply"].as_str() {
            assert_message_valid(reply);
        }
    }
    for answered in report["http"].as_array().unwrap() {
        for event in event_texts(answered["body"].as_str().unwrap()) {
            if event != ":" {
                assert_message_valid(event);
            }
        }
    }
    let first = &report["stdio"][0];
    assert_eq!(first["written"].as_array().unwrap().len(), 3, "{first:#}");
    assert_eq!(
        report["http"][0]["body"]
            .as_str()
            .unwrap()
            .matches(":\n\n")
            .count(),
        1
    );
    assert_eq!(report["cancelled"]["start"], 1_u64);
    assert_eq!(report["cancelled"]["running"], 0_u64);
    assert_eq!(report["cancelled"]["reply"], Value::Null);
    assert_eq!(report["plain"]["http"]["status"], 404_u64);
    held_to(
        "DiscoverResult",
        &serde_json::from_str::<Value>(report["discover"]["listening"].as_str().unwrap()).unwrap()
            ["result"],
    );
    assert_eq!(
        serde_json::from_str::<Value>(report["discover"]["listening"].as_str().unwrap()).unwrap()["result"]
            ["capabilities"]["resources"],
        json!({ "listChanged": true, "subscribe": true })
    );
    assert_eq!(
        serde_json::from_str::<Value>(report["discover"]["plain"].as_str().unwrap()).unwrap()["result"]
            ["capabilities"]["resources"],
        json!({})
    );
}

#[test]
fn the_rust_dispatchers_listen_alike_through_every_entry_point() {
    let (alone, served) = rust_listens_alike!(
        mcp_library_dispatcher,
        LibraryServiceSchema,
        Stand,
        ready_output
    );
    assert_eq!(alone, served);
    assert!(alone.is_some());
}

#[test]
fn the_depot_and_the_bare_dispatcher_listen_alike_through_every_entry_point() {
    let (depot, depot_served) = rust_listens_alike!(
        mcp_depot_dispatcher,
        DepotServiceSchema,
        Stand,
        ready_output
    );
    assert_eq!(depot, depot_served);
    assert!(depot.is_some());
    let (bare, bare_served) =
        rust_listens_alike!(mcp_bare_dispatcher, BareServiceSchema, Stand, ready_output);
    assert_eq!(bare, bare_served);
    assert!(bare.is_some());
}

#[test]
fn the_rust_report_holds_to_the_schema_and_to_the_design() {
    assert_matches(&rust_report());
}
