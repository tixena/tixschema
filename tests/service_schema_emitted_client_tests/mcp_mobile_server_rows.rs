//! What the Kotlin `mcp` server run is held to: the requests it is sent, in plain terms and as
//! JSON-RPC texts, and the answers the Rust dispatcher gives them.

use super::mcp_mobile_billing;
use super::mcp_mobile_document::{self, McpEndpoint, McpHttpRequest, McpHttpSettings};
use super::mcp_mobile_rows::{VERSION, block_on, call, request};
use super::mcp_mobile_service::{Archive, Sink};
use super::{MobileBillingServiceSchema, MobileDocumentServiceSchema};
use serde_json::{Value, json};

pub const ORIGIN: &str = "https://app.example.com";

const DOCUMENT_SETTINGS: McpHttpSettings<'static> = McpHttpSettings {
    allowed_origins: &[ORIGIN],
    oauth: None,
};

const BILLING_SETTINGS: mcp_mobile_billing::McpHttpSettings<'static> =
    mcp_mobile_billing::McpHttpSettings {
        allowed_origins: &[ORIGIN],
        oauth: None,
    };

/// Two services behind one endpoint, built in a `const` so a shared name would stop the build.
pub const WORKSPACE: McpEndpoint = McpEndpoint::new(
    &[
        MobileDocumentServiceSchema::MCP_NAMES,
        MobileBillingServiceSchema::MCP_NAMES,
    ],
    &[mcp_mobile_document::catalog, mcp_mobile_billing::catalog],
);

/// How the answer of another server is held against the Rust dispatcher's.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Match {
    /// An error of the same code and no `id`: the words of the parse error are the library's.
    CodeOnly,
    /// The same bytes.
    Exact,
    /// The same bytes, once the `detail` of the fault in the answer is left out: it is the words
    /// of the serialization library that refused the value.
    WithoutDetail,
    /// The same bytes, but for the `id` key, which Rust answers as `null` and the official schema
    /// does not allow to be null.
    WithoutId,
}

pub struct ServerRow {
    pub matches: Match,
    pub name: &'static str,
    pub text: String,
}

/// One HTTP request, handed to each side.
pub struct HttpRow {
    pub body: String,
    pub headers: Vec<(String, String)>,
    pub method: &'static str,
    pub name: &'static str,
    pub without_id: bool,
}

fn row(name: &'static str, asked: &Value) -> ServerRow {
    ServerRow {
        matches: Match::Exact,
        name,
        text: asked.to_string(),
    }
}

fn row_as(name: &'static str, matches: Match, text: &str) -> ServerRow {
    ServerRow {
        matches,
        name,
        text: text.to_owned(),
    }
}

fn without_meta_key(mut asked: Value, key: &str) -> Value {
    asked["params"]["_meta"]
        .as_object_mut()
        .unwrap()
        .remove(key);
    asked
}

/// The requests of the document's run.
fn documented_rows() -> Vec<ServerRow> {
    let mut unsupported = call(7, "get-version", &json!({ "document_id": "doc-1" }));
    unsupported["params"]["_meta"]["io.modelcontextprotocol/protocolVersion"] = json!("2025-11-25");
    vec![
        row(
            "server/discover",
            &request(&json!("discover-1"), "server/discover", json!({})),
        ),
        row(
            "tools/list",
            &request(&Value::from(1_u64), "tools/list", json!({})),
        ),
        row(
            "a success",
            &call(2, "get-version", &json!({ "document_id": "doc-1" })),
        ),
        row(
            "a declared error",
            &call(3, "get-version", &json!({ "document_id": "missing" })),
        ),
        row(
            "a message the validator refuses",
            &call(4, "get-version", &json!({ "document_id": "" })),
        ),
        row(
            "a tool of the second service",
            &call(
                5,
                "get-balance",
                &json!({ "value": "acct-1", "x-ledger": "main" }),
            ),
        ),
        row("an unknown tool", &call(6, "delete-everything", &json!({}))),
        row_as(
            "a notification",
            Match::Exact,
            &json!({ "jsonrpc": "2.0", "method": "notifications/cancelled", "params": {} })
                .to_string(),
        ),
        row("an unsupported version", &unsupported),
        row(
            "an unknown method",
            &request(&Value::from(8_u64), "ping", json!({})),
        ),
        row(
            "a resource of the second service",
            &request(
                &Value::from(9_u64),
                "resources/read",
                json!({ "uri": "billing://invoices/inv-1" }),
            ),
        ),
        row(
            "resource templates",
            &request(&Value::from(10_u64), "resources/templates/list", json!({})),
        ),
        row(
            "resources",
            &request(&Value::from(29_u64), "resources/list", json!({})),
        ),
        row(
            "resources from a cursor that names no service",
            &request(
                &Value::from(30_u64),
                "resources/list",
                json!({ "cursor": "5:" }),
            ),
        ),
        row(
            "a completion",
            &request(
                &Value::from(31_u64),
                "completion/complete",
                json!({
                    "argument": { "name": "invoice", "value": "inv" },
                    "ref": { "type": "ref/resource", "uri": "billing://invoices/{invoice}" },
                }),
            ),
        ),
    ]
}

