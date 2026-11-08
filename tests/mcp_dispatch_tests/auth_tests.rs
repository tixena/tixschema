//! Authorization around `answer_http` and `serve_http`: the metadata document, `admit`, every
//! challenge, and the scopes a declaration names. A token is a name a stand-in `verify_token`
//! knows; tixschema checks none.

mod api_key_headers {
    use super::*;

    const KEYS: McpHttpSettings<'static> = McpHttpSettings {
        allowed_origins: &[ORIGIN],
        oauth: None,
    };

    struct Account {
        organization_admin: bool,
    }

    fn validate_api_key(key: &str, secret: &str) -> Option<Account> {
        match (key, secret) {
            ("admin-key", "admin-secret") => Some(Account {
                organization_admin: true,
            }),
            ("member-key", "member-secret") => Some(Account {
                organization_admin: false,
            }),
            _ => None,
        }
    }

    fn respond_keys(asked: &McpHttpRequest) -> McpHttpResponse {
        block_on(document_mcp::answer_http(
            asked,
            &KEYS,
            &Archive::default(),
            async |request: &McpHttpRequest, _call: McpCall| {
                let (Some(key), Some(secret)) =
                    (request.header("x-api-key"), request.header("x-api-secret"))
                else {
                    return Err(McpDenied::NoValidCredentials { scope: None });
                };
                match validate_api_key(key, secret) {
                    Some(account) if account.organization_admin => Ok(()),
                    _ => Err(McpDenied::Forbidden),
                }
            },
        ))
    }

    fn keyed(key: Option<&str>, secret: Option<&str>) -> McpHttpRequest {
        let mut asked = post(&get_version());
        for (name, held) in [("x-api-key", key), ("x-api-secret", secret)] {
            if let Some(text) = held {
                asked = with_header(asked, name, text);
            }
        }
        asked
    }

    #[test]
    fn the_check_of_two_headers_answers_as_the_server_it_replaces_did() {
        let admin = (Some("admin-key"), Some("admin-secret"));
        let cases = [
            (keyed(None, None), 401),
            (keyed(Some("admin-key"), None), 401),
            (keyed(None, Some("admin-secret")), 401),
            (keyed(Some("unknown"), Some("unknown")), 403),
            (keyed(Some("member-key"), Some("member-secret")), 403),
            (keyed(admin.0, admin.1), 200),
            (with_header(keyed(admin.0, admin.1), "Origin", ORIGIN), 200),
            (
                with_header(
                    keyed(admin.0, admin.1),
                    "Origin",
                    "https://evil.example.com",
                ),
                403,
            ),
            (
                with_header(keyed(None, None), "Origin", "https://evil.example.com"),
                403,
            ),
        ];
        for (asked, status) in cases {
            let response = respond_keys(&asked);
            assert_eq!(response.status, status, "{:?}", asked.headers);
            assert_eq!(challenge(&response), None, "{:?}", asked.headers);
        }
    }
}

use crate::document_mcp::{
    self, McpCall, McpDenied, McpHttpRequest, McpHttpResponse, McpHttpSettings, McpOAuth,
};
use crate::http_tests::{ORIGIN, body_of, get_version, post};
use crate::support::{HEADER_MISMATCH, assert_error, assert_valid, block_on, call, id, request};
use crate::tests::Archive;
use crate::{DocumentServiceSchema, billing_mcp};
use serde_json::{Value, json};
use std::sync::Mutex;

const METADATA_URL: &str = "https://mcp.example.com/.well-known/oauth-protected-resource/mcp";
const METADATA_PATH: &str = "/.well-known/oauth-protected-resource/mcp";
const RESOURCE: &str = "https://mcp.example.com/mcp";

const OAUTH: McpOAuth<'static> = McpOAuth {
    authorization_servers: &["https://auth.example.com"],
    resource: RESOURCE,
    resource_metadata_url: METADATA_URL,
    scopes_supported: &["documents:read"],
};

const SETTINGS: McpHttpSettings<'static> = McpHttpSettings {
    allowed_origins: &[ORIGIN],
    oauth: Some(OAUTH),
};

struct Claims {
    scope: &'static str,
}

