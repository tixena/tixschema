//! `serve_http` and `answer_http`: one request per rule, in the order the rules are applied.

use crate::billing_mcp;
use crate::document_mcp::{self, McpHttpRequest, McpHttpResponse, McpHttpSettings};
use crate::support::{
    CENTS, HEADER_MISMATCH, INVALID_PARAMS, INVALID_REQUEST, METHOD_NOT_FOUND, UNSUPPORTED_VERSION,
    VERSION_NUMBER, WORKSPACE, assert_error, assert_result, assert_valid, block_on, call, id,
    request, through,
};
use crate::tests::Archive;
use serde_json::{Value, json};

pub const ORIGIN: &str = "https://app.example.com";
const SETTINGS: McpHttpSettings<'static> = McpHttpSettings::new(&[ORIGIN], None);

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
    McpHttpRequest::new(
        "POST".to_owned(),
        "/mcp".to_owned(),
        headers_for(message),
        message.to_string().into_bytes(),
    )
}

fn respond(asked: &McpHttpRequest) -> McpHttpResponse {
    through(|stream| {
        block_on(document_mcp::answer_http(
            asked,
            &SETTINGS,
            stream,
            &Archive::default(),
            async |_request, _call| Ok(document_mcp::McpAdmitted::new(())),
        ))
    })
}

pub fn body_of(response: &McpHttpResponse) -> Value {
    serde_json::from_slice(response.body()).unwrap()
}

pub fn get_version() -> Value {
    call(&id(1), "get-version", &json!({ "document_id": "doc-1" }))
}

fn edited(from: &McpHttpRequest, edit: impl FnOnce(&mut Vec<(String, String)>)) -> McpHttpRequest {
    let mut headers = from.headers().to_vec();
    edit(&mut headers);
    McpHttpRequest::new(
        from.method().to_owned(),
        from.path().to_owned(),
        headers,
        from.body().to_vec(),
    )
}

fn nothing() -> Vec<u8> {
    Vec::new()
}

#[test]
fn a_method_other_than_post_is_refused_with_405_and_no_body() {
    for method in ["GET", "DELETE", "PUT", "OPTIONS"] {
        let posted = post(&get_version());
        let asked = McpHttpRequest::new(
            method.to_owned(),
            posted.path().to_owned(),
            posted.headers().to_vec(),
            posted.body().to_vec(),
        );
        let response = respond(&asked);
        assert_eq!(response.status(), 405, "{method}");
        assert_eq!(response.body(), nothing());
    }
}

#[test]
fn an_origin_that_is_not_allowed_is_refused_with_403_and_no_body() {
    let asked = edited(&post(&get_version()), |headers| {
        headers.push(("Origin".to_owned(), "https://evil.example.com".to_owned()));
    });
    let response = respond(&asked);
    assert_eq!(response.status(), 403);
    assert_eq!(response.body(), nothing());
}

#[test]
fn an_origin_that_is_allowed_or_absent_is_answered() {
    assert_eq!(respond(&post(&get_version())).status(), 200);
    let asked = edited(&post(&get_version()), |headers| {
        headers.push(("origin".to_owned(), ORIGIN.to_owned()));
    });
    assert_eq!(respond(&asked).status(), 200);
}

#[test]
fn an_unreadable_body_is_refused_with_400_and_32600() {
    for body in [b"not json".to_vec(), vec![0xff, 0xfe], b"[]".to_vec()] {
        let posted = post(&get_version());
        let asked = McpHttpRequest::new(
            posted.method().to_owned(),
            posted.path().to_owned(),
            posted.headers().to_vec(),
            body,
        );
        let response = respond(&asked);
        assert_eq!(response.status(), 400);
        let sent = body_of(&response);
        assert_eq!(sent["error"]["code"], INVALID_REQUEST);
        assert!(sent.get("id").is_none());
        assert_valid("JSONRPCErrorResponse", &sent);
    }
}