/// The operations the design settles beside them: wrapping, headers, unit and one-way.
fn operation_rows() -> Vec<ServerRow> {
    vec![
        row(
            "a wrapped message",
            &call(11, "lookup-document", &json!({ "value": "doc-1" })),
        ),
        row(
            "a wrapped message with no value",
            &call(12, "lookup-document", &json!({})),
        ),
        row(
            "headers both ways",
            &call(
                13,
                "get-tagged",
                &json!({ "value": "doc-1", "if-none-match": "\"v6\"" }),
            ),
        ),
        row(
            "a declared error with a header",
            &call(14, "get-tagged", &json!({ "value": "missing" })),
        ),
        row(
            "a unit success",
            &call(
                15,
                "rename-document",
                &json!({ "documentId": "doc-1", "title": "New" }),
            ),
        ),
        row(
            "a flat message with a header",
            &call(
                27,
                "archive-document",
                &json!({ "documentId": "doc-1", "reason": "stale", "x-exchange": "reviewer-7" }),
            ),
        ),
        row(
            "a one-way operation",
            &call(
                16,
                "touch-document",
                &json!({ "documentId": "doc-1", "reader": "reviewer-7" }),
            ),
        ),
        row(
            "a one-way operation whose handler comes apart",
            &call(
                17,
                "touch-document",
                &json!({ "documentId": "doc-1", "reader": "panic" }),
            ),
        ),
        row(
            "a handler that comes apart",
            &call(18, "get-version", &json!({ "document_id": "panic" })),
        ),
        row(
            "arguments named after generated locals",
            &call(
                19,
                "tag-document",
                &json!({
                    "value": "t", "x-arguments": "a", "x-message": "m", "x-outcome": "o",
                    "x-members": "s",
                }),
            ),
        ),
        row(
            "more arguments named after generated locals",
            &call(
                28,
                "label-document",
                &json!({
                    "value": "l", "x-sent": "s", "x-next-id": "n", "x-mcp-source": "m",
                    "x-carried": "c",
                }),
            ),
        ),
    ]
}

/// The messages that are refused, and the texts that are no message.
fn refused_rows() -> Vec<ServerRow> {
    let mut not_an_object = call(21, "get-version", &json!({}));
    not_an_object["params"]["arguments"] = json!([1_u8]);
    let mut unnamed = request(&Value::from(22_u64), "tools/call", json!({}));
    unnamed["params"]["arguments"] = json!({});
    let unmetered = without_meta_key(
        call(23, "get-version", &json!({ "document_id": "doc-1" })),
        "io.modelcontextprotocol/clientCapabilities",
    );
    let looser = |name, asked: Value| ServerRow {
        matches: Match::WithoutDetail,
        name,
        text: asked.to_string(),
    };
    vec![
        looser(
            "a missing header",
            call(
                20,
                "tag-document",
                &json!({ "value": "t", "x-message": "m", "x-outcome": "o", "x-members": "s" }),
            ),
        ),
        row("arguments that are no object", &not_an_object),
        row("a call naming no tool", &unnamed),
        row("a missing _meta key", &unmetered),
        looser(
            "arguments that do not read",
            call(24, "get-version", &json!({ "document_id": 5_u8 })),
        ),
        looser(
            "a header that does not read",
            call(
                25,
                "get-tagged",
                &json!({ "value": "doc-1", "if-none-match": 5_u8 }),
            ),
        ),
        looser(
            "a wrapped message that does not read",
            call(26, "lookup-document", &json!({ "value": 5_u8 })),
        ),
        row_as("text that is no JSON", Match::CodeOnly, "not json"),
        row_as("an empty text", Match::CodeOnly, ""),
        row_as("an array", Match::WithoutId, "[]"),
        row_as(
            "another JSON-RPC version",
            Match::WithoutId,
            "{\"jsonrpc\":\"1.0\",\"id\":1,\"method\":\"x\"}",
        ),
        row_as(
            "no method",
            Match::WithoutId,
            "{\"jsonrpc\":\"2.0\",\"id\":1}",
        ),
        row_as(
            "an identifier that is an object",
            Match::WithoutId,
            "{\"jsonrpc\":\"2.0\",\"id\":{},\"method\":\"x\"}",
        ),
    ]
}

