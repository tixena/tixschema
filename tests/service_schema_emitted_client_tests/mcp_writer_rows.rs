//! `WriterService`, whose handler reports progress and stops when it is cancelled, the Rust
//! dispatcher that answers it, and the rows the TypeScript, Dart and Kotlin servers are measured
//! against: the three lines of a stdio host, a write that fails, and the HTTP requests that
//! answer an event stream. Nothing here waits on a clock.

use super::mcp_streams::Recorded;
use super::mcp_writer_dispatcher as dispatcher;
use crate::mcp_streams::through;
use alloc::sync::Arc;
use alloc::task::Wake;
use core::future::{Future, poll_fn, ready};
use core::pin::{Pin, pin};
use core::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use core::task::{Context, Poll, Waker};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::sync::Mutex;
use tixschema::{model_schema, service_schema};

/// The official 2026-07-28 schema of the Model Context Protocol.
const MCP_SCHEMA: &str = include_str!("../mcp_declaration_tests/mcp-schema-2026-07-28.json");

/// The row whose fault carries the words of each language's own reader, so its `detail` is left
/// out of the comparison.
const WORDED_BY_THE_LANGUAGE: usize = 4;

const STREAM_SETTINGS: dispatcher::McpHttpSettings<'static> = dispatcher::McpHttpSettings {
    allowed_origins: &[],
    oauth: None,
};

#[model_schema(enable = ["jsonschema"])]
#[derive(Debug, Deserialize, Serialize)]
pub struct WriterVersionRequest {
    #[model_schema_prop(minLength = 1)]
    pub document_id: String,
}

#[model_schema(enable = ["jsonschema"])]
#[derive(Debug, Deserialize, Serialize)]
pub struct WriterVersion {
    pub version: u32,
}

#[model_schema(enable = ["jsonschema"])]
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case", tag = "errorCode")]
pub enum WriterRefusal {
    NotFound,
}

/// Exports folders and answers versions.
#[service_schema(transports = ["mcp"])]
pub trait WriterService<Ctx> {
    /// Exports one folder, reporting one step of three.
    async fn export_folder(&self, ctx: &Ctx, folder: String) -> Result<(), WriterRefusal>;

    /// Answers the current version of one document.
    async fn get_version(
        &self,
        ctx: &Ctx,
        req: WriterVersionRequest,
    ) -> Result<WriterVersion, WriterRefusal>;
}

type ProgressFn = dyn Fn(f64, Option<f64>, Option<&str>) + Send + Sync;

/// What a handler reaches of the request in flight.
pub struct Reporting {
    cancelled: Box<dyn Fn() -> bool + Send + Sync>,
    gate: Arc<Gate>,
    progress: Box<ProgressFn>,
}

impl Reporting {
    fn gated(call: &dispatcher::McpCall, gate: Arc<Gate>) -> Self {
        let (reporter, watcher) = (call.clone(), call.clone());
        Self {
            cancelled: Box::new(move || watcher.cancelled()),
            gate,
            progress: Box::new(move |done, total, message| reporter.progress(done, total, message)),
        }
    }

    fn of(call: &dispatcher::McpCall) -> Self {
        Self::gated(call, Arc::new(Gate::default()))
    }
}

/// What the folder `held` waits for: the test opens it.
#[derive(Default)]
pub struct Gate {
    open: AtomicBool,
    waiting: Mutex<Option<Waker>>,
}

impl Gate {
    fn open(&self) {
        self.open.store(true, Ordering::SeqCst);
        let waiting = self.waiting.lock().unwrap().take();
        if let Some(waker) = waiting {
            waker.wake();
        }
    }

    async fn opened(&self) {
        poll_fn(|cx| {
            if self.open.load(Ordering::SeqCst) {
                return Poll::Ready(());
            }
            *self.waiting.lock().unwrap() = Some(cx.waker().clone());
            Poll::Pending
        })
        .await;
    }
}

/// Reports one step, and for the folder `forever` waits for the cancellation, for the folder `held`
/// waits for the test to open its gate.
struct Stand;

