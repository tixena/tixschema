//! What the Kotlin and Swift `mcp` runs share: the requests a client is expected to write, the
//! answers the Rust dispatcher gives them, and the check of every message against the official
//! 2026-07-28 schema.

use super::mcp_mobile_document;
use super::mcp_mobile_service::Archive;
use core::future::Future;
use core::pin::pin;
use core::task::{Context, Poll, Waker};
use serde_json::{Value, json};

/// The official 2026-07-28 schema of the Model Context Protocol.
const MCP_SCHEMA: &str = include_str!("../mcp_declaration_tests/mcp-schema-2026-07-28.json");

pub const VERSION: &str = "2026-07-28";

/// The line each call of a run prints, in the order of [`client_calls`].
pub const CLIENT_LINES: [&str; 17] = [
    "ok version=7",
    "declared not-found",
    "fault failed-validation",
    "fault handler-panic",
    "fault transport-failure",
    "fault undeserializable-payload",
    "fault transport-failure",
    "ok version=7",
    "ok version=7 etag=\"v7\"",
    "declared not-found reason=no such document",
    "ok unit",
    "ok unit",
    "ok sent",
    "ok version=9",
    "ok version=10",
    "refused failed-validation",
    "refused transport-failure",
];

/// What the transport of a run does with a request.
pub enum ClientAnswer {
    /// Answers what the Rust dispatcher answers to the request.
    Rust,
    /// Answers this text, whatever the request.
    Text(String),
    /// Throws.
    Throws,
}

/// One call of the run: the tool, the arguments the client writes, and the answer its transport
/// hands back.
pub struct ClientCall {
    pub answer: ClientAnswer,
    pub arguments: Value,
    pub tool: &'static str,
}

/// What `answering` answers on its first poll: every handler here is ready at once.
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

fn schema_errors(definition: &str, instance: &Value) -> Vec<String> {
    let schema: Value = serde_json::from_str(MCP_SCHEMA).unwrap();
    let rooted = json!({ "$ref": format!("#/$defs/{definition}"), "$defs": schema["$defs"] });
    let validator = jsonschema::validator_for(&rooted).unwrap();
    validator
        .iter_errors(instance)
        .map(|error| format!("{error} at {}", error.instance_path()))
        .collect()
}

fn assert_valid(definition: &str, instance: &Value) {
    let errors = schema_errors(definition, instance);
    assert_eq!(errors, Vec::<String>::new(), "{definition}: {instance:#}");
}

/// Holds an answer of the server to `method` to the official schema: the response, and the result
/// or the error inside it.
pub fn assert_answer_valid(method: &str, answer: &str) {
    let parsed: Value = serde_json::from_str(answer).unwrap();
    if parsed.get("error").is_some() {
        assert_valid("JSONRPCErrorResponse", &parsed);
        assert_valid("Error", &parsed["error"]);
        return;
    }
    assert_valid("JSONRPCResultResponse", &parsed);
    let definition = match method {
        "server/discover" => "DiscoverResult",
        "tools/list" => "ListToolsResult",
        "resources/list" => "ListResourcesResult",
        "resources/templates/list" => "ListResourceTemplatesResult",
        _ => "CallToolResult",
    };
    assert_valid(definition, &parsed["result"]);
}

fn meta() -> Value {
    json!({
        "io.modelcontextprotocol/protocolVersion": VERSION,
        "io.modelcontextprotocol/clientCapabilities": {},
    })
}

pub fn request(request_id: &Value, method: &str, params: Value) -> Value {
    let mut carried = params;
    carried["_meta"] = meta();
    json!({ "jsonrpc": "2.0", "id": request_id, "method": method, "params": carried })
}

pub fn call(request_id: u64, tool: &str, arguments: &Value) -> Value {
    let asked = request(
        &Value::from(request_id),
        "tools/call",
        json!({ "name": tool, "arguments": arguments }),
    );
    assert_valid("CallToolRequest", &asked);
    asked
}

/// What the Rust dispatcher of `MobileDocumentService` alone answers to `text`.
pub fn alone_answer(text: &str) -> Option<String> {
    block_on(mcp_mobile_document::answer(text, &Archive, &()))
}

