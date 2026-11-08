//! What the TypeScript, Dart and Kotlin `mcp` servers are held to for resources, templates, a
//! listing and a completion: the requests they are sent, as JSON-RPC texts and in plain HTTP terms,
//! the answers the Rust dispatcher gives them, and the check of every answer against the official
//! 2026-07-28 schema.

use super::mcp_bare_dispatcher;
use super::mcp_depot_dispatcher;
use super::mcp_lesson_dispatcher;
use super::mcp_lesson_service::Lessons;
use super::mcp_library_dispatcher::{self, McpEndpoint, McpHttpRequest, McpHttpSettings};
use super::mcp_library_service::{Sink, Stand};
use super::mcp_shelf_dispatcher;
use super::mcp_shelf_service::Shelves;
use super::tests::poll_once;
use super::{
    BareServiceSchema, DepotServiceSchema, LessonServiceSchema, LibraryServiceSchema,
    MirrorService, MirrorServiceSchema, ShelfServiceSchema,
};
use crate::mcp_streams::through;
use serde_json::{Value, json};

macro_rules! exercised {
    ($name:ident, $module:ident, $schema:ident, $stand:expr) => {
        /// Serves the row over HTTP alone and on an endpoint of one, and answers each denial.
        fn $name(row: &HttpRow) {
            let terms = $module::McpHttpRequest::new(
                "POST".to_owned(),
                "/mcp".to_owned(),
                row.headers.clone(),
                row.body.clone().into_bytes(),
            );
            let settings = $module::McpHttpSettings::new(&[ORIGIN], None);
            let endpoint = $module::McpEndpoint::new(&[$schema::MCP_NAMES], &[$module::catalog]);
            let alone = crate::mcp_streams::through(|stream| {
                poll_once($module::answer_http(
                    &terms,
                    &settings,
                    stream,
                    &$stand,
                    async |_request, _call| Ok(()),
                ))
                .unwrap()
            });
            let served = crate::mcp_streams::through(|stream| {
                poll_once($module::serve_http(
                    &terms,
                    &settings,
                    stream,
                    &$module::McpServer::new(&endpoint, None, $module::SERVER_NAME),
                    async |_request, _call| Ok(()),
                    &|ctx: &(), _part, method: &str, params: &Value| {
                        Box::pin(async move { $module::route(method, params, &$stand, ctx).await })
                    },
                ))
                .unwrap()
            });
            let listed = poll_once($module::serve(
                &row.body,
                "Workspace",
                None,
                &endpoint,
                |call| call.progress(0.0_f64, None, None),
                $module::McpHost::new(&$module::McpInFlight::default(), &Sink),
                &|ctx: &(), _part, method: &str, params: &Value| {
                    Box::pin(async move { $module::route(method, params, &$stand, ctx).await })
                },
            ))
            .flatten();
            assert_eq!(listed.is_some(), !alone.body().is_empty());
            assert_eq!(alone.status(), served.status());
            assert_eq!(alone.body(), served.body());
            assert_eq!(
                terms.mcp_name().as_deref(),
                row.headers
                    .iter()
                    .find(|(name, _)| name.eq_ignore_ascii_case("mcp-name"))
                    .map(|(_, value)| value.as_str())
            );
            assert_eq!(terms.header("mcp-method"), Some("resources/read"));
            assert_eq!(terms.bearer_token(), None);
            for (denied, status) in [
                (
                    $module::McpDenied::NoValidCredentials { scope: None },
                    401_u16,
                ),
                (
                    $module::McpDenied::InsufficientScope {
                        scope: "books:read".to_owned(),
                    },
                    403_u16,
                ),
                ($module::McpDenied::Forbidden, 403_u16),
            ] {
                let response = crate::mcp_streams::through(|stream| {
                    poll_once($module::answer_http(
                        &terms,
                        &settings,
                        stream,
                        &$stand,
                        async |_request: &$module::McpHttpRequest,
                               _call: $module::McpCall|
                               -> Result<(), $module::McpDenied> {
                            Err(denied.clone())
                        },
                    ))
                    .unwrap()
                });
                assert_eq!(response.status(), status, "{denied:?}");
            }
        }
    };
}

/// The official 2026-07-28 schema of the Model Context Protocol.
const MCP_SCHEMA: &str = include_str!("../mcp_declaration_tests/mcp-schema-2026-07-28.json");