impl WriterService<Reporting> for Stand {
    async fn export_folder(&self, ctx: &Reporting, folder: String) -> Result<(), WriterRefusal> {
        (ctx.progress)(1.0_f64, Some(3.0_f64), Some("exported 1 of 3 files"));
        if folder == "held" {
            ctx.gate.opened().await;
        }
        if folder == "forever" {
            poll_fn(|_cx| {
                if (ctx.cancelled)() {
                    Poll::Ready(())
                } else {
                    Poll::Pending
                }
            })
            .await;
        }
        Ok(())
    }

    async fn get_version(
        &self,
        _ctx: &Reporting,
        _req: WriterVersionRequest,
    ) -> Result<WriterVersion, WriterRefusal> {
        ready(()).await;
        Ok(WriterVersion { version: 7 })
    }
}

impl WriterService<()> for Stand {
    async fn export_folder(&self, _ctx: &(), _folder: String) -> Result<(), WriterRefusal> {
        ready(()).await;
        Ok(())
    }

    async fn get_version(
        &self,
        _ctx: &(),
        _req: WriterVersionRequest,
    ) -> Result<WriterVersion, WriterRefusal> {
        ready(()).await;
        Ok(WriterVersion { version: 7 })
    }
}

/// Where a stdio host writes: every message kept, or every write refused once the client is gone.
#[derive(Default)]
struct Lines {
    gone: bool,
    written: Mutex<Vec<String>>,
}

impl Lines {
    fn written(&self) -> Vec<String> {
        self.written.lock().unwrap().clone()
    }
}

impl dispatcher::McpEventWriter for Lines {
    fn write(
        &self,
        message: String,
    ) -> impl Future<Output = Result<(), dispatcher::McpClientGone>> {
        let kept = !self.gone;
        if kept {
            self.written.lock().unwrap().push(message);
        }
        ready(kept.then_some(()).ok_or(dispatcher::McpClientGone))
    }
}

struct Woken(AtomicBool);

impl Wake for Woken {
    fn wake(self: Arc<Self>) {
        self.0.store(true, Ordering::SeqCst);
    }
}

type Task<'task> = Pin<Box<dyn Future<Output = ()> + 'task>>;

/// Tasks polled in turn, each only after something woke it.
#[derive(Default)]
pub struct Tasks<'tasks> {
    running: Vec<(Arc<Woken>, Task<'tasks>)>,
}

impl<'tasks> Tasks<'tasks> {
    pub fn run_until_idle(&mut self) {
        loop {
            let mut polled = false;
            self.running.retain_mut(|(woken, task)| {
                if !woken.0.swap(false, Ordering::SeqCst) {
                    return true;
                }
                polled = true;
                let waker = Waker::from(Arc::clone(woken));
                task.as_mut()
                    .poll(&mut Context::from_waker(&waker))
                    .is_pending()
            });
            if !polled {
                return;
            }
        }
    }

    pub fn running(&self) -> usize {
        self.running.len()
    }

    pub fn spawn(&mut self, task: impl Future<Output = ()> + 'tasks) {
        self.running
            .push((Arc::new(Woken(AtomicBool::new(true))), Box::pin(task)));
    }
}

/// One HTTP request: its method, headers and body.
pub struct HttpRow {
    pub body: String,
    pub headers: Vec<(String, String)>,
    pub method: &'static str,
}

/// What `answering` answers on its first poll.
pub fn ready_output<Answering>(answering: Answering) -> Answering::Output
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

fn meta() -> Value {
    json!({
        "io.modelcontextprotocol/protocolVersion": "2026-07-28",
        "io.modelcontextprotocol/clientCapabilities": {},
    })
}

fn tool_call(id: u64, tool: &str, arguments: &Value, token: Option<&str>) -> String {
    let mut params = json!({ "name": tool, "arguments": arguments, "_meta": meta() });
    if let Some(wanted) = token {
        params["_meta"]["progressToken"] = json!(wanted);
    }
    json!({ "jsonrpc": "2.0", "id": id, "method": "tools/call", "params": params }).to_string()
}