/// Every request a server is sent in a run, in order.
pub fn server_rows() -> Vec<ServerRow> {
    let mut rows = documented_rows();
    rows.extend(operation_rows());
    rows.extend(refused_rows());
    rows
}

/// What the Rust dispatcher answers to `text` for the endpoint of both services.
pub fn workspace_answer(text: &str) -> Option<String> {
    block_on(mcp_mobile_document::serve(
        text,
        "Workspace",
        None,
        &WORKSPACE,
        |call| call.progress(0.0_f64, None, None),
        mcp_mobile_document::McpHost {
            in_flight: &mcp_mobile_document::McpInFlight::default(),
            writer: &Sink,
        },
        &|ctx: &(), part, method: &str, params: &Value| {
            Box::pin(async move {
                match part {
                    0 => mcp_mobile_document::route(method, params, &Archive, ctx).await,
                    1 => mcp_mobile_billing::route(method, params, &Archive, ctx).await,
                    _ => None,
                }
            })
        },
    ))
}

/// What the Rust dispatcher of `MobileBillingService` alone answers to `text`.
pub fn billing_answer(text: &str) -> Option<String> {
    block_on(mcp_mobile_billing::answer(
        text,
        &Archive,
        |call| call.progress(0.0_f64, None, None),
        &Sink,
        &mcp_mobile_billing::McpInFlight::default(),
    ))
}

/// The answer a row expects, held to the form its match says.
pub fn expected(matches: Match, text: &str) -> String {
    let mut parsed: Value = serde_json::from_str(text).unwrap();
    match matches {
        Match::Exact => text.to_owned(),
        Match::WithoutId => {
            assert_eq!(parsed["id"], Value::Null, "Rust answers {text}");
            parsed.as_object_mut().unwrap().remove("id");
            parsed.to_string()
        }
        Match::WithoutDetail => without_detail(&parsed).to_string(),
        Match::CodeOnly => {
            json!({ "code": parsed["error"]["code"].clone(), "id": false }).to_string()
        }
    }
}

/// `answered`, a text another server wrote, in the form [`expected`] holds the Rust answer to.
pub fn observed(matches: Match, answered: &str) -> String {
    let parsed: Value = serde_json::from_str(answered).unwrap();
    match matches {
        Match::Exact | Match::WithoutId => answered.to_owned(),
        Match::WithoutDetail => without_detail(&parsed).to_string(),
        Match::CodeOnly => {
            assert!(parsed["error"]["message"].is_string(), "{answered}");
            json!({ "code": parsed["error"]["code"].clone(), "id": parsed.get("id").is_some() })
                .to_string()
        }
    }
}

/// An answer whose tool execution error carries a fault, with the fault's `detail` left out.
fn without_detail(answer: &Value) -> Value {
    let mut stripped = answer.clone();
    let Some(text) = stripped["result"]["content"][0]["text"].as_str() else {
        return stripped;
    };
    let mut carried: Value = serde_json::from_str(text).unwrap();
    carried["fault"].as_object_mut().unwrap().remove("detail");
    stripped["result"]["content"][0]["text"] = Value::String(carried.to_string());
    stripped
}

fn headers_for(message: &Value) -> Vec<(String, String)> {
    let method = message["method"].as_str().unwrap().to_owned();
    let mut headers = vec![
        ("Content-Type".to_owned(), "application/json".to_owned()),
        ("MCP-Protocol-Version".to_owned(), VERSION.to_owned()),
        ("Mcp-Method".to_owned(), method.clone()),
    ];
    if method == "tools/call" {
        headers.push((
            "Mcp-Name".to_owned(),
            message["params"]["name"].as_str().unwrap().to_owned(),
        ));
    }
    headers
}