pub const VERSION: &str = "2026-07-28";
pub const ORIGIN: &str = "https://app.example.com";

const SETTINGS: McpHttpSettings<'static> = McpHttpSettings::new(&[ORIGIN], None);

/// Two services behind one endpoint, built in a `const` so a shared name would stop the build.
const WORKSPACE: McpEndpoint = McpEndpoint::new(
    &[
        LibraryServiceSchema::MCP_NAMES,
        DepotServiceSchema::MCP_NAMES,
    ],
    &[
        mcp_library_dispatcher::catalog,
        mcp_depot_dispatcher::catalog,
    ],
);

/// How the answer of another server is held against the Rust dispatcher's.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Match {
    /// The same bytes.
    Exact,
    /// The same bytes, once the `detail` of the fault in the answer is left out: it is the words
    /// of the library that refused the value.
    WithoutDetail,
}

pub struct Row {
    pub matches: Match,
    pub name: &'static str,
    pub text: String,
}

/// What a plain HTTP request is answered with: the status, the headers and the body.
pub type HttpAnswer = (u16, Vec<(String, String)>, String);

/// One plain HTTP request, handed to each side.
pub struct HttpRow {
    pub body: String,
    pub headers: Vec<(String, String)>,
    pub name: &'static str,
}

fn meta() -> Value {
    json!({
        "io.modelcontextprotocol/protocolVersion": VERSION,
        "io.modelcontextprotocol/clientCapabilities": {},
    })
}

pub fn request(request_id: &Value, method: &str, params: &Value) -> Value {
    let mut carried = params.clone();
    carried["_meta"] = meta();
    json!({ "jsonrpc": "2.0", "id": request_id, "method": method, "params": carried })
}

fn asked(request_id: u64, method: &str, params: &Value) -> String {
    request(&Value::from(request_id), method, params).to_string()
}

fn row(name: &'static str, request_id: u64, method: &str, params: &Value) -> Row {
    Row {
        matches: Match::Exact,
        name,
        text: asked(request_id, method, params),
    }
}

fn row_without_detail(name: &'static str, request_id: u64, method: &str, params: &Value) -> Row {
    Row {
        matches: Match::WithoutDetail,
        ..row(name, request_id, method, params)
    }
}

fn completion(request_id: u64, name: &'static str, argument: &Value, context: &Value) -> Row {
    row(
        name,
        request_id,
        "completion/complete",
        &json!({
            "ref": { "type": "ref/resource", "uri": "library://covers/{book_id}" },
            "argument": argument,
            "context": context,
        }),
    )
}

fn listed(name: &'static str, request_id: u64, cursor: &Value) -> Row {
    row(
        name,
        request_id,
        "resources/list",
        &json!({ "cursor": cursor }),
    )
}

fn read(name: &'static str, request_id: u64, uri: &str) -> Row {
    row(name, request_id, "resources/read", &json!({ "uri": uri }))
}

fn typed(value: &str) -> Value {
    json!({ "name": "book_id", "value": value })
}

/// The requests that ask what the services list.
fn listing_rows() -> Vec<Row> {
    vec![
        row("server/discover", 1, "server/discover", &json!({})),
        row("tools/list", 2, "tools/list", &json!({})),
        row(
            "resource templates",
            3,
            "resources/templates/list",
            &json!({}),
        ),
        row(
            "the first page of resources",
            4,
            "resources/list",
            &json!({}),
        ),
        listed(
            "a cursor that resumes the first service",
            5,
            &json!("0:page-2"),
        ),
        listed(
            "a cursor that moves on to the second service",
            6,
            &json!("1:"),
        ),
        listed("a cursor that writes a plus sign", 7, &json!("+1:")),
        listed("a cursor with leading zeros", 8, &json!("00:page-2")),
        listed("a cursor that names no service", 9, &json!("7:")),
        listed("a cursor with no colon", 10, &json!("x")),
        listed("a cursor that is only a number", 11, &json!("0")),
        listed("a cursor with a negative number", 12, &json!("-1:a")),
        listed("a cursor with no number", 13, &json!(":a")),
        listed("a cursor with a blank number", 14, &json!(" 1:")),
        listed("a cursor that is no text", 15, &json!(4_i32)),
        listed("a cursor that is null", 16, &Value::Null),
        listed("a listing that fails", 17, &json!("0:nowhere")),
        listed("a listing that comes apart", 18, &json!("0:panic")),
    ]
}

