//! `GuideService`, whose operations are prompts and the completion of a prompt's argument, the
//! Rust dispatcher that answers it, and the rows the TypeScript, Dart and Kotlin servers are
//! measured against and the clients are read against. Nothing here waits on a clock.

use super::mcp_guide_dispatcher as dispatcher;
use crate::mcp_streams::through;
use alloc::collections::BTreeMap;
use alloc::sync::Arc;
use core::future::{Future, ready};
use core::pin::pin;
use core::task::{Context, Poll, Waker};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::HashMap;
use std::panic::resume_unwind;
use std::sync::Mutex;
use tixschema::{model_schema, service_schema};

/// The official 2026-07-28 schema of the Model Context Protocol.
const MCP_SCHEMA: &str = include_str!("../mcp_declaration_tests/mcp-schema-2026-07-28.json");

const EVENT_PREFIX: &str = "event: message\ndata: ";

/// The requests whose validation fault is compared without its detail, which each language words
/// in its own terms.
const FAULTED_ANSWERS: [&str; 2] = ["get_missing_argument", "get_wrong_type"];

/// The method and name of each scope query every server answers.
pub const QUERIES: [(&str, Option<&str>); 6] = [
    ("prompts/get", Some("greet")),
    ("prompts/get", Some("review-document-type")),
    ("prompts/get", Some("nothing")),
    ("completion/complete", Some("review-document-type")),
    ("completion/complete", Some("nothing")),
    ("tools/list", None),
];

#[model_schema(enable = ["jsonschema"])]
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case", tag = "errorCode")]
pub enum GuideError {
    NotFound,
}

#[model_schema(enable = ["jsonschema"])]
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct GuideVersion {
    pub version: u32,
}

/// Guides the model through the setup of an organization.
#[service_schema(transports = ["mcp"])]
pub trait GuideService<Ctx> {
    /// Guides the model through the configuration of one organization.
    #[service_schema_op(mcp(prompt))]
    async fn configure_organization(&self, ctx: &Ctx) -> Result<String, GuideError>;

    /// Answers the current version of the guide.
    async fn get_guide_version(&self, ctx: &Ctx) -> Result<GuideVersion, GuideError>;

    /// Greets one person.
    #[service_schema_op(mcp(prompt, title = "A Greeting", icon = "https://example.com/hi.png", scopes = ["prompts:read"]))]
    async fn greet(&self, ctx: &Ctx, name: String) -> Result<String, GuideError>;

    /// Reviews one document type against the naming guide.
    #[service_schema_op(mcp(prompt))]
    async fn review_document_type(
        &self,
        ctx: &Ctx,
        document_type: String,
        focus: Option<String>,
    ) -> Result<Vec<guide_service_schema::McpPromptMessage>, GuideError>;

    /// Suggests the document types that start with what the user typed.
    #[service_schema_op(mcp(completes(prompt = "review_document_type", parameter = "document_type"), scopes = ["prompts:read"]))]
    async fn suggest_document_types(
        &self,
        ctx: &Ctx,
        typed: String,
        chosen: HashMap<String, String>,
    ) -> Result<Vec<String>, GuideError>;
}

/// Answers every operation, and writes down what reached it.
#[derive(Clone, Default)]
pub struct Stand {
    pub reached: Arc<Mutex<Vec<Value>>>,
}

impl Stand {
    fn reach(&self, call: Value) {
        self.reached.lock().unwrap().push(call);
    }
}

// Every method below is synchronous work wrapped in an already-ready `Future`: nothing here waits
// on anything.
impl GuideService<()> for Stand {
    fn configure_organization(
        &self,
        _ctx: &(),
    ) -> impl Future<Output = Result<String, GuideError>> {
        self.reach(json!(["configure-organization"]));
        ready(Ok("Apply the entity types in this order.".to_owned()))
    }

    fn get_guide_version(
        &self,
        _ctx: &(),
    ) -> impl Future<Output = Result<GuideVersion, GuideError>> {
        self.reach(json!(["get-guide-version"]));
        ready(Ok(GuideVersion { version: 3 }))
    }