/// The export that reports a step and runs until it is cancelled, the version call, and the
/// cancellation of the export.
pub fn lines() -> [String; 3] {
    [
        tool_call(
            1,
            "export-folder",
            &json!({ "value": "forever" }),
            Some("export-1"),
        ),
        tool_call(2, "get-version", &json!({ "document_id": "doc-1" }), None),
        json!({
            "jsonrpc": "2.0",
            "method": "notifications/cancelled",
            "params": { "requestId": 1, "reason": "the user stopped it" },
        })
        .to_string(),
    ]
}

/// An export that finishes at once, whose progress the client has no way to receive.
pub fn failed_line() -> String {
    tool_call(
        1,
        "export-folder",
        &json!({ "value": "docs" }),
        Some("export-1"),
    )
}

fn posted(body: String, tool: &str) -> HttpRow {
    HttpRow {
        body,
        headers: vec![
            ("MCP-Protocol-Version".to_owned(), "2026-07-28".to_owned()),
            ("Mcp-Method".to_owned(), "tools/call".to_owned()),
            ("Mcp-Name".to_owned(), tool.to_owned()),
        ],
        method: "POST",
    }
}

/// An export with a token, an export without one, a version call with a token, an unknown tool
/// with a token, and a version call with a token whose argument the validator refuses.
pub fn http_rows() -> Vec<HttpRow> {
    let docs = json!({ "value": "docs" });
    let version = json!({ "document_id": "doc-1" });
    vec![
        posted(
            tool_call(1, "export-folder", &docs, Some("export-1")),
            "export-folder",
        ),
        posted(tool_call(2, "export-folder", &docs, None), "export-folder"),
        posted(
            tool_call(3, "get-version", &version, Some("version-1")),
            "get-version",
        ),
        posted(
            tool_call(4, "delete-everything", &json!({}), Some("export-1")),
            "delete-everything",
        ),
        posted(
            tool_call(
                5,
                "get-version",
                &json!({ "document_id": "" }),
                Some("version-2"),
            ),
            "get-version",
        ),
    ]
}

fn request_row(row: &HttpRow) -> Value {
    json!({
        "body": row.body,
        "headers": row.headers.iter().map(|(name, value)| json!([name, value])).collect::<Vec<_>>(),
        "method": row.method,
        "path": "/mcp",
    })
}

/// What the Rust side hands each language's program.
pub fn input() -> Value {
    json!({
        "lines": lines(),
        "failed": failed_line(),
        "streams": json!({
            "held": request_row(&posted(tool_call(1, "export-folder", &json!({ "value": "held" }), Some("export-1")), "export-folder")),
            "docs": request_row(&posted(tool_call(2, "export-folder", &json!({ "value": "docs" }), Some("export-1")), "export-folder")),
            "plain": request_row(&posted(tool_call(3, "export-folder", &json!({ "value": "docs" }), None), "export-folder")),
        }),
        "http": http_rows().iter().map(|row| json!({
            "body": row.body,
            "headers": row.headers.iter().map(|(name, value)| json!([name, value])).collect::<Vec<_>>(),
            "method": row.method,
            "path": "/mcp",
        })).collect::<Vec<_>>(),
    })
}

fn stdio_written() -> Vec<String> {
    let stand = Stand;
    let stdout = Lines::default();
    let in_flight = dispatcher::McpInFlight::default();
    let mut tasks = Tasks::default();
    for line in lines() {
        let (out, flying, standing) = (&stdout, &in_flight, &stand);
        tasks.spawn(async move {
            let answered =
                dispatcher::answer(&line, standing, |call| Reporting::of(&call), out, flying).await;
            if let Some(reply) = answered {
                dispatcher::McpEventWriter::write(out, reply).await.unwrap();
            }
        });
        tasks.run_until_idle();
    }
    stdout.written()
}