/// The requests that read a resource.
fn read_rows() -> Vec<Row> {
    let mut unnamed = request(&Value::from(32_u64), "resources/read", &json!({}));
    unnamed["params"]["uri"] = json!(5_i32);
    vec![
        read("a fixed resource", 19, "guide://library"),
        read("a template that reads bytes", 20, "library://covers/book-1"),
        read(
            "a template whose bytes are not text",
            21,
            "library://covers/book-2",
        ),
        read(
            "a resource that does not exist",
            22,
            "library://covers/missing",
        ),
        read("a read that comes apart", 23, "library://covers/panic"),
        read(
            "a template that sets a lifetime",
            24,
            "depot://crates/crate-1",
        ),
        read("a template that fails", 25, "depot://crates/crate-2"),
        read(
            "a template with two placeholders",
            26,
            "depot://aisles/a1/bins/b7",
        ),
        read("a placeholder that is empty", 27, "library://covers/"),
        read(
            "a placeholder that runs past a slash",
            28,
            "library://covers/book-1/extra",
        ),
        read("a URI that stops short", 29, "library://covers"),
        read(
            "an inner placeholder that is empty",
            30,
            "depot://aisles//bins/b7",
        ),
        read("a URI no service owns", 31, "elsewhere://anything"),
        Row {
            matches: Match::Exact,
            name: "a read whose URI is no text",
            text: unnamed.to_string(),
        },
        row(
            "a read that names nothing",
            33,
            "resources/read",
            &json!({}),
        ),
    ]
}

/// The requests that complete the parameter of a template.
fn completion_rows() -> Vec<Row> {
    let unread = Row {
        matches: Match::WithoutDetail,
        ..row(
            "a completion whose chosen values are not text",
            41,
            "completion/complete",
            &json!({
                "ref": { "type": "ref/resource", "uri": "library://covers/{book_id}" },
                "argument": typed("book"),
                "context": { "arguments": { "kind": 4_i32 } },
            }),
        )
    };
    vec![
        completion(
            34,
            "a completion",
            &typed("book"),
            &json!({ "arguments": { "kind": "pdf" } }),
        ),
        completion(
            35,
            "a completion that holds a hundred values",
            &typed("many"),
            &json!({}),
        ),
        completion(
            36,
            "a completion of a parameter nothing completes",
            &json!({ "name": "page", "value": "book" }),
            &json!({}),
        ),
        completion(37, "a completion that fails", &typed("none"), &json!({})),
        completion(
            38,
            "a completion that comes apart",
            &typed("panic"),
            &json!({}),
        ),
        completion(
            39,
            "a completion that reads the chosen values",
            &typed("kind"),
            &json!({ "arguments": { "kind": "pdf" } }),
        ),
        completion(
            40,
            "a completion with no context",
            &typed("book-1"),
            &Value::Null,
        ),
        unread,
        row(
            "a completion for a prompt",
            42,
            "completion/complete",
            &json!({
                "ref": { "type": "ref/prompt", "name": "review" },
                "argument": typed("book"),
            }),
        ),
        row(
            "a completion for a template nothing owns",
            43,
            "completion/complete",
            &json!({
                "ref": { "type": "ref/resource", "uri": "elsewhere://{thing}" },
                "argument": typed("book"),
            }),
        ),
        row(
            "a completion that names no template",
            44,
            "completion/complete",
            &json!({ "argument": typed("book") }),
        ),
        row(
            "a completion that names no argument",
            45,
            "completion/complete",
            &json!({ "ref": { "type": "ref/resource", "uri": "library://covers/{book_id}" } }),
        ),
    ]
}

