//! The stand-in executor, the messages a client writes, and the check of every message against
//! the official schema.

use crate::tests::Archive;
use crate::{
    BillingServiceSchema, DocumentServiceSchema, approval_mcp, billing_mcp, document_mcp,
    export_mcp,
};
use alloc::sync::Arc;
use alloc::task::Wake;
use core::future::{Future, ready};
use core::pin::{Pin, pin};
use core::sync::atomic::{AtomicBool, Ordering};
use core::task::{Context, Poll, Waker};
use serde_json::{Value, json};
use std::sync::Mutex;

macro_rules! writes_to {
    ($($module:ident),*) => {$(
        impl $module::McpEventWriter for Lines {
            fn write(
                &self,
                message: String,
            ) -> impl Future<Output = Result<(), $module::McpClientGone>> {
                ready(self.keep(message).then_some(()).ok_or($module::McpClientGone))
            }
        }
    )*};
}

/// The official 2026-07-28 schema of the Model Context Protocol.
const MCP_SCHEMA: &str = include_str!("../mcp_declaration_tests/mcp-schema-2026-07-28.json");

pub const CENTS: i64 = 1250;
pub const HEADER_MISMATCH: i64 = -32020;
pub const INTERNAL_ERROR: i64 = -32603;
pub const INVALID_PARAMS: i64 = -32602;
pub const INVALID_REQUEST: i64 = -32600;
pub const LISTED_FOR_MS: u64 = 300_000;
pub const METHOD_NOT_FOUND: i64 = -32601;
pub const UNSUPPORTED_VERSION: i64 = -32022;
pub const VERSION: &str = "2026-07-28";
pub const VERSION_NUMBER: u32 = 7;

/// Two services behind one endpoint, built in a `const` so a shared name would stop the build.
pub const WORKSPACE: document_mcp::McpEndpoint = document_mcp::McpEndpoint::new(
    &[
        DocumentServiceSchema::MCP_NAMES,
        BillingServiceSchema::MCP_NAMES,
    ],
    &[document_mcp::catalog, billing_mcp::catalog],
);

/// Where a stdio host writes: every message kept, or every write refused once the client is gone.
#[derive(Default)]
pub struct Lines {
    gone: bool,
    written: Mutex<Vec<String>>,
}

impl Lines {
    pub fn gone() -> Self {
        Self {
            gone: true,
            written: Mutex::new(Vec::new()),
        }
    }

    fn keep(&self, message: String) -> bool {
        if self.gone {
            return false;
        }
        self.written.lock().unwrap().push(message);
        true
    }

    pub fn written(&self) -> Vec<String> {
        self.written.lock().unwrap().clone()
    }
}

writes_to!(document_mcp, billing_mcp, export_mcp, approval_mcp);

type Task<'task> = Pin<Box<dyn Future<Output = ()> + 'task>>;

/// Tasks polled in turn, each only after something woke it, with no clock anywhere.
#[derive(Default)]
pub struct Tasks<'tasks> {
    running: Vec<(Arc<Woken>, Task<'tasks>)>,
}

struct Woken(AtomicBool);

impl Wake for Woken {
    fn wake(self: Arc<Self>) {
        self.0.store(true, Ordering::SeqCst);
    }
}

impl<'tasks> Tasks<'tasks> {
    /// Polls every task that was woken until none is, and drops the ones that finished.
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
/// The identifier of a request.
pub fn id(number: u64) -> Value {
    Value::from(number)
}

/// What `answering` answers on its first poll: every handler here is ready at once, and a future
/// that is not has nothing to wake it.
pub fn block_on<Answering>(answering: Answering) -> Answering::Output
where
    Answering: Future,
{
    let mut pinned = pin!(answering);
    let polled = pinned
        .as_mut()
        .poll(&mut Context::from_waker(Waker::noop()));
    match polled {
        Poll::Ready(output) => Some(output),
        Poll::Pending => None,
    }
    .unwrap()
}

/// Every error the vendored definition `definition` finds in `instance`, each with where it is.
pub fn schema_errors(definition: &str, instance: &Value) -> Vec<String> {
    let schema: Value = serde_json::from_str(MCP_SCHEMA).unwrap();
    let rooted = json!({ "$ref": format!("#/$defs/{definition}"), "$defs": schema["$defs"] });
    let validator = jsonschema::validator_for(&rooted).unwrap();
    validator
        .iter_errors(instance)
        .map(|error| format!("{error} at {}", error.instance_path()))
        .collect()
}

pub fn assert_valid(definition: &str, instance: &Value) {
    let errors = schema_errors(definition, instance);
    assert_eq!(errors, Vec::<String>::new(), "{definition}: {instance:#}");
}

/// An answer that holds a `result` of the definition `result`, and passes as a response.
pub fn assert_result(result: &str, response: &Value) {
    assert_valid("JSONRPCResultResponse", response);
    assert_valid(result, &response["result"]);
}

/// An answer that holds an `error`, and passes as the response to a request.
pub fn assert_error(definition: &str, response: &Value) {
    assert_valid("JSONRPCErrorResponse", response);
    assert_valid(definition, response);
}

/// The `_meta` a request of the latest revision carries.
pub fn meta() -> Value {
    json!({
        "io.modelcontextprotocol/protocolVersion": VERSION,
        "io.modelcontextprotocol/clientCapabilities": {},
    })
}

/// The `_meta` of a result of the server called `name`.
pub fn server_info(name: &str) -> Value {
    json!({
        "io.modelcontextprotocol/serverInfo": {
            "name": name,
            "version": env!("CARGO_PKG_VERSION"),
        },
    })
}

/// A request, with the `_meta` a request carries added to its `params`.
pub fn request(request_id: &Value, method: &str, params: Value) -> Value {
    let mut carried = params;
    carried["_meta"] = meta();
    json!({ "jsonrpc": "2.0", "id": request_id, "method": method, "params": carried })
}

pub fn call(request_id: &Value, tool: &str, arguments: &Value) -> Value {
    let asked = request(
        request_id,
        "tools/call",
        json!({ "name": tool, "arguments": arguments }),
    );
    assert_valid("CallToolRequest", &asked);
    asked
}

/// The answer of `DocumentService` alone to `asked`.
pub fn answer(archive: &Archive, asked: &Value) -> Option<Value> {
    block_on(document_mcp::answer(
        &asked.to_string(),
        archive,
        |_call| (),
        &Lines::default(),
        &document_mcp::McpInFlight::default(),
    ))
    .map(|text| serde_json::from_str(&text).unwrap())
}

/// The answer of the endpoint that holds both services to `asked`.
pub fn answer_workspace(archive: &Archive, asked: &Value) -> Option<Value> {
    block_on(document_mcp::serve(
        &asked.to_string(),
        "Workspace",
        None,
        &WORKSPACE,
        |_call| (),
        document_mcp::McpHost {
            in_flight: &document_mcp::McpInFlight::default(),
            writer: &Lines::default(),
        },
        &|ctx: &(), part, method: &str, params: &Value| {
            let held = archive.clone();
            Box::pin(async move {
                match part {
                    0 => document_mcp::route(method, params, &held, ctx).await,
                    1 => billing_mcp::route(method, params, &held, ctx).await,
                    _ => None,
                }
            })
        },
    ))
    .map(|text| serde_json::from_str(&text).unwrap())
}