fn http_row(name: &'static str, message: &Value) -> HttpRow {
    HttpRow {
        body: message.to_string(),
        headers: headers_for(message),
        method: "POST",
        name,
        without_id: false,
    }
}

pub fn http_rows() -> Vec<HttpRow> {
    let good = call(1, "get-version", &json!({ "document_id": "doc-1" }));
    let mut foreign = http_row("an origin that is not allowed", &good);
    foreign
        .headers
        .push(("Origin".to_owned(), "https://elsewhere.example".to_owned()));
    let mut allowed = http_row("an origin that is allowed", &good);
    allowed
        .headers
        .push(("Origin".to_owned(), ORIGIN.to_owned()));
    let mut wrong_method = http_row("a method header that differs from the body", &good);
    wrong_method.headers[2].1 = "tools/list".to_owned();
    let mut no_version = http_row("a request with no version header", &good);
    no_version.headers.remove(1);
    let mut wrong_name = http_row("a name header that differs from the body", &good);
    wrong_name.headers[3].1 = "lookup-document".to_owned();
    let mut encoded = http_row("a name header written in base64", &good);
    encoded.headers[3].1 = "=?base64?Z2V0LXZlcnNpb24=?=".to_owned();
    let unmetered = without_meta_key(
        call(2, "get-version", &json!({ "document_id": "doc-1" })),
        "io.modelcontextprotocol/clientCapabilities",
    );
    let mut unsupported = call(3, "get-version", &json!({ "document_id": "doc-1" }));
    unsupported["params"]["_meta"]["io.modelcontextprotocol/protocolVersion"] = json!("2025-11-25");
    let mut old = http_row("an unsupported version", &unsupported);
    old.headers[1].1 = "2025-11-25".to_owned();
    vec![
        http_row("a success", &good),
        HttpRow {
            method: "GET",
            name: "a request that is no POST",
            ..http_row("", &good)
        },
        foreign,
        allowed,
        HttpRow {
            body: "[]".to_owned(),
            name: "a body that is no message",
            without_id: true,
            ..http_row("", &good)
        },
        http_row(
            "a notification",
            &json!({ "jsonrpc": "2.0", "method": "notifications/cancelled", "params": {} }),
        ),
        wrong_method,
        no_version,
        wrong_name,
        encoded,
        http_row("a missing _meta key", &unmetered),
        old,
        http_row(
            "an unknown method",
            &request(&Value::from(4_u64), "ping", json!({})),
        ),
        http_row("an unknown tool", &call(5, "delete-everything", &json!({}))),
        http_row(
            "a tool list",
            &request(&Value::from(6_u64), "tools/list", json!({})),
        ),
    ]
}

fn plain_terms(row: &HttpRow) -> McpHttpRequest {
    McpHttpRequest {
        body: row.body.clone().into_bytes(),
        headers: row.headers.clone(),
        method: row.method.to_owned(),
        path: "/mcp".to_owned(),
    }
}

/// What the Rust dispatcher of `MobileDocumentService` alone answers to the request in plain terms.
pub fn rust_http(row: &HttpRow) -> (u16, Vec<(String, String)>, String) {
    let answered = block_on(mcp_mobile_document::answer_http(
        &plain_terms(row),
        &DOCUMENT_SETTINGS,
        &Archive,
        async |_request, _call| Ok(()),
    ));
    (
        answered.status,
        answered.headers,
        String::from_utf8(answered.body).unwrap(),
    )
}

fn billing_terms(row: &HttpRow) -> mcp_mobile_billing::McpHttpRequest {
    mcp_mobile_billing::McpHttpRequest {
        body: row.body.clone().into_bytes(),
        headers: row.headers.clone(),
        method: row.method.to_owned(),
        path: "/mcp".to_owned(),
    }
}