    fn greet(&self, _ctx: &(), name: String) -> impl Future<Output = Result<String, GuideError>> {
        self.reach(json!(["greet", name]));
        if name == "boom" {
            resume_unwind(Box::new("the handler came apart"));
        }
        ready(if name == "missing" {
            Err(GuideError::NotFound)
        } else {
            Ok(format!("Say hello to {name}."))
        })
    }

    fn review_document_type(
        &self,
        _ctx: &(),
        document_type: String,
        focus: Option<String>,
    ) -> impl Future<Output = Result<Vec<guide_service_schema::McpPromptMessage>, GuideError>> {
        use guide_service_schema::{McpContent, McpPromptMessage, McpResourceContents, McpRole};
        self.reach(json!(["review-document-type", document_type, focus]));
        ready(Ok(vec![
            McpPromptMessage {
                role: McpRole::User,
                content: McpContent::Text {
                    text: format!("Review the document type {document_type} against this guide."),
                },
            },
            McpPromptMessage {
                role: McpRole::User,
                content: McpContent::Resource {
                    uri: "guide://naming".to_owned(),
                    mime_type: Some("text/markdown".to_owned()),
                    contents: McpResourceContents::Text("# Naming".to_owned()),
                },
            },
            McpPromptMessage {
                role: McpRole::Assistant,
                content: McpContent::Image {
                    data: vec![0x89, b'P', b'N', b'G'],
                    mime_type: "image/png".to_owned(),
                },
            },
            McpPromptMessage {
                role: McpRole::Assistant,
                content: McpContent::ResourceLink {
                    uri: "guide://naming".to_owned(),
                    name: "naming".to_owned(),
                    title: Some("Naming".to_owned()),
                    description: None,
                    mime_type: Some("text/markdown".to_owned()),
                },
            },
        ]))
    }