fn verify_token(token: &str) -> Result<Claims, McpDenied> {
    match token {
        "read-token" => Ok(Claims {
            scope: "documents:read",
        }),
        "write-token" => Ok(Claims {
            scope: "documents:read documents:write",
        }),
        "file-token" => Ok(Claims {
            scope: "documents:read files:read",
        }),
        _ => Err(McpDenied::NoValidCredentials { scope: None }),
    }
}

async fn admit_scopes(request: &McpHttpRequest, _call: McpCall) -> Result<(), McpDenied> {
    let name = request.mcp_name();
    let method = request.header("mcp-method").unwrap_or_default();
    let needed = DocumentServiceSchema::mcp_scopes(method, name.as_deref());
    let Some(token) = request.bearer_token() else {
        let scope = (!needed.is_empty()).then(|| needed.join(" "));
        return Err(McpDenied::NoValidCredentials { scope });
    };
    let claims = verify_token(token)?;
    if needed
        .iter()
        .any(|scope| !claims.scope.split(' ').any(|held| held == *scope))
    {
        return Err(McpDenied::InsufficientScope {
            scope: needed.join(" "),
        });
    }
    Ok(())
}

fn respond_with(asked: &McpHttpRequest, settings: &McpHttpSettings<'_>) -> McpHttpResponse {
    block_on(document_mcp::answer_http(
        asked,
        settings,
        &Archive::default(),
        admit_scopes,
    ))
}

fn respond(asked: &McpHttpRequest) -> McpHttpResponse {
    respond_with(asked, &SETTINGS)
}

fn with_header(mut asked: McpHttpRequest, name: &str, value: &str) -> McpHttpRequest {
    asked.headers.push((name.to_owned(), value.to_owned()));
    asked
}

fn without_name(mut asked: McpHttpRequest) -> McpHttpRequest {
    asked
        .headers
        .retain(|(name, _)| !name.eq_ignore_ascii_case("mcp-name"));
    asked
}

fn bearing(token: &str, asked: McpHttpRequest) -> McpHttpRequest {
    with_header(asked, "Authorization", &format!("Bearer {token}"))
}

fn challenge(response: &McpHttpResponse) -> Option<&str> {
    response
        .headers
        .iter()
        .find(|(name, _)| name == "WWW-Authenticate")
        .map(|(_, value)| value.as_str())
}

fn rename() -> Value {
    call(
        &id(2),
        "rename-document",
        &json!({ "document_id": "doc-1", "title": "Plan" }),
    )
}

fn metadata_get() -> McpHttpRequest {
    McpHttpRequest {
        body: Vec::new(),
        headers: Vec::new(),
        method: "GET".to_owned(),
        path: METADATA_PATH.to_owned(),
    }
}

#[test]
fn a_get_at_the_metadata_path_answers_the_document_byte_for_byte() {
    let response = respond(&metadata_get());
    assert_eq!(response.status, 200);
    assert_eq!(
        response.headers,
        [("Content-Type".to_owned(), "application/json".to_owned())]
    );
    assert_eq!(
        String::from_utf8(response.body).unwrap(),
        concat!(
            r#"{"authorization_servers":["https://auth.example.com"],"#,
            r#""bearer_methods_supported":["header"],"#,
            r#""resource":"https://mcp.example.com/mcp","#,
            r#""scopes_supported":["documents:read"]}"#
        )
    );
}

#[test]
fn the_metadata_document_leaves_out_an_empty_scope_list_and_ignores_a_query() {
    let settings = McpHttpSettings {
        allowed_origins: &[],
        oauth: Some(McpOAuth {
            scopes_supported: &[],
            ..OAUTH
        }),
    };
    let mut asked = metadata_get();
    asked.path = format!("{METADATA_PATH}?probe=1");
    let sent = body_of(&respond_with(&asked, &settings));
    assert!(sent.get("scopes_supported").is_none(), "{sent}");
    assert_eq!(sent["resource"], RESOURCE);
}

#[test]
fn the_metadata_path_is_answered_ahead_of_the_origin_rule_and_without_credentials() {
    let asked = with_header(metadata_get(), "Origin", "https://evil.example.com");
    assert_eq!(respond(&asked).status, 200);
}