#[test]
fn a_notification_is_accepted_with_202_and_no_body() {
    let notification =
        json!({ "jsonrpc": "2.0", "method": "notifications/cancelled", "params": {} });
    let response = respond(&post(&notification));
    assert_eq!(response.status(), 202);
    assert_eq!(response.body(), nothing());
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
        let asked = edited(&post(&get_version()), |headers| {
            headers.retain(|(held, _)| lowered(held) != lowered(header));
            if let Some(value) = replacement {
                headers.push((header.to_owned(), value.to_owned()));
            }
        });
        let response = respond(&asked);
        assert_eq!(response.status(), 400, "{header} {replacement:?}");
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
    let posted = post(&get_version());
    let asked = McpHttpRequest::new(
        posted.method().to_owned(),
        posted.path().to_owned(),
        vec![
            ("mcp-protocol-version".to_owned(), "2026-07-28".to_owned()),
            ("MCP-METHOD".to_owned(), "tools/call".to_owned()),
            (
                "mcp-name".to_owned(),
                "=?base64?Z2V0LXZlcnNpb24=?=".to_owned(),
            ),
        ],
        posted.body().to_vec(),
    );
    let response = respond(&asked);
    assert_eq!(response.status(), 200);
    assert_eq!(
        body_of(&response)["result"]["structuredContent"],
        json!({ "version": VERSION_NUMBER })
    );
    let garbled = edited(&asked, |headers| {
        headers[2].1 = "=?base64?%%%?=".to_owned();
    });
    assert_eq!(respond(&garbled).status(), 400);
}

#[test]
fn a_required_meta_key_missing_is_refused_with_400_and_32602() {
    let mut message = get_version();
    message["params"]["_meta"].as_object_mut().unwrap().clear();
    let response = respond(&post(&message));
    assert_eq!(response.status(), 400);
    let sent = body_of(&response);
    assert_eq!(sent["error"]["code"], INVALID_PARAMS);
    assert_error("JSONRPCErrorResponse", &sent);
}

#[test]
fn an_unsupported_version_is_refused_with_400_and_32022() {
    let mut message = get_version();
    message["params"]["_meta"]["io.modelcontextprotocol/protocolVersion"] = json!("2025-11-25");
    let asked = edited(&post(&message), |headers| {
        headers.retain(|(held, _)| !held.eq_ignore_ascii_case("mcp-protocol-version"));
        headers.push(("MCP-Protocol-Version".to_owned(), "2025-11-25".to_owned()));
    });
    let response = respond(&asked);
    assert_eq!(response.status(), 400);
    let sent = body_of(&response);
    assert_eq!(sent["error"]["code"], UNSUPPORTED_VERSION);
    assert_eq!(sent["error"]["data"]["requested"], "2025-11-25");
    assert_error("UnsupportedProtocolVersionError", &sent);
}

#[test]
fn an_unknown_method_is_refused_with_404_and_32601() {
    let response = respond(&post(&request(&id(1), "sampling/createMessage", json!({}))));
    assert_eq!(response.status(), 404);
    let sent = body_of(&response);
    assert_eq!(sent["error"]["code"], METHOD_NOT_FOUND);
    assert_error("JSONRPCErrorResponse", &sent);
}