    fn suggest_document_types(
        &self,
        _ctx: &(),
        typed: String,
        chosen: HashMap<String, String>,
    ) -> impl Future<Output = Result<Vec<String>, GuideError>> {
        let mut known: Vec<_> = chosen.into_iter().collect();
        known.sort();
        self.reach(json!(["suggest-document-types", typed, known]));
        ready(Ok(["ID card", "Invoice", "Passport"]
            .into_iter()
            .filter(|candidate| candidate.to_lowercase().starts_with(&typed.to_lowercase()))
            .map(str::to_owned)
            .collect()))
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
    pub prompt: &'static str,
}

fn request(id: u64, method: &str, params: Value) -> String {
    let mut carried = params;
    carried["_meta"] = json!({
        "io.modelcontextprotocol/protocolVersion": "2026-07-28",
        "io.modelcontextprotocol/clientCapabilities": {},
    });
    json!({ "jsonrpc": "2.0", "id": id, "method": method, "params": carried }).to_string()
}

fn get(prompt: &str, arguments: &Value) -> String {
    request(
        3,
        "prompts/get",
        json!({ "name": prompt, "arguments": arguments }),
    )
}

fn complete(prompt: &str, argument: &str, typed: &str) -> String {
    request(
        4,
        "completion/complete",
        json!({
            "ref": { "type": "ref/prompt", "name": prompt },
            "argument": { "name": argument, "value": typed },
            "context": { "arguments": { "focus": "names" } },
        }),
    )
}

/// The requests every server is sent, by name, in the order they are answered.
pub fn requests() -> Vec<(&'static str, String)> {
    vec![
        ("discover", request(1, "server/discover", json!({}))),
        ("prompts_list", request(2, "prompts/list", json!({}))),
        ("tools_list", request(2, "tools/list", json!({}))),
        (
            "tool_version",
            request(
                5,
                "tools/call",
                json!({ "name": "get-guide-version", "arguments": {} }),
            ),
        ),
        ("get_string", get("configure-organization", &json!({}))),
        (
            "get_no_arguments",
            request(
                3,
                "prompts/get",
                json!({ "name": "configure-organization" }),
            ),
        ),
        ("get_named", get("greet", &json!({ "name": "Ana" }))),
        (
            "get_messages",
            get(
                "review-document-type",
                &json!({ "documentType": "ID card" }),
            ),
        ),
        (
            "get_messages_focus",
            get(
                "review-document-type",
                &json!({ "documentType": "ID card", "focus": "names" }),
            ),
        ),
        ("get_declared", get("greet", &json!({ "name": "missing" }))),
        (
            "get_missing_argument",
            get("review-document-type", &json!({})),
        ),
        (
            "get_wrong_type",
            get("review-document-type", &json!({ "documentType": 5_u8 })),
        ),
        ("get_unknown", get("nothing", &json!({}))),
        ("get_panic", get("greet", &json!({ "name": "boom" }))),
        (
            "get_nameless",
            request(3, "prompts/get", json!({ "arguments": {} })),
        ),
        (
            "complete_argument",
            complete("review-document-type", "documentType", "i"),
        ),
        (
            "complete_other_argument",
            complete("review-document-type", "focus", "i"),
        ),
        (
            "complete_unknown_prompt",
            complete("nothing", "documentType", "i"),
        ),
    ]
}

/// The HTTP requests every server is sent: `prompts/get` with the right `Mcp-Name`, a wrong one,
/// and none.
pub fn http_rows() -> Vec<HttpRow> {
    let body = get("greet", &json!({ "name": "Ana" }));
    let headers = |named: Option<&str>| {
        let mut held = vec![
            ("Content-Type".to_owned(), "application/json".to_owned()),
            ("MCP-Protocol-Version".to_owned(), "2026-07-28".to_owned()),
            ("Mcp-Method".to_owned(), "prompts/get".to_owned()),
        ];
        if let Some(written) = named {
            held.push(("Mcp-Name".to_owned(), written.to_owned()));
        }
        held
    };
    let mut tokened: Value = serde_json::from_str(&body).unwrap();
    tokened["params"]["_meta"]["progressToken"] = json!("greet-1");
    vec![
        HttpRow {
            body: body.clone(),
            headers: headers(Some("greet")),
            name: "http_matching",
        },
        HttpRow {
            body: tokened.to_string(),
            headers: headers(Some("greet")),
            name: "http_streamed",
        },
        HttpRow {
            body: body.clone(),
            headers: headers(Some("another")),
            name: "http_mismatched",
        },
        HttpRow {
            body,
            headers: headers(None),
            name: "http_nameless",
        },
    ]
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
    let settings = dispatcher::McpHttpSettings::new(&[], None);
    let asked = dispatcher::McpHttpRequest::new(
        "POST".to_owned(),
        "/mcp".to_owned(),
        headers.to_vec(),
        body.as_bytes().to_vec(),
    );
    let response = through(|stream| {
        ready_output(dispatcher::answer_http(
            &asked,
            &settings,
            stream,
            &Stand::default(),
            async |_request, _call| Ok(()),
        ))
    });
    json!({ "status": response.status(), "body": String::from_utf8(response.body().to_vec()).unwrap() })
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
        .map(|(method, name)| GuideServiceSchema::mcp_scopes(method, *name).to_vec())
        .collect();
    json!({ "answers": answers, "reached": reached, "http": http, "scopes": scopes })
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
    json!({ "requests": requests, "http": http, "queries": queries })
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
        "prompts_list" => Some("ListPromptsResult"),
        "tools_list" => Some("ListToolsResult"),
        "tool_version" => Some("CallToolResult"),
        "complete_argument" => Some("CompleteResult"),
        _ if name.starts_with("get_") => Some("GetPromptResult"),
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

/// Holds the report of a language's server program to what the Rust dispatcher answered, and
/// every answer to the official schema.
pub fn assert_matches(report: &Value) {
    let rust = rust_report();
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
        }
        assert_eq!(
            report["reached"][name], rust["reached"][name],
            "{name} reached"
        );
    }
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
    let streamed = rust["http"]["http_streamed"]["body"].as_str().unwrap();
    assert!(streamed.starts_with(EVENT_PREFIX), "{streamed}");
    assert!(
        !rust["http"]["http_matching"]["body"]
            .as_str()
            .unwrap()
            .starts_with(EVENT_PREFIX)
    );
    assert_eq!(rust["http"]["http_streamed"]["status"], json!(200_u16));
    assert_eq!(rust["http"]["http_matching"]["status"], json!(200_u16));
    assert_eq!(rust["http"]["http_mismatched"]["status"], json!(400_u16));
    assert_eq!(rust["http"]["http_nameless"]["status"], json!(400_u16));
    assert_eq!(report["scopes"], rust["scopes"], "scopes");
    assert_eq!(rust["scopes"][0], json!(["prompts:read"]));
    assert_eq!(rust["scopes"][1], json!([]));
    assert_eq!(rust["scopes"][3], json!(["prompts:read"]));
}