fn failed_write() -> Value {
    let stdout = Lines {
        gone: true,
        written: Mutex::new(Vec::new()),
    };
    let seen = Mutex::new(None);
    let answered = ready_output(dispatcher::answer(
        &failed_line(),
        &Stand,
        |call| {
            *seen.lock().unwrap() = Some(call.clone());
            Reporting::of(&call)
        },
        &stdout,
        &dispatcher::McpInFlight::default(),
    ));
    let cancelled = seen.lock().unwrap().as_ref().unwrap().cancelled();
    json!({
        "cancelled": cancelled,
        "reply": answered,
        "written": stdout.written(),
    })
}

fn http_answer(row: &HttpRow) -> Value {
    let settings = dispatcher::McpHttpSettings {
        allowed_origins: &[],
        oauth: None,
    };
    let asked = dispatcher::McpHttpRequest {
        body: row.body.clone().into_bytes(),
        headers: row.headers.clone(),
        method: row.method.to_owned(),
        path: "/mcp".to_owned(),
    };
    let response = through(|stream| {
        ready_output(dispatcher::answer_http(
            &asked,
            &settings,
            stream,
            &Stand,
            async |_request: &dispatcher::McpHttpRequest, call: dispatcher::McpCall| {
                Ok::<_, dispatcher::McpDenied>(Reporting::of(&call))
            },
        ))
    });
    json!({
        "body": String::from_utf8(response.body).unwrap(),
        "headers": response.headers.iter().map(|(name, value)| json!([name, value])).collect::<Vec<_>>(),
        "status": response.status,
    })
}

fn streamed_request(row: &HttpRow) -> dispatcher::McpHttpRequest {
    dispatcher::McpHttpRequest {
        body: row.body.clone().into_bytes(),
        headers: row.headers.clone(),
        method: row.method.to_owned(),
        path: "/mcp".to_owned(),
    }
}

fn held_row() -> HttpRow {
    posted(
        tool_call(
            1,
            "export-folder",
            &json!({ "value": "held" }),
            Some("export-1"),
        ),
        "export-folder",
    )
}

/// A request with a token whose handler is held: what the stream has received while it is held,
/// and when it ends.
fn streamed_progress() -> Value {
    let stream = Recorded::default();
    let gate = Arc::new(Gate::default());
    let asked = streamed_request(&held_row());
    let returned = Mutex::new(None);
    let mut tasks = Tasks::default();
    tasks.spawn(async {
        let answered = dispatcher::answer_http(
            &asked,
            &STREAM_SETTINGS,
            &stream,
            &Stand,
            async |_request: &dispatcher::McpHttpRequest, call: dispatcher::McpCall| {
                Ok::<_, dispatcher::McpDenied>(Reporting::gated(&call, Arc::clone(&gate)))
            },
        )
        .await;
        *returned.lock().unwrap() = Some(answered.is_some());
    });
    tasks.run_until_idle();
    let held = stream.report();
    let ended_while_held = returned.lock().unwrap().is_some();
    gate.open();
    tasks.run_until_idle();
    let answered = returned.lock().unwrap().unwrap();
    json!({
        "endedWhileHeld": ended_while_held,
        "full": stream.report(),
        "held": held,
        "returned": answered,
    })
}

/// A request with a token whose stream goes away after it began: the first event is refused.
fn streamed_failure() -> Value {
    let stream = Recorded::gone_after(1);
    let gate = Arc::new(Gate::default());
    let asked = streamed_request(&held_row());
    let seen_call = Mutex::new(None);
    let returned = Mutex::new(None);
    let mut tasks = Tasks::default();
    tasks.spawn(async {
        let answered = dispatcher::answer_http(
            &asked,
            &STREAM_SETTINGS,
            &stream,
            &Stand,
            async |_request: &dispatcher::McpHttpRequest, call: dispatcher::McpCall| {
                *seen_call.lock().unwrap() = Some(call.clone());
                Ok::<_, dispatcher::McpDenied>(Reporting::gated(&call, Arc::clone(&gate)))
            },
        )
        .await;
        *returned.lock().unwrap() = Some(answered.is_some());
    });
    tasks.run_until_idle();
    gate.open();
    tasks.run_until_idle();
    let cancelled = seen_call.lock().unwrap().as_ref().unwrap().cancelled();
    let answered = returned.lock().unwrap().unwrap();
    json!({ "cancelled": cancelled, "returned": answered, "seen": stream.report() })
}

