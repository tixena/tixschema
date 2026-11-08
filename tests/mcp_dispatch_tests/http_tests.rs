//! `serve_http` and `answer_http`: one request per rule, in the order the rules are applied.

use crate::billing_mcp;
use crate::document_mcp::{self, McpHttpRequest, McpHttpResponse, McpHttpSettings};
use crate::support::{
    CENTS, HEADER_MISMATCH, INVALID_PARAMS, INVALID_REQUEST, METHOD_NOT_FOUND, UNSUPPORTED_VERSION,
    VERSION_NUMBER, WORKSPACE, assert_error, assert_result, assert_valid, block_on, call, id,
    request,
};
use crate::tests::Archive;
use serde_json::{Value, json};

pub const ORIGIN: &str = "https://app.example.com";
const SETTINGS: McpHttpSettings<'static> = McpHttpSettings {
    allowed_origins: &[ORIGIN],
    oauth: None,
};

fn headers_for(message: &Value) -> Vec<(String, String)> {
    let method = message["method"].as_str().unwrap().to_owned();
    let mut headers = vec![
        ("Content-Type".to_owned(), "application/json".to_owned()),
        ("MCP-Protocol-Version".to_owned(), "2026-07-28".to_owned()),
        ("Mcp-Method".to_owned(), method.clone()),
    ];
    if method == "tools/call" {
        headers.push((
            "Mcp-Name".to_owned(),
            message["params"]["name"].as_str().unwrap().to_owned(),
        ));
    }
    if method == "resources/read" {
        headers.push((
            "Mcp-Name".to_owned(),
            message["params"]["uri"].as_str().unwrap().to_owned(),
        ));
    }
    headers
}

pub fn post(message: &Value) -> McpHttpRequest {
    McpHttpRequest {
        body: message.to_string().into_bytes(),
        headers: headers_for(message),
        method: "POST".to_owned(),
        path: "/mcp".to_owned(),
    }
}

fn respond(asked: &McpHttpRequest) -> McpHttpResponse {
    block_on(document_mcp::answer_http(
        asked,
        &SETTINGS,
        &Archive::default(),
        async |_request, _call| Ok(()),
    ))
}

pub fn body_of(response: &McpHttpResponse) -> Value {
    serde_json::from_slice(&response.body).unwrap()
}

pub fn get_version() -> Value {
    call(&id(1), "get-version", &json!({ "document_id": "doc-1" }))
}

fn nothing() -> Vec<u8> {
    Vec::new()
}

#[test]
fn a_method_other_than_post_is_refused_with_405_and_no_body() {
    for method in ["GET", "DELETE", "PUT", "OPTIONS"] {
        let mut asked = post(&get_version());
        method.clone_into(&mut asked.method);
        let response = respond(&asked);
        assert_eq!(response.status, 405, "{method}");
        assert_eq!(response.body, nothing());
    }
}

#[test]
fn an_origin_that_is_not_allowed_is_refused_with_403_and_no_body() {
    let mut asked = post(&get_version());
    asked
        .headers
        .push(("Origin".to_owned(), "https://evil.example.com".to_owned()));
    let response = respond(&asked);
    assert_eq!(response.status, 403);
    assert_eq!(response.body, nothing());
}

#[test]
fn an_origin_that_is_allowed_or_absent_is_answered() {
    assert_eq!(respond(&post(&get_version())).status, 200);
    let mut asked = post(&get_version());
    asked.headers.push(("origin".to_owned(), ORIGIN.to_owned()));
    assert_eq!(respond(&asked).status, 200);
}

#[test]
fn an_unreadable_body_is_refused_with_400_and_32600() {
    for body in [b"not json".to_vec(), vec![0xff, 0xfe], b"[]".to_vec()] {
        let mut asked = post(&get_version());
        asked.body = body;
        let response = respond(&asked);
        assert_eq!(response.status, 400);
        let sent = body_of(&response);
        assert_eq!(sent["error"]["code"], INVALID_REQUEST);
        assert_eq!(sent["id"], Value::Null);
        assert_valid("Error", &sent["error"]);
    }
}

#[test]
fn a_notification_is_accepted_with_202_and_no_body() {
    let notification =
        json!({ "jsonrpc": "2.0", "method": "notifications/cancelled", "params": {} });
    let response = respond(&post(&notification));
    assert_eq!(response.status, 202);
    assert_eq!(response.body, nothing());
}