fn client_calls() -> Vec<ClientCall> {
    let messages = read(
        &answered(&get(
            "review-document-type",
            &json!({ "documentType": "ID card" }),
        ))
        .0,
    )["result"]["messages"]
        .clone();
    vec![
        ClientCall {
            answer: "get_string",
            arguments: json!({}),
            expected: json!({ "ok": "Apply the entity types in this order." }),
            label: "string",
            prompt: "configure-organization",
        },
        ClientCall {
            answer: "get_named",
            arguments: json!({ "name": "Ana" }),
            expected: json!({ "ok": "Say hello to Ana." }),
            label: "named",
            prompt: "greet",
        },
        ClientCall {
            answer: "get_messages",
            arguments: json!({ "documentType": "ID card" }),
            expected: json!({ "ok": messages }),
            label: "messages",
            prompt: "review-document-type",
        },
        ClientCall {
            answer: "get_declared",
            arguments: json!({ "name": "missing" }),
            expected: json!({ "declared": { "errorCode": "not-found" } }),
            label: "declared",
            prompt: "greet",
        },
        ClientCall {
            answer: "get_panic",
            arguments: json!({ "name": "boom" }),
            expected: json!({ "fault": "handler-panic" }),
            label: "panicked",
            prompt: "greet",
        },
        ClientCall {
            answer: "get_unknown",
            arguments: json!({ "name": "nobody" }),
            expected: json!({ "fault": "unknown-operation" }),
            label: "unknown",
            prompt: "greet",
        },
        ClientCall {
            answer: "get_missing_argument",
            arguments: json!({ "documentType": "ID card", "focus": "names" }),
            expected: json!({ "fault": "undeserializable-payload" }),
            label: "unreadable",
            prompt: "review-document-type",
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

/// Holds the outcomes a client read, and the requests it sent, to the calls above.
pub fn assert_client(report: &Value) {
    let calls = client_calls();
    let sent = report["seen"].as_array().unwrap();
    assert_eq!(sent.len(), calls.len(), "{report:#}");
    for (call, request) in calls.iter().zip(sent) {
        held_to("GetPromptRequest", request);
        assert_eq!(request["method"], "prompts/get", "{}", call.label);
        assert_eq!(request["params"]["name"], call.prompt, "{}", call.label);
        assert_eq!(
            request["params"]["arguments"], call.arguments,
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
            floats(&report["outcomes"][call.label]),
            floats(&call.expected),
            "{}",
            call.label
        );
    }
}

#[test]
fn the_rust_dispatcher_answers_alone_beside_an_endpoint_and_over_http_alike() {
    let (alone, served, status) = rust_serves_alike!(
        dispatcher,
        GuideServiceSchema,
        Stand::default(),
        ready_output
    );
    assert_eq!(alone, served);
    assert!(alone.is_some());
    assert_eq!(status, 200);
}

#[test]
fn the_rust_dispatcher_listens_alike_through_every_entry_point() {
    let (alone, served) = rust_listens_alike!(
        dispatcher,
        GuideServiceSchema,
        Stand::default(),
        ready_output
    );
    assert_eq!(alone, served);
    assert!(alone.is_some());
}

#[test]
fn an_admit_that_denies_stops_a_prompt_before_its_handler_runs() {
    let settings = dispatcher::McpHttpSettings::new(&[], None);
    let row = http_rows().remove(0);
    let stand = Stand::default();
    let asked = dispatcher::McpHttpRequest::new(
        "POST".to_owned(),
        "/mcp".to_owned(),
        row.headers,
        row.body.into_bytes(),
    );
    for (denied, status) in [
        (dispatcher::McpDenied::Forbidden, 403_u16),
        (
            dispatcher::McpDenied::InsufficientScope {
                scope: "prompts:read".to_owned(),
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
                    assert_eq!(request.mcp_name().as_deref(), Some("greet"));
                    assert_eq!(request.bearer_token(), None);
                    assert_eq!(request.header("mcp-method"), Some("prompts/get"));
                    Err::<(), _>(denied.clone())
                },
            ))
        });
        assert_eq!(response.status(), status);
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