/// A request with a token whose stream cannot begin: its handler is never reached.
fn streamed_refusal() -> Value {
    let stream = Recorded::gone_after(0);
    let seen_call = Mutex::new(None);
    let reached = Arc::new(AtomicUsize::new(0));
    let answered = ready_output(dispatcher::answer_http(
        &streamed_request(&http_rows()[0]),
        &STREAM_SETTINGS,
        &stream,
        &Stand,
        async |_request: &dispatcher::McpHttpRequest, call: dispatcher::McpCall| {
            *seen_call.lock().unwrap() = Some(call);
            let counter = Arc::clone(&reached);
            Ok::<_, dispatcher::McpDenied>(Reporting {
                cancelled: Box::new(|| false),
                gate: Arc::new(Gate::default()),
                progress: Box::new(move |_, _, _| {
                    counter.fetch_add(1, Ordering::SeqCst);
                }),
            })
        },
    ));
    let cancelled = seen_call.lock().unwrap().as_ref().unwrap().cancelled();
    json!({
        "cancelled": cancelled,
        "reached": reached.load(Ordering::SeqCst),
        "returned": answered.is_some(),
        "seen": stream.report(),
    })
}

/// A request without a token: one JSON object is returned and the stream is never begun.
fn streamed_plain() -> Value {
    let stream = Recorded::default();
    let answered = ready_output(dispatcher::answer_http(
        &streamed_request(&http_rows()[1]),
        &STREAM_SETTINGS,
        &stream,
        &Stand,
        async |_request: &dispatcher::McpHttpRequest, call: dispatcher::McpCall| {
            Ok::<_, dispatcher::McpDenied>(Reporting::of(&call))
        },
    ))
    .unwrap();
    json!({ "returned": true, "seen": stream.report(), "status": answered.status })
}

/// What an HTTP host sees of a request that answers an event stream while its handler is held, and
/// of the three ways a stream and a request can end otherwise.
fn streams_report() -> Value {
    json!({
        "failed": streamed_failure(),
        "plain": streamed_plain(),
        "progress": streamed_progress(),
        "refused": streamed_refusal(),
    })
}