/// The requests that are no resource request, and the ones the protocol refuses.
fn other_rows() -> Vec<Row> {
    let mut unsupported = request(
        &Value::from(60_u64),
        "resources/read",
        &json!({ "uri": "guide://library" }),
    );
    unsupported["params"]["_meta"]["io.modelcontextprotocol/protocolVersion"] = json!("2025-11-25");
    let mut unmetered = request(
        &Value::from(61_u64),
        "resources/read",
        &json!({ "uri": "guide://library" }),
    );
    if let Some(carried) = unmetered["params"]["_meta"].as_object_mut() {
        carried.remove("io.modelcontextprotocol/clientCapabilities");
    }
    vec![
        row(
            "a tool",
            46,
            "tools/call",
            &json!({ "name": "get-book", "arguments": { "book_id": "book-1" } }),
        ),
        row(
            "a tool of another service",
            47,
            "tools/call",
            &json!({ "name": "get-plain", "arguments": { "book_id": "book-1" } }),
        ),
        row_without_detail(
            "a tool whose message the validator refuses",
            48,
            "tools/call",
            &json!({ "name": "get-book", "arguments": { "book_id": "" } }),
        ),
        row(
            "a resource called as a tool",
            49,
            "tools/call",
            &json!({ "name": "library-guide", "arguments": {} }),
        ),
        row("a call that names no tool", 50, "tools/call", &json!({})),
        row("an unknown method", 51, "ping", &json!({})),
        Row {
            matches: Match::Exact,
            name: "a notification",
            text: json!({ "jsonrpc": "2.0", "method": "notifications/cancelled", "params": {} })
                .to_string(),
        },
        Row {
            matches: Match::Exact,
            name: "an unsupported version",
            text: unsupported.to_string(),
        },
        Row {
            matches: Match::Exact,
            name: "a missing _meta key",
            text: unmetered.to_string(),
        },
    ]
}

/// The requests of a run, in order.
pub fn rows() -> Vec<Row> {
    let mut every = listing_rows();
    every.extend(read_rows());
    every.extend(completion_rows());
    every.extend(other_rows());
    every
}

pub fn headers_for(message: &Value) -> Vec<(String, String)> {
    let method = message["method"].as_str().unwrap().to_owned();
    let mut headers = vec![
        ("Content-Type".to_owned(), "application/json".to_owned()),
        ("MCP-Protocol-Version".to_owned(), VERSION.to_owned()),
        ("Mcp-Method".to_owned(), method.clone()),
    ];
    if method == "resources/read" {
        headers.push((
            "Mcp-Name".to_owned(),
            message["params"]["uri"].as_str().unwrap().to_owned(),
        ));
    }
    headers
}

fn http_row(name: &'static str, message: &Value) -> HttpRow {
    HttpRow {
        body: message.to_string(),
        headers: headers_for(message),
        name,
    }
}

pub fn http_rows() -> Vec<HttpRow> {
    let reading = request(
        &Value::from(1_u64),
        "resources/read",
        &json!({ "uri": "library://covers/book-1" }),
    );
    let mut wrong_name = http_row("a name header that differs from the URI", &reading);
    wrong_name.headers[3].1 = "library://covers/book-2".to_owned();
    let mut encoded = http_row("a name header written in base64", &reading);
    encoded.headers[3].1 = "=?base64?bGlicmFyeTovL2NvdmVycy9ib29rLTE=?=".to_owned();
    let mut no_name = http_row("a read with no name header", &reading);
    no_name.headers.remove(3);
    vec![
        http_row("a resource read by its URI", &reading),
        wrong_name,
        encoded,
        no_name,
        http_row(
            "a listing",
            &request(&Value::from(2_u64), "resources/list", &json!({})),
        ),
        http_row(
            "a completion",
            &request(
                &Value::from(3_u64),
                "completion/complete",
                &json!({
                    "ref": { "type": "ref/resource", "uri": "library://covers/{book_id}" },
                    "argument": { "name": "book_id", "value": "book" },
                }),
            ),
        ),
        http_row(
            "a template list",
            &request(&Value::from(4_u64), "resources/templates/list", &json!({})),
        ),
    ]
}

fn plain_terms(row: &HttpRow) -> McpHttpRequest {
    McpHttpRequest::new(
        "POST".to_owned(),
        "/mcp".to_owned(),
        row.headers.clone(),
        row.body.clone().into_bytes(),
    )
}