#[test]
fn a_get_elsewhere_or_without_oauth_answers_405() {
    let mut elsewhere = metadata_get();
    "/mcp".clone_into(&mut elsewhere.path);
    assert_eq!(respond(&elsewhere).status, 405);
    let without_oauth = McpHttpSettings {
        allowed_origins: &[],
        oauth: None,
    };
    assert_eq!(respond_with(&metadata_get(), &without_oauth).status, 405);
}

#[test]
fn no_credentials_answers_401_naming_the_scope_the_operation_needs() {
    let response = respond(&post(&rename()));
    assert_eq!(response.status, 401);
    assert_eq!(response.body, Vec::<u8>::new());
    assert_eq!(
        challenge(&response),
        Some(&*format!(
            "Bearer resource_metadata=\"{METADATA_URL}\", scope=\"documents:write\""
        ))
    );
}

#[test]
fn no_credentials_for_an_operation_without_scopes_names_none() {
    let list = request(&id(3), "tools/list", json!({}));
    let response = respond(&post(&list));
    assert_eq!(response.status, 401);
    assert_eq!(
        challenge(&response),
        Some(&*format!("Bearer resource_metadata=\"{METADATA_URL}\""))
    );
}

#[test]
fn a_token_that_does_not_hold_answers_401_with_invalid_token() {
    let response = respond(&bearing("forged", post(&get_version())));
    assert_eq!(response.status, 401);
    assert_eq!(
        challenge(&response),
        Some(&*format!(
            "Bearer resource_metadata=\"{METADATA_URL}\", error=\"invalid_token\""
        ))
    );
}

#[test]
fn a_notification_without_credentials_answers_401() {
    let notification =
        json!({ "jsonrpc": "2.0", "method": "notifications/cancelled", "params": {} });
    let response = respond(&post(&notification));
    assert_eq!(response.status, 401);
    assert_eq!(
        challenge(&response),
        Some(&*format!("Bearer resource_metadata=\"{METADATA_URL}\""))
    );
}

#[test]
fn enough_scopes_answer_200() {
    let response = respond(&bearing("read-token", post(&get_version())));
    assert_eq!(response.status, 200);
    assert_eq!(
        respond(&bearing("write-token", post(&rename()))).status,
        200
    );
}

#[test]
fn too_few_scopes_answer_403_naming_the_missing_scope_with_a_plain_or_base64_name() {
    let expected = format!(
        "Bearer error=\"insufficient_scope\", scope=\"documents:write\", \
         resource_metadata=\"{METADATA_URL}\""
    );
    let plain = respond(&bearing("read-token", post(&rename())));
    assert_eq!(plain.status, 403);
    assert_eq!(challenge(&plain), Some(&*expected));
    let encoded = with_header(
        without_name(post(&rename())),
        "Mcp-Name",
        "=?base64?cmVuYW1lLWRvY3VtZW50?=",
    );
    let response = respond(&bearing("read-token", encoded));
    assert_eq!(response.status, 403);
    assert_eq!(challenge(&response), Some(&*expected));
}

#[test]
fn a_forbidden_answers_403_with_no_challenge() {
    let response = block_on(document_mcp::answer_http(
        &post(&get_version()),
        &SETTINGS,
        &Archive::default(),
        async |_request: &McpHttpRequest, _call: McpCall| -> Result<(), McpDenied> {
            Err(McpDenied::Forbidden)
        },
    ));
    assert_eq!(response.status, 403);
    assert_eq!(challenge(&response), None);
}

#[test]
fn without_oauth_every_denial_answers_its_status_with_no_challenge() {
    let settings = McpHttpSettings {
        allowed_origins: &[ORIGIN],
        oauth: None,
    };
    for (denied, status) in [
        (McpDenied::NoValidCredentials { scope: None }, 401),
        (
            McpDenied::NoValidCredentials {
                scope: Some("documents:read".to_owned()),
            },
            401,
        ),
        (
            McpDenied::InsufficientScope {
                scope: "documents:write".to_owned(),
            },
            403,
        ),
        (McpDenied::Forbidden, 403),
    ] {
        let response = block_on(document_mcp::answer_http(
            &bearing("forged", post(&get_version())),
            &settings,
            &Archive::default(),
            async |_request: &McpHttpRequest, _call: McpCall| -> Result<(), McpDenied> {
                Err(denied.clone())
            },
        ));
        assert_eq!(response.status, status, "{denied:?}");
        assert_eq!(challenge(&response), None, "{denied:?}");
        assert_eq!(response.body, Vec::<u8>::new(), "{denied:?}");
    }
}