/// What the Rust dispatcher writes for the rows above, in the shape each language reports.
pub fn rust_report() -> Value {
    json!({
        "streams": streams_report(),
        "failed": failed_write(),
        "http": http_rows().iter().map(http_answer).collect::<Vec<_>>(),
        "written": stdio_written(),
    })
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

fn parsed(text: &str) -> Value {
    serde_json::from_str(text).unwrap()
}

fn read(text: &str) -> Value {
    floats(&parsed(text))
}

fn without_detail(mut message: Value) -> Value {
    if let Some(text) = message["result"]["content"][0]["text"].as_str() {
        let mut carried = parsed(text);
        if let Some(fault) = carried["fault"].as_object_mut() {
            fault.remove("detail");
        }
        message["result"]["content"][0]["text"] = Value::String(carried.to_string());
    }
    message
}

fn event_texts(body: &str) -> Vec<&str> {
    body.split_terminator("\n\n")
        .map(|event| event.strip_prefix("event: message\ndata: ").unwrap())
        .collect()
}

fn events_of(body: &str) -> Vec<Value> {
    event_texts(body).into_iter().map(read).collect()
}

fn raw_events_of(body: &str) -> Vec<Value> {
    event_texts(body).into_iter().map(parsed).collect()
}

/// `scenario` with every text a stream received read as the JSON message it carries.
fn read_streamed(scenario: &Value) -> Value {
    let mut read_all = scenario.clone();
    for key in ["held", "full", "seen"] {
        if let Some(entries) = scenario.get(key).and_then(Value::as_array) {
            read_all[key] = Value::Array(
                entries
                    .iter()
                    .map(|entry| match entry.get("wrote").and_then(Value::as_str) {
                        Some(":\n\n") => json!({ "wrote": ":" }),
                        Some(text) => json!({ "wrote": read(
                            text.strip_prefix("event: message\ndata: ")
                                .and_then(|rest| rest.strip_suffix("\n\n"))
                                .unwrap(),
                        ) }),
                        None => entry.clone(),
                    })
                    .collect(),
            );
        }
    }
    read_all
}

/// Holds what a language's stream received to what the Rust stream received, and every message
/// to the official schema.
fn assert_streams(report: &Value, rust: &Value) {
    for scenario in ["progress", "failed", "refused", "plain"] {
        assert_eq!(
            read_streamed(&report["streams"][scenario]),
            read_streamed(&rust["streams"][scenario]),
            "{scenario}: {report:#}"
        );
    }
    let progress = &report["streams"]["progress"];
    assert_eq!(progress["endedWhileHeld"], false, "{report:#}");
    assert_eq!(progress["returned"], false, "{report:#}");
    assert_eq!(progress["held"].as_array().unwrap().len(), 2, "{report:#}");
    assert_eq!(progress["full"].as_array().unwrap().len(), 3, "{report:#}");
    assert_eq!(progress["held"][0]["began"][0], 200_u64, "{report:#}");
    assert_eq!(
        progress["held"][0]["began"][1],
        json!([
            ["Content-Type", "text/event-stream"],
            ["X-Accel-Buffering", "no"]
        ]),
        "{report:#}"
    );
    for (at, definition) in [(1, "ProgressNotification"), (2, "JSONRPCResultResponse")] {
        let text = progress["full"][at]["wrote"].as_str().unwrap();
        held_to(
            definition,
            &parsed(
                text.strip_prefix("event: message\ndata: ")
                    .and_then(|rest| rest.strip_suffix("\n\n"))
                    .unwrap(),
            ),
        );
    }
    let failed = &report["streams"]["failed"];
    assert_eq!(failed["cancelled"], true, "{report:#}");
    assert_eq!(failed["returned"], false, "{report:#}");
    assert_eq!(failed["seen"].as_array().unwrap().len(), 1, "{report:#}");
    let refused = &report["streams"]["refused"];
    assert_eq!(refused["cancelled"], true, "{report:#}");
    assert_eq!(refused["reached"], 0_u64, "{report:#}");
    assert_eq!(refused["returned"], false, "{report:#}");
    assert_eq!(refused["seen"], json!([]), "{report:#}");
    let plain = &report["streams"]["plain"];
    assert_eq!(plain["returned"], true, "{report:#}");
    assert_eq!(plain["status"], 200_u64, "{report:#}");
    assert_eq!(plain["seen"], json!([]), "{report:#}");
}

/// Holds the report of a language's program to what the Rust dispatcher wrote, and every message
/// to the official schema.
pub fn assert_matches(report: &Value) {
    let rust = rust_report();
    assert_streams(report, &rust);
    assert_eq!(report["observed"], true, "{report:#}");
    let written: Vec<&str> = report["written"]
        .as_array()
        .unwrap()
        .iter()
        .map(|text| text.as_str().unwrap())
        .collect();
    assert_eq!(written.len(), 2, "{report:#}");
    let expected = rust["written"].as_array().unwrap();
    assert_eq!(expected.len(), 2, "{rust:#}");
    for (text, rust_text) in written.iter().zip(expected) {
        assert_eq!(read(text), read(rust_text.as_str().unwrap()), "{report:#}");
    }
    held_to("ProgressNotification", &parsed(written[0]));
    held_to("JSONRPCResultResponse", &parsed(written[1]));
    held_to("CancelledNotification", &parsed(&lines()[2]));
    assert_eq!(report["failed"], rust["failed"], "{report:#}");
    let http = report["http"].as_array().unwrap();
    let expected_http = rust["http"].as_array().unwrap();
    assert_eq!(http.len(), expected_http.len(), "{report:#}");
    for (at, (answered, rust_answered)) in http.iter().zip(expected_http).enumerate() {
        let worded = |text: &str| {
            if at == WORDED_BY_THE_LANGUAGE {
                without_detail(read(text))
            } else {
                read(text)
            }
        };
        assert_eq!(answered["status"], rust_answered["status"], "{answered:#}");
        assert_eq!(
            answered["headers"], rust_answered["headers"],
            "{answered:#}"
        );
        let streamed = answered["headers"][0][1] == "text/event-stream";
        let (body, rust_body) = (
            answered["body"].as_str().unwrap(),
            rust_answered["body"].as_str().unwrap(),
        );
        if streamed {
            let events = |text| {
                event_texts(text)
                    .into_iter()
                    .map(worded)
                    .collect::<Vec<_>>()
            };
            assert_eq!(events(body), events(rust_body), "{answered:#}");
            for event in &raw_events_of(body) {
                held_to(
                    if event["method"] == "notifications/progress" {
                        "ProgressNotification"
                    } else if event.get("error").is_some() {
                        "JSONRPCErrorResponse"
                    } else {
                        "JSONRPCResultResponse"
                    },
                    event,
                );
            }
        } else {
            assert_eq!(worded(body), worded(rust_body), "{answered:#}");
        }
    }
    let kinds: Vec<(u64, bool)> = http
        .iter()
        .map(|answered| {
            (
                answered["status"].as_u64().unwrap(),
                answered["headers"][0][1] == "text/event-stream",
            )
        })
        .collect();
    assert_eq!(
        kinds,
        [
            (200, true),
            (200, false),
            (200, true),
            (200, true),
            (200, true)
        ],
        "{report:#}"
    );
    assert_eq!(events_of(http[0]["body"].as_str().unwrap()).len(), 2);
    assert_eq!(events_of(http[2]["body"].as_str().unwrap()).len(), 1);
}

#[test]
fn the_rust_dispatcher_answers_alone_beside_an_endpoint_and_over_http_alike() {
    let (alone, served, status) =
        rust_serves_alike!(dispatcher, WriterServiceSchema, Stand, ready_output);
    assert_eq!(alone, served);
    assert!(alone.is_some());
    assert_eq!(status, 200);
}

#[test]
fn an_admit_that_denies_by_name_stops_the_request_before_any_handler_runs() {
    let settings = dispatcher::McpHttpSettings {
        allowed_origins: &[],
        oauth: None,
    };
    let base = &http_rows()[0];
    for (name, status) in [("forbidden", 403_u16), ("scoped", 403), ("anonymous", 401)] {
        let mut headers = base.headers.clone();
        headers[2].1 = name.to_owned();
        let asked = dispatcher::McpHttpRequest {
            body: base.body.clone().into_bytes(),
            headers,
            method: base.method.to_owned(),
            path: "/mcp".to_owned(),
        };
        let response = through(|stream| {
            ready_output(dispatcher::answer_http(
                &asked,
                &settings,
                stream,
                &Stand,
                async |request: &dispatcher::McpHttpRequest, _call: dispatcher::McpCall| {
                    match request.mcp_name().as_deref() {
                        Some("forbidden") => Err::<(), _>(dispatcher::McpDenied::Forbidden),
                        Some("scoped") => Err(dispatcher::McpDenied::InsufficientScope {
                            scope: "exports:write".to_owned(),
                        }),
                        _ => Err(dispatcher::McpDenied::NoValidCredentials { scope: None }),
                    }
                },
            ))
        });
        assert_eq!(response.status, status, "{name}");
        assert!(response.body.is_empty(), "{name}");
    }
}

#[test]
fn the_rust_dispatcher_listens_alike_through_every_entry_point() {
    let (alone, served) = rust_listens_alike!(dispatcher, WriterServiceSchema, Stand, ready_output);
    assert_eq!(alone, served);
    assert!(alone.is_some());
}