#[test]
fn a_header_missing_or_differing_from_the_body_is_refused_with_400_and_32020() {
    let lowered = |header: &str| header.to_ascii_lowercase();
    for (header, replacement) in [
        ("MCP-Protocol-Version", None),
        ("MCP-Protocol-Version", Some("2025-11-25")),
        ("Mcp-Method", None),
        ("Mcp-Method", Some("tools/list")),
        ("Mcp-Name", None),
        ("Mcp-Name", Some("rename-document")),
    ] {
        let mut asked = post(&get_version());
        asked
            .headers
            .retain(|(held, _)| lowered(held) != lowered(header));
        if let Some(value) = replacement {
            asked.headers.push((header.to_owned(), value.to_owned()));
        }
        let response = respond(&asked);
        assert_eq!(response.status, 400, "{header} {replacement:?}");
        let sent = body_of(&response);
        assert_eq!(
            sent["error"]["code"], HEADER_MISMATCH,
            "{header} {replacement:?}"
        );
        assert_eq!(sent["id"], id(1));
        assert_error("HeaderMismatchError", &sent);
    }
}

#[test]
fn header_names_are_compared_in_any_case_and_a_name_may_be_written_in_base64() {
    let mut asked = post(&get_version());
    asked.headers = vec![
        ("mcp-protocol-version".to_owned(), "2026-07-28".to_owned()),
        ("MCP-METHOD".to_owned(), "tools/call".to_owned()),
        (
            "mcp-name".to_owned(),
            "=?base64?Z2V0LXZlcnNpb24=?=".to_owned(),
        ),
    ];
    let response = respond(&asked);
    assert_eq!(response.status, 200);
    assert_eq!(
        body_of(&response)["result"]["structuredContent"],
        json!({ "version": VERSION_NUMBER })
    );
    asked.headers[2].1 = "=?base64?%%%?=".to_owned();
    assert_eq!(respond(&asked).status, 400);
}

#[test]
fn a_required_meta_key_missing_is_refused_with_400_and_32602() {
    let mut message = get_version();
    message["params"]["_meta"].as_object_mut().unwrap().clear();
    let response = respond(&post(&message));
    assert_eq!(response.status, 400);
    let sent = body_of(&response);
    assert_eq!(sent["error"]["code"], INVALID_PARAMS);
    assert_error("JSONRPCErrorResponse", &sent);
}

#[test]
fn an_unsupported_version_is_refused_with_400_and_32022() {
    let mut message = get_version();
    message["params"]["_meta"]["io.modelcontextprotocol/protocolVersion"] = json!("2025-11-25");
    let mut asked = post(&message);
    asked
        .headers
        .retain(|(held, _)| !held.eq_ignore_ascii_case("mcp-protocol-version"));
    asked
        .headers
        .push(("MCP-Protocol-Version".to_owned(), "2025-11-25".to_owned()));
    let response = respond(&asked);
    assert_eq!(response.status, 400);
    let sent = body_of(&response);
    assert_eq!(sent["error"]["code"], UNSUPPORTED_VERSION);
    assert_eq!(sent["error"]["data"]["requested"], "2025-11-25");
    assert_error("UnsupportedProtocolVersionError", &sent);
}

#[test]
fn an_unknown_method_is_refused_with_404_and_32601() {
    let response = respond(&post(&request(&id(1), "sampling/createMessage", json!({}))));
    assert_eq!(response.status, 404);
    let sent = body_of(&response);
    assert_eq!(sent["error"]["code"], METHOD_NOT_FOUND);
    assert_error("JSONRPCErrorResponse", &sent);
}

#[test]
fn everything_else_is_answered_with_200_and_json() {
    let found = respond(&post(&get_version()));
    assert_eq!(found.status, 200);
    assert_eq!(
        found.headers,
        [("Content-Type".to_owned(), "application/json".to_owned())]
    );
    assert_result("CallToolResult", &body_of(&found));
    let unknown = respond(&post(&call(&id(2), "delete-everything", &json!({}))));
    assert_eq!(unknown.status, 200);
    assert_eq!(body_of(&unknown)["error"]["code"], INVALID_PARAMS);
    let listed = respond(&post(&request(&id(3), "tools/list", json!({}))));
    assert_eq!(listed.status, 200);
    assert_result("ListToolsResult", &body_of(&listed));
}

#[test]
fn an_endpoint_of_two_services_is_served_over_http_too() {
    let archive = Archive::default();
    let asked = post(&call(
        &id(1),
        "get-balance",
        &json!({ "value": "acct-1", "x-ledger": "main" }),
    ));
    let response = block_on(document_mcp::serve_http(
        &asked,
        &SETTINGS,
        "Workspace",
        None,
        &WORKSPACE,
        async |_request, _call| Ok(()),
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
    ));
    assert_eq!(response.status, 200);
    assert_eq!(
        body_of(&response)["result"]["structuredContent"],
        json!({ "cents": CENTS })
    );
}

#[test]
fn the_request_reads_a_header_in_any_case() {
    let asked = post(&get_version());
    assert_eq!(asked.header("MCP-METHOD"), Some("tools/call"));
    assert_eq!(asked.header("missing"), None);
}