#[test]
fn an_origin_that_is_not_allowed_is_refused_ahead_of_admit_with_no_challenge() {
    let asked = with_header(post(&get_version()), "Origin", "https://evil.example.com");
    let response = respond(&asked);
    assert_eq!(response.status, 403);
    assert_eq!(challenge(&response), None);
}

#[test]
fn admit_reads_the_method_and_the_name_ahead_of_the_rule_that_holds_them_to_the_body() {
    let seen = Mutex::new(Vec::new());
    let archive = Archive::default();
    let asked = with_header(
        without_name(post(&get_version())),
        "Mcp-Name",
        "rename-document",
    );
    let response = block_on(document_mcp::answer_http(
        &asked,
        &SETTINGS,
        &archive,
        async |request: &McpHttpRequest, _call: McpCall| -> Result<(), McpDenied> {
            seen.lock().unwrap().push((
                request.header("mcp-method").map(str::to_owned),
                request.mcp_name(),
            ));
            Ok(())
        },
    ));
    assert_eq!(
        *seen.lock().unwrap(),
        [(
            Some("tools/call".to_owned()),
            Some("rename-document".to_owned())
        )]
    );
    assert_eq!(response.status, 400);
    let sent = body_of(&response);
    assert_eq!(sent["error"]["code"], HEADER_MISMATCH);
    assert_error("HeaderMismatchError", &sent);
    assert!(archive.reached.lock().unwrap().is_empty());
}

#[test]
fn a_name_that_names_a_read_tool_over_a_write_body_reaches_no_handler() {
    let archive = Archive::default();
    let asked = bearing(
        "read-token",
        with_header(without_name(post(&rename())), "Mcp-Name", "get-version"),
    );
    let response = block_on(document_mcp::answer_http(
        &asked,
        &SETTINGS,
        &archive,
        admit_scopes,
    ));
    assert_eq!(response.status, 400);
    assert_eq!(body_of(&response)["error"]["code"], HEADER_MISMATCH);
    assert!(archive.reached.lock().unwrap().is_empty());
}

#[test]
fn the_helpers_read_the_authorization_header() {
    let asked = |value: &str| with_header(post(&get_version()), "authorization", value);
    assert_eq!(asked("Bearer abc").bearer_token(), Some("abc"));
    assert_eq!(asked("bEaReR abc").bearer_token(), Some("abc"));
    assert_eq!(asked("Basic abc").bearer_token(), None);
    assert_eq!(asked("Bearer ").bearer_token(), None);
    assert_eq!(post(&get_version()).bearer_token(), None);
    assert_eq!(
        post(&get_version()).mcp_name().as_deref(),
        Some("get-version")
    );
}

#[test]
fn mcp_scopes_answers_what_serve_routes() {
    let scopes = DocumentServiceSchema::mcp_scopes;
    assert_eq!(
        scopes("tools/call", Some("get-version")),
        ["documents:read"]
    );
    assert_eq!(
        scopes("tools/call", Some("rename-document")),
        ["documents:write"]
    );
    assert_eq!(
        scopes("tools/call", Some("touch-document")),
        ["documents:read", "documents:touch"]
    );
    assert_eq!(
        scopes("tools/call", Some("lookup-document")),
        ["documents:read"]
    );
    assert_eq!(scopes("tools/call", Some("lookup")), [] as [&str; 0]);
    assert_eq!(scopes("tools/call", Some("get-tagged")), [] as [&str; 0]);
    assert_eq!(scopes("tools/call", None), [] as [&str; 0]);
    assert_eq!(scopes("tools/list", None), [] as [&str; 0]);
    assert_eq!(scopes("tools/list", Some("get-version")), [] as [&str; 0]);
}

#[test]
fn mcp_scopes_answers_the_scopes_of_a_resource_for_a_fixed_uri_and_a_template_uri() {
    let scopes = DocumentServiceSchema::mcp_scopes;
    assert_eq!(
        scopes("resources/read", Some("guide://configure-workspace")),
        ["guides:read"]
    );
    for uri in ["workspace://documents/doc-1", "workspace://documents/a b"] {
        assert_eq!(
            scopes("resources/read", Some(uri)),
            ["documents:read", "files:read"],
            "{uri}"
        );
    }
    for uri in [
        "workspace://documents/",
        "workspace://documents/doc-1/pages",
        "workspace://other/doc-1",
        "guide://configure-workspace/more",
    ] {
        assert_eq!(
            scopes("resources/read", Some(uri)),
            [] as [&str; 0],
            "{uri}"
        );
    }
    assert_eq!(scopes("resources/read", None), [] as [&str; 0]);
}

