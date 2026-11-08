//! The stand-in executor, the messages a client writes, and the check of every message against
//! the official schema.

use crate::tests::Archive;
use crate::{BillingServiceSchema, DocumentServiceSchema, billing_mcp, document_mcp};
use core::future::Future;
use core::pin::pin;
use core::task::{Context, Poll, Waker};
use serde_json::{Value, json};

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
    block_on(document_mcp::answer(&asked.to_string(), archive, &()))
        .map(|text| serde_json::from_str(&text).unwrap())
}

/// The answer of the endpoint that holds both services to `asked`.
pub fn answer_workspace(archive: &Archive, asked: &Value) -> Option<Value> {
    block_on(document_mcp::serve(
        &asked.to_string(),
        "Workspace",
        None,
        &WORKSPACE,
        async |part, method, params| match part {
            0 => document_mcp::route(method, params, archive, &()).await,
            1 => billing_mcp::route(method, params, archive, &()).await,
            _ => None,
        },
    ))
    .map(|text| serde_json::from_str(&text).unwrap())
}