/// The calls of a run, numbered from 1 as a client numbers its requests.
pub fn client_calls() -> Vec<ClientCall> {
    let rust = |tool, arguments| ClientCall {
        answer: ClientAnswer::Rust,
        arguments,
        tool,
    };
    let touching = json!({ "documentId": "doc-1", "reader": "reviewer-7" });
    let fault = json!({
        "fault": { "detail": "the reader is unknown", "kind": "failed-validation", "operation": "touch-document" },
        "isServiceFault": true,
    });
    vec![
        rust("get-version", json!({ "document_id": "doc-1" })),
        rust("get-version", json!({ "document_id": "missing" })),
        rust("get-version", json!({ "document_id": "" })),
        rust("get-version", json!({ "document_id": "panic" })),
        ClientCall {
            answer: ClientAnswer::Text(
                json!({ "jsonrpc": "2.0", "id": 5_u8, "error": {
                    "code": -32601, "message": "Method not found: tools/call",
                } })
                .to_string(),
            ),
            arguments: json!({ "document_id": "doc-1" }),
            tool: "get-version",
        },
        ClientCall {
            answer: ClientAnswer::Text("not json".to_owned()),
            arguments: json!({ "document_id": "doc-1" }),
            tool: "get-version",
        },
        ClientCall {
            answer: ClientAnswer::Throws,
            arguments: json!({ "document_id": "doc-1" }),
            tool: "get-version",
        },
        rust("lookup-document", json!({ "value": "doc-1" })),
        rust(
            "get-tagged",
            json!({ "value": "doc-1", "if-none-match": "\"v6\"" }),
        ),
        rust("get-tagged", json!({ "value": "missing" })),
        rust(
            "rename-document",
            json!({ "documentId": "doc-1", "title": "New" }),
        ),
        rust(
            "archive-document",
            json!({ "documentId": "doc-1", "reason": "stale", "x-exchange": "reviewer-7" }),
        ),
        rust("touch-document", touching.clone()),
        rust(
            "tag-document",
            json!({
                "value": "t", "x-arguments": "a", "x-message": "m", "x-outcome": "o",
                "x-members": "s",
            }),
        ),
        rust(
            "label-document",
            json!({
                "value": "l", "x-sent": "s", "x-next-id": "n", "x-mcp-source": "m",
                "x-carried": "c",
            }),
        ),
        ClientCall {
            answer: ClientAnswer::Text(
                json!({ "jsonrpc": "2.0", "id": 14_u8, "result": {
                    "resultType": "complete",
                    "content": [{ "type": "text", "text": fault.to_string() }],
                    "isError": true,
                } })
                .to_string(),
            ),
            arguments: touching.clone(),
            tool: "touch-document",
        },
        ClientCall {
            answer: ClientAnswer::Throws,
            arguments: touching,
            tool: "touch-document",
        },
    ]
}

/// The answers the transport of a run gives, by the number of the request: `Some(text)` for a
/// text, `None` for a transport that throws.
pub fn client_replies() -> Vec<(u64, Option<String>)> {
    client_calls()
        .iter()
        .enumerate()
        .map(|(at, called)| {
            let number = u64::try_from(at).unwrap() + 1;
            let asked = call(number, called.tool, &called.arguments);
            let answered = match &called.answer {
                ClientAnswer::Rust => {
                    let text = alone_answer(&asked.to_string()).unwrap();
                    assert_answer_valid("tools/call", &text);
                    Some(text)
                }
                ClientAnswer::Text(text) => Some(text.clone()),
                ClientAnswer::Throws => None,
            };
            (number, answered)
        })
        .collect()
}

/// A request a client wrote, held to the one the run expects it to write: the same message but
/// for the `clientInfo` that names the client.
pub fn assert_client_request(number: u64, sent: &str) {
    let called = &client_calls()[usize::try_from(number).unwrap() - 1];
    let mut written: Value = serde_json::from_str(sent).unwrap();
    let info = written["params"]["_meta"]["io.modelcontextprotocol/clientInfo"].clone();
    assert_eq!(
        info["name"], "MobileDocumentServiceMcpClient",
        "got: {info}"
    );
    assert_eq!(info["version"], env!("CARGO_PKG_VERSION"), "got: {info}");
    written["params"]["_meta"]
        .as_object_mut()
        .unwrap()
        .remove("io.modelcontextprotocol/clientInfo");
    assert_eq!(
        written,
        call(number, called.tool, &called.arguments),
        "request {number} is the message the Rust dispatcher reads"
    );
}

#[cfg(feature = "kotlin")]
/// `text` as it is written between the quotes of a Kotlin string.
pub fn kotlin_text(text: &str) -> String {
    let mut written = String::with_capacity(text.len());
    for letter in text.chars() {
        match letter {
            '\\' => written.push_str("\\\\"),
            '"' => written.push_str("\\\""),
            '$' => written.push_str("\\$"),
            '\n' => written.push_str("\\n"),
            '\r' => written.push_str("\\r"),
            '\t' => written.push_str("\\t"),
            other => written.push(other),
        }
    }
    written
}

#[cfg(feature = "swift")]
/// `text` as it is written between the quotes of a Swift string.
pub fn swift_text(text: &str) -> String {
    let mut written = String::with_capacity(text.len());
    for letter in text.chars() {
        match letter {
            '\\' => written.push_str("\\\\"),
            '"' => written.push_str("\\\""),
            '\n' => written.push_str("\\n"),
            '\r' => written.push_str("\\r"),
            '\t' => written.push_str("\\t"),
            other => written.push(other),
        }
    }
    written
}