/// What the Rust dispatcher of `LibraryService` alone answers to the request in plain terms.
pub fn rust_http(row: &HttpRow) -> HttpAnswer {
    let answered = through(|stream| {
        poll_once(mcp_library_dispatcher::answer_http(
            &plain_terms(row),
            &SETTINGS,
            stream,
            &Stand,
            async |_request, _call| Ok(()),
        ))
        .unwrap()
    });
    (
        answered.status(),
        answered.headers().to_vec(),
        String::from_utf8(answered.body().to_vec()).unwrap(),
    )
}

/// What the Rust dispatcher answers to `text` for the endpoint of the library and the depot.
pub fn workspace_answer(text: &str) -> Option<String> {
    poll_once(mcp_library_dispatcher::serve(
        text,
        "Workspace",
        None,
        &WORKSPACE,
        |call| call.progress(0.0_f64, None, None),
        mcp_library_dispatcher::McpHost::new(
            &mcp_library_dispatcher::McpInFlight::default(),
            &Sink,
        ),
        &|ctx: &(), part, method: &str, params: &Value| {
            Box::pin(async move {
                match part {
                    0 => mcp_library_dispatcher::route(method, params, &Stand, ctx).await,
                    1 => mcp_depot_dispatcher::route(method, params, &Stand, ctx).await,
                    _ => None,
                }
            })
        },
    ))
    .flatten()
}

/// What the Rust dispatcher of `LibraryService` alone answers to `text`.
pub fn library_answer(text: &str) -> Option<String> {
    poll_once(mcp_library_dispatcher::answer(
        text,
        &Stand,
        |call| call.progress(0.0_f64, None, None),
        &Sink,
        &mcp_library_dispatcher::McpInFlight::default(),
    ))
    .flatten()
}

/// What the Rust dispatcher of `DepotService` alone answers to `text`.
pub fn depot_answer(text: &str) -> Option<String> {
    poll_once(mcp_depot_dispatcher::answer(
        text,
        &Stand,
        |call| call.progress(0.0_f64, None, None),
        &Sink,
        &mcp_depot_dispatcher::McpInFlight::default(),
    ))
    .flatten()
}