#[test]
fn mcp_scopes_answers_none_for_the_listings_and_the_completion_that_declare_none() {
    let scopes = DocumentServiceSchema::mcp_scopes;
    for method in [
        "resources/list",
        "resources/templates/list",
        "completion/complete",
    ] {
        assert_eq!(scopes(method, None), [] as [&str; 0], "{method}");
        assert_eq!(
            scopes(method, Some("workspace://documents/{document_id}")),
            [] as [&str; 0],
            "{method}"
        );
    }
}

#[test]
fn a_resource_read_without_the_scope_answers_403_insufficient_scope() {
    let read = |uri: &str| post(&request(&id(1), "resources/read", json!({ "uri": uri })));
    for (uri, needed) in [
        ("guide://configure-workspace", "guides:read"),
        ("workspace://documents/doc-1", "documents:read files:read"),
    ] {
        let response = respond(&bearing("read-token", read(uri)));
        assert_eq!(response.status, 403, "{uri}");
        assert_eq!(
            challenge(&response),
            Some(&*format!(
                "Bearer error=\"insufficient_scope\", scope=\"{needed}\", \
                 resource_metadata=\"{METADATA_URL}\""
            )),
            "{uri}"
        );
    }
    let encoded = with_header(
        without_name(read("guide://configure-workspace")),
        "Mcp-Name",
        "=?base64?Z3VpZGU6Ly9jb25maWd1cmUtd29ya3NwYWNl?=",
    );
    assert_eq!(respond(&bearing("read-token", encoded)).status, 403);
}

#[test]
fn a_resource_read_with_the_scopes_reaches_the_handler() {
    let read = post(&request(
        &id(1),
        "resources/read",
        json!({ "uri": "workspace://documents/doc-1" }),
    ));
    let response = respond(&bearing("file-token", read));
    assert_eq!(response.status, 200);
    let sent = body_of(&response);
    assert_valid("JSONRPCResultResponse", &sent);
    assert_eq!(sent["result"]["contents"][0]["blob"], "JVBERi0=");
}

#[test]
fn the_futures_of_answer_http_are_send() {
    fn is_send<Held>(held: Held) -> Held
    where
        Held: Send,
    {
        held
    }
    let archive = Archive::default();
    let asked = post(&get_version());
    drop(is_send(document_mcp::answer_http(
        &asked,
        &SETTINGS,
        &archive,
        admit_scopes,
    )));
}

#[test]
fn the_second_service_denies_and_reads_the_name_as_the_first_does() {
    let archive = Archive::default();
    let asked = post(&call(
        &id(1),
        "get-balance",
        &json!({ "value": "acct-1", "x-ledger": "main" }),
    ));
    let settings = billing_mcp::McpHttpSettings {
        allowed_origins: &[ORIGIN],
        oauth: None,
    };
    for (denied, status) in [
        (
            billing_mcp::McpDenied::NoValidCredentials { scope: None },
            401,
        ),
        (
            billing_mcp::McpDenied::InsufficientScope {
                scope: "billing:read".to_owned(),
            },
            403,
        ),
        (billing_mcp::McpDenied::Forbidden, 403),
    ] {
        let response = block_on(billing_mcp::answer_http(
            &billing_request(&asked),
            &settings,
            &archive,
            async |request: &billing_mcp::McpHttpRequest, _call: billing_mcp::McpCall| {
                assert_eq!(request.mcp_name().as_deref(), Some("get-balance"));
                Err::<(), _>(denied.clone())
            },
        ));
        assert_eq!(response.status, status, "{denied:?}");
    }
}

fn billing_request(from: &McpHttpRequest) -> billing_mcp::McpHttpRequest {
    billing_mcp::McpHttpRequest {
        body: from.body.clone(),
        headers: from.headers.clone(),
        method: from.method.clone(),
        path: from.path.clone(),
    }
}