#[test]
fn the_second_service_answers_alone_and_beside_the_first() {
    let asked = http_rows().remove(0);
    let endpoint = mcp_mobile_billing::McpEndpoint::new(
        &[MobileBillingServiceSchema::MCP_NAMES],
        &[mcp_mobile_billing::catalog],
    );
    let listed = request(&Value::from(1_u64), "tools/list", json!({})).to_string();
    let alone = billing_answer(&listed).unwrap();
    let served = block_on(mcp_mobile_billing::serve(
        &listed,
        mcp_mobile_billing::SERVER_NAME,
        mcp_mobile_billing::INSTRUCTIONS,
        &endpoint,
        |call| call.progress(0.0_f64, None, None),
        mcp_mobile_billing::McpHost {
            in_flight: &mcp_mobile_billing::McpInFlight::default(),
            writer: &Sink,
        },
        &|ctx: &(), _part, method: &str, params: &Value| {
            Box::pin(async move { mcp_mobile_billing::route(method, params, &Archive, ctx).await })
        },
    ))
    .unwrap();
    assert_eq!(alone, served);
    assert_eq!(
        mcp_mobile_billing::INSTRUCTIONS,
        Some("Answers what a caller owes.")
    );
    let over_http = block_on(mcp_mobile_billing::answer_http(
        &billing_terms(&asked),
        &BILLING_SETTINGS,
        &Archive,
        async |_request, _call| Ok(()),
    ));
    let served_over_http = block_on(mcp_mobile_billing::serve_http(
        &billing_terms(&asked),
        &BILLING_SETTINGS,
        mcp_mobile_billing::SERVER_NAME,
        None,
        &endpoint,
        async |_request, _call| Ok(()),
        &|ctx: &(), _part, method: &str, params: &Value| {
            Box::pin(async move { mcp_mobile_billing::route(method, params, &Archive, ctx).await })
        },
    ));
    assert_eq!(over_http.status, 200);
    assert_eq!(over_http.status, served_over_http.status);
    let workspace = block_on(mcp_mobile_document::serve_http(
        &plain_terms(&asked),
        &DOCUMENT_SETTINGS,
        "Workspace",
        None,
        &WORKSPACE,
        async |_request, _call| Ok(()),
        &|ctx: &(), part, method: &str, params: &Value| {
            Box::pin(async move {
                match part {
                    0 => mcp_mobile_document::route(method, params, &Archive, ctx).await,
                    1 => mcp_mobile_billing::route(method, params, &Archive, ctx).await,
                    _ => None,
                }
            })
        },
    ));
    assert_eq!(workspace.status, rust_http(&asked).0);
}

#[test]
fn a_denial_answers_its_status_and_the_name_header_is_read_on_both_services() {
    let asked = http_rows().remove(0);
    let written = asked
        .headers
        .iter()
        .find(|(name, _)| name.eq_ignore_ascii_case("mcp-name"))
        .map(|(_, value)| value.clone());
    assert_eq!(plain_terms(&asked).mcp_name(), written);
    assert_eq!(billing_terms(&asked).mcp_name(), written);
    for (denied, status) in [
        (
            mcp_mobile_document::McpDenied::NoValidCredentials { scope: None },
            401,
        ),
        (
            mcp_mobile_document::McpDenied::InsufficientScope {
                scope: "documents:write".to_owned(),
            },
            403,
        ),
        (mcp_mobile_document::McpDenied::Forbidden, 403),
    ] {
        let response = block_on(mcp_mobile_document::answer_http(
            &plain_terms(&asked),
            &DOCUMENT_SETTINGS,
            &Archive,
            async |_request: &McpHttpRequest,
                   _call: mcp_mobile_document::McpCall|
                   -> Result<(), mcp_mobile_document::McpDenied> {
                Err(denied.clone())
            },
        ));
        assert_eq!(response.status, status, "{denied:?}");
    }
    for (denied, status) in [
        (
            mcp_mobile_billing::McpDenied::NoValidCredentials { scope: None },
            401,
        ),
        (
            mcp_mobile_billing::McpDenied::InsufficientScope {
                scope: "billing:read".to_owned(),
            },
            403,
        ),
        (mcp_mobile_billing::McpDenied::Forbidden, 403),
    ] {
        let response = block_on(mcp_mobile_billing::answer_http(
            &billing_terms(&asked),
            &BILLING_SETTINGS,
            &Archive,
            async |_request: &mcp_mobile_billing::McpHttpRequest,
                   _call: mcp_mobile_billing::McpCall|
                   -> Result<(), mcp_mobile_billing::McpDenied> { Err(denied.clone()) },
        ));
        assert_eq!(response.status, status, "{denied:?}");
    }
}