/// What the Rust dispatcher of `BareService` alone answers to `text`: a service that declares no
/// resource, so every resource request is one nothing on its endpoint owns.
pub fn bare_answer(text: &str) -> Option<String> {
    poll_once(mcp_bare_dispatcher::answer(
        text,
        &Stand,
        |call| call.progress(0.0_f64, None, None),
        &Sink,
        &mcp_bare_dispatcher::McpInFlight::default(),
    ))
    .flatten()
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

/// Holds an answer of a server to `method` to the official schema: the response, and the result or
/// the error inside it.
pub fn assert_answer_valid(method: &str, answer: &str) {
    let parsed: Value = serde_json::from_str(answer).unwrap();
    if let Some(error) = parsed.get("error") {
        let mut held = parsed.clone();
        if held["id"].is_null() {
            held.as_object_mut().unwrap().remove("id");
        }
        assert_valid("JSONRPCErrorResponse", &held);
        assert_valid("Error", error);
        return;
    }
    assert_valid("JSONRPCResultResponse", &parsed);
    let definition = match method {
        "server/discover" => "DiscoverResult",
        "tools/list" => "ListToolsResult",
        "resources/list" => "ListResourcesResult",
        "resources/templates/list" => "ListResourceTemplatesResult",
        "resources/read" => "ReadResourceResult",
        "completion/complete" => "CompleteResult",
        "skills/list" | "skills/get" | "skills" => return,
        _ => "CallToolResult",
    };
    assert_valid(definition, &parsed["result"]);
}

/// The answer a row expects, held to the form its match says.
pub fn expected(matches: Match, text: &str) -> String {
    match matches {
        Match::Exact => text.to_owned(),
        Match::WithoutDetail => without_detail(&serde_json::from_str(text).unwrap()).to_string(),
    }
}

/// `answered`, a text another server wrote, in the form [`expected`] holds the Rust answer to.
pub fn observed(matches: Match, answered: &str) -> String {
    match matches {
        Match::Exact => answered.to_owned(),
        Match::WithoutDetail => {
            without_detail(&serde_json::from_str(answered).unwrap()).to_string()
        }
    }
}

/// An answer whose fault, in a tool execution error or in the data of an error, is left without
/// its `detail`.
fn without_detail(answer: &Value) -> Value {
    let strip = |fault: &mut serde_json::Map<String, Value>| {
        fault.remove("detail");
    };
    let mut stripped = answer.clone();
    if let Some(fault) = stripped["error"]["data"]["fault"].as_object_mut() {
        strip(fault);
        return stripped;
    }
    let Some(text) = stripped["result"]["content"][0]["text"].as_str() else {
        return stripped;
    };
    let mut carried: Value = serde_json::from_str(text).unwrap();
    if let Some(fault) = carried["fault"].as_object_mut() {
        strip(fault);
    }
    stripped["result"]["content"][0]["text"] = Value::String(carried.to_string());
    stripped
}

/// The method a text asks for, for holding its answer to the right definition.
pub fn method_of(text: &str) -> String {
    serde_json::from_str::<Value>(text)
        .ok()
        .and_then(|asked| asked["method"].as_str().map(str::to_owned))
        .unwrap_or_default()
}

/// Holds every answer of a server to the Rust dispatcher's, row by row.
pub fn assert_answers(
    which: &str,
    rust: fn(&str) -> Option<String>,
    answers: &[Option<String>],
    rows: &[Row],
) {
    assert_eq!(answers.len(), rows.len(), "{which}");
    for (row, answered) in rows.iter().zip(answers) {
        let rust_said = rust(&row.text);
        assert_eq!(
            answered.is_some(),
            rust_said.is_some(),
            "{which}: {} is answered as Rust answers it",
            row.name
        );
        let (Some(other), Some(rust_text)) = (answered, rust_said) else {
            continue;
        };
        assert_answer_valid(&method_of(&row.text), other);
        assert_eq!(
            observed(row.matches, other),
            expected(row.matches, &rust_text),
            "{which}: {} is answered as Rust answers it. Server: {other} Rust: {rust_text}",
            row.name
        );
    }
}

/// Holds every plain HTTP answer of a server to the Rust dispatcher's.
pub fn assert_http(which: &str, rows: &[HttpRow], answers: &[HttpAnswer]) {
    assert_eq!(answers.len(), rows.len(), "{which}");
    for (row, (status, headers, body)) in rows.iter().zip(answers) {
        let (rust_status, rust_headers, rust_body) = rust_http(row);
        assert_eq!(*status, rust_status, "{which}: {}: {body}", row.name);
        let lowered = |held: &[(String, String)]| -> Vec<(String, String)> {
            held.iter()
                .map(|(name, value)| (name.to_ascii_lowercase(), value.clone()))
                .collect()
        };
        assert_eq!(
            lowered(headers),
            lowered(&rust_headers),
            "{which}: {}",
            row.name
        );
        assert_eq!(body, &rust_body, "{which}: {}", row.name);
    }
}

exercised!(
    library_exercised,
    mcp_library_dispatcher,
    LibraryServiceSchema,
    Stand
);
exercised!(
    depot_exercised,
    mcp_depot_dispatcher,
    DepotServiceSchema,
    Stand
);
exercised!(
    bare_exercised,
    mcp_bare_dispatcher,
    BareServiceSchema,
    Stand
);
exercised!(
    shelf_exercised,
    mcp_shelf_dispatcher,
    ShelfServiceSchema,
    Shelves
);

exercised!(
    lesson_exercised,
    mcp_lesson_dispatcher,
    LessonServiceSchema,
    Lessons
);

#[test]
fn every_dispatcher_serves_a_read_over_http_alone_and_on_an_endpoint_and_answers_a_denial() {
    let row = http_rows().remove(0);
    library_exercised(&row);
    depot_exercised(&row);
    bare_exercised(&row);
    shelf_exercised(&row);
    lesson_exercised(&row);
}

#[test]
fn the_mirror_repeats_the_uris_of_the_library_and_answers_them() {
    assert_eq!(
        MirrorServiceSchema::MCP_NAMES,
        &[
            ("resource template", "library://covers/{book_id}"),
            ("resource", "guide://library")
        ]
    );
    let guide = poll_once(MirrorService::mirror_guide(&Stand, &())).unwrap();
    assert_eq!(guide, Ok("# The library again".to_owned()));
    let cover = poll_once(MirrorService::mirror_cover(
        &Stand,
        &(),
        "book-1".to_owned(),
    ))
    .unwrap();
    assert_eq!(cover, Ok("book-1".to_owned()));
}