#[test]
fn everything_else_is_answered_with_200_and_json() {
    let found = respond(&post(&get_version()));
    assert_eq!(found.status(), 200);
    assert_eq!(
        found.headers(),
        [("Content-Type".to_owned(), "application/json".to_owned())]
    );
    assert_result("CallToolResult", &body_of(&found));
    let unknown = respond(&post(&call(&id(2), "delete-everything", &json!({}))));
    assert_eq!(unknown.status(), 200);
    assert_eq!(body_of(&unknown)["error"]["code"], INVALID_PARAMS);
    let listed = respond(&post(&request(&id(3), "tools/list", json!({}))));
    assert_eq!(listed.status(), 200);
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
    let response = through(|stream| {
        block_on(document_mcp::serve_http(
            &asked,
            &SETTINGS,
            stream,
            &document_mcp::McpServer::new(&WORKSPACE, None, "Workspace"),
            async |_request, _call| Ok(document_mcp::McpAdmitted::new(())),
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
    });
    assert_eq!(response.status(), 200);
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

fn admitted_as(name: Option<&str>) -> document_mcp::McpAdmitted<()> {
    name.map_or_else(
        || document_mcp::McpAdmitted::new(()),
        |given| document_mcp::McpAdmitted::named((), given.to_owned()),
    )
}

fn serve_as(asked: &McpHttpRequest, name: Option<&str>) -> McpHttpResponse {
    let archive = Archive::default();
    through(|stream| {
        block_on(document_mcp::serve_http(
            asked,
            &SETTINGS,
            stream,
            &document_mcp::McpServer::new(&WORKSPACE, None, "Workspace"),
            async |_request, _call| Ok(admitted_as(name)),
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
    })
}

fn answer_as(asked: &McpHttpRequest, name: Option<&str>) -> McpHttpResponse {
    through(|stream| {
        block_on(document_mcp::answer_http(
            asked,
            &SETTINGS,
            stream,
            &Archive::default(),
            async |_request, _call| Ok(admitted_as(name)),
        ))
    })
}

fn named_in(response: &McpHttpResponse) -> String {
    assert_eq!(response.status(), 200);
    body_of(response)["result"]["_meta"]["io.modelcontextprotocol/serverInfo"]["name"]
        .as_str()
        .unwrap()
        .to_owned()
}

fn discover() -> McpHttpRequest {
    post(&request(&id(1), "server/discover", json!({})))
}

#[test]
fn a_name_admit_gave_is_the_name_of_every_answer_of_the_request() {
    for message in [
        request(&id(1), "server/discover", json!({})),
        request(&id(2), "tools/list", json!({})),
        request(&id(3), "resources/list", json!({})),
        call(&id(4), "get-version", &json!({ "document_id": "doc-1" })),
    ] {
        let answered = serve_as(&post(&message), Some("Workspace-acme"));
        assert_eq!(named_in(&answered), "Workspace-acme", "{message}");
        let unnamed = serve_as(&post(&message), None);
        assert_eq!(named_in(&unnamed), "Workspace", "{message}");
    }
}

#[test]
fn two_callers_of_one_server_are_answered_under_their_own_names() {
    let asked = discover();
    assert_eq!(
        named_in(&serve_as(&asked, Some("Workspace-acme"))),
        "Workspace-acme"
    );
    assert_eq!(
        named_in(&serve_as(&asked, Some("Workspace-globex"))),
        "Workspace-globex"
    );
    assert_eq!(named_in(&serve_as(&asked, None)), "Workspace");
}

#[test]
fn a_public_answer_is_private_when_admit_named_the_server_and_keeps_its_ttl() {
    for message in [
        request(&id(1), "server/discover", json!({})),
        request(&id(2), "tools/list", json!({})),
    ] {
        let named = body_of(&serve_as(&post(&message), Some("Workspace-acme")))["result"].clone();
        let plain = body_of(&serve_as(&post(&message), None))["result"].clone();
        assert_eq!(plain["cacheScope"], "public", "{message}");
        assert_eq!(named["cacheScope"], "private", "{message}");
        assert_eq!(named["ttlMs"], plain["ttlMs"], "{message}");
    }
}

#[test]
fn an_answer_that_was_private_stays_private_under_a_name_admit_gave() {
    let message = request(&id(1), "resources/list", json!({}));
    for name in [None, Some("Workspace-acme")] {
        let answered = body_of(&serve_as(&post(&message), name));
        assert_eq!(answered["result"]["cacheScope"], "private", "{name:?}");
    }
}

#[test]
fn a_service_served_alone_answers_the_admitted_name_and_then_its_own() {
    let asked = discover();
    let named = answer_as(&asked, Some("DocumentService-acme"));
    assert_eq!(named_in(&named), "DocumentService-acme");
    assert_eq!(body_of(&named)["result"]["cacheScope"], "private");
    let own = answer_as(&asked, None);
    assert_eq!(named_in(&own), document_mcp::SERVER_NAME);
    assert_eq!(body_of(&own)["result"]["cacheScope"], "public");
}

#[test]
fn a_denied_request_answers_403_with_no_body_and_no_headers() {
    let asked = discover();
    let denied = through(|stream| {
        block_on(document_mcp::answer_http(
            &asked,
            &SETTINGS,
            stream,
            &Archive::default(),
            async |_request, _call| {
                Err::<document_mcp::McpAdmitted<()>, _>(document_mcp::McpDenied::Forbidden)
            },
        ))
    });
    assert_eq!(denied.status(), 403);
    assert_eq!(denied.body(), nothing());
    assert_eq!(denied.headers(), []);
}
