//! The requests every emitted `mcp` server is sent to measure its authorization, and the
//! stand-in decisions its `admit` makes. A token is a name a stand-in knows; tixschema checks none.

use serde_json::{Value, json};

/// What the Rust dispatcher of a service, expanded into `$module`, answers to a `tools/list`
/// alone, beside itself on an endpoint, and over HTTP: the three texts and the status.
macro_rules! rust_serves_alike {
    ($module:ident, $schema:ty, $svc:expr, $drive:path) => {{
        struct Sink;
        impl $module::McpEventWriter for Sink {
            fn write(
                &self,
                _message: String,
            ) -> impl ::core::future::Future<Output = Result<(), $module::McpClientGone>> + Send
            {
                ::core::future::ready(Ok(()))
            }
        }
        let row = $crate::mcp_auth_rows::list_row();
        let endpoint = $module::McpEndpoint::new(&[<$schema>::MCP_NAMES], &[$module::catalog]);
        let alone = $drive($module::answer(
            &row.body,
            &$svc,
            |call| call.progress(0.0_f64, None, None),
            &Sink,
            &$module::McpInFlight::default(),
        ));
        let served = $drive($module::serve(
            &row.body,
            $module::SERVER_NAME,
            $module::INSTRUCTIONS,
            &endpoint,
            |call| call.progress(0.0_f64, None, None),
            $module::McpHost {
                in_flight: &$module::McpInFlight::default(),
                writer: &Sink,
            },
            &|ctx: &(), _part, method: &str, params: &Value| {
                Box::pin(async move { $module::route(method, params, &$svc, ctx).await })
            },
        ));
        let asked = $module::McpHttpRequest {
            body: row.body.clone().into_bytes(),
            headers: row.headers.clone(),
            method: row.method.to_owned(),
            path: row.path.to_owned(),
        };
        let settings = $module::McpHttpSettings {
            allowed_origins: &[],
            oauth: None,
        };
        let over_http = $drive($module::serve_http(
            &asked,
            &settings,
            $module::SERVER_NAME,
            $module::INSTRUCTIONS,
            &endpoint,
            async |_request: &$module::McpHttpRequest,
                   _call: $module::McpCall|
                   -> Result<(), $module::McpDenied> { Ok(()) },
            &|ctx: &(), _part, method: &str, params: &Value| {
                Box::pin(async move { $module::route(method, params, &$svc, ctx).await })
            },
        ));
        (alone, served, over_http.status)
    }};
}

/// What the Rust dispatcher of a service, expanded into `$module`, answers to `$row`: its status,
/// its headers and its body. `$drive` runs a future that is ready on its first poll.
macro_rules! rust_auth_answer {
    ($module:ident, $schema:ty, $svc:expr, $row:expr, $drive:path) => {{
        use $crate::mcp_auth_rows::{
            AUTHORIZATION_SERVER, Decision, METADATA_URL, ORIGIN, RESOURCE, SCOPES_SUPPORTED,
            Scheme, decide_keys, decide_oauth,
        };
        let row: &$crate::mcp_auth_rows::AuthRow = $row;
        let asked = $module::McpHttpRequest {
            body: row.body.clone().into_bytes(),
            headers: row.headers.clone(),
            method: row.method.to_owned(),
            path: row.path.to_owned(),
        };
        let oauth = $module::McpOAuth {
            authorization_servers: &[AUTHORIZATION_SERVER],
            resource: RESOURCE,
            resource_metadata_url: METADATA_URL,
            scopes_supported: &[SCOPES_SUPPORTED],
        };
        let settings = $module::McpHttpSettings {
            allowed_origins: &[ORIGIN],
            oauth: (row.scheme == Scheme::Oauth).then_some(oauth),
        };
        let denied = |decision: Decision| -> Result<(), $module::McpDenied> {
            match decision {
                Decision::Admit => Ok(()),
                Decision::Forbidden => Err($module::McpDenied::Forbidden),
                Decision::InsufficientScope(scope) => {
                    Err($module::McpDenied::InsufficientScope { scope })
                }
                Decision::NoValidCredentials(scope) => {
                    Err($module::McpDenied::NoValidCredentials { scope })
                }
            }
        };
        let answered = match row.scheme {
            Scheme::Oauth => $drive($module::answer_http(
                &asked,
                &settings,
                &$svc,
                async |request: &$module::McpHttpRequest, _call: $module::McpCall| {
                    let name = request.mcp_name();
                    let method = request.header("mcp-method").unwrap_or_default();
                    let needed = <$schema>::mcp_scopes(method, name.as_deref());
                    denied(decide_oauth(request.bearer_token(), needed))
                },
            )),
            Scheme::Keys => $drive($module::answer_http(
                &asked,
                &settings,
                &$svc,
                async |request: &$module::McpHttpRequest, _call: $module::McpCall| {
                    denied(decide_keys(
                        request.header("x-api-key"),
                        request.header("x-api-secret"),
                    ))
                },
            )),
        };
        (
            answered.status,
            answered.headers,
            String::from_utf8(answered.body).unwrap(),
        )
    }};
}

pub const AUTHORIZATION_SERVER: &str = "https://auth.example.com";
pub const METADATA_PATH: &str = "/.well-known/oauth-protected-resource/mcp";
pub const METADATA_URL: &str = "https://mcp.example.com/.well-known/oauth-protected-resource/mcp";
pub const ORIGIN: &str = "https://app.example.com";
pub const RESOURCE: &str = "https://mcp.example.com/mcp";
pub const SCOPES_SUPPORTED: &str = "documents:read";

/// The method and name pairs `mcp_scopes` is asked.
pub const SCOPE_QUERIES: [(&str, Option<&str>); 7] = [
    ("tools/call", Some("get-version")),
    ("tools/call", Some("rename-document")),
    ("tools/call", Some("touch-document")),
    ("tools/call", Some("lookup")),
    ("tools/call", None),
    ("tools/list", None),
    ("tools/list", Some("get-version")),
];

const EVIL: &str = "https://evil.example.com";

pub struct AuthRow {
    pub body: String,
    pub headers: Vec<(String, String)>,
    pub method: &'static str,
    pub name: &'static str,
    pub path: &'static str,
    pub scheme: Scheme,
}

/// What a stand-in `admit` answers.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Decision {
    Admit,
    Forbidden,
    InsufficientScope(String),
    NoValidCredentials(Option<String>),
}

/// Which `admit` and which settings a request meets.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Scheme {
    /// No OAuth settings, and an `admit` that checks two headers.
    Keys,
    /// OAuth settings, and an `admit` that checks a bearer token against the scopes the call needs.
    Oauth,
}

impl Scheme {
    pub const fn word(self) -> &'static str {
        match self {
            Self::Keys => "keys",
            Self::Oauth => "oauth",
        }
    }
}

pub fn decide_oauth(bearer: Option<&str>, needed: &[&str]) -> Decision {
    let Some(token) = bearer else {
        let scope = (!needed.is_empty()).then(|| needed.join(" "));
        return Decision::NoValidCredentials(scope);
    };
    let held = match token {
        "read-token" => "documents:read",
        "write-token" => "documents:read documents:write",
        "forbidden-token" => return Decision::Forbidden,
        _ => return Decision::NoValidCredentials(None),
    };
    if needed
        .iter()
        .any(|scope| !held.split(' ').any(|have| have == *scope))
    {
        return Decision::InsufficientScope(needed.join(" "));
    }
    Decision::Admit
}

pub fn decide_keys(sent_key: Option<&str>, sent_secret: Option<&str>) -> Decision {
    let (Some(key), Some(secret)) = (sent_key, sent_secret) else {
        return Decision::NoValidCredentials(None);
    };
    match (key, secret) {
        ("admin-key", "admin-secret") => Decision::Admit,
        _ => Decision::Forbidden,
    }
}

fn meta() -> Value {
    json!({
        "io.modelcontextprotocol/protocolVersion": "2026-07-28",
        "io.modelcontextprotocol/clientCapabilities": {},
    })
}

fn call(number: u64, tool: &str, arguments: &Value) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": number,
        "method": "tools/call",
        "params": { "name": tool, "arguments": arguments, "_meta": meta() },
    })
}

fn get_version() -> Value {
    call(1, "get-version", &json!({ "document_id": "doc-1" }))
}

fn rename() -> Value {
    call(
        2,
        "rename-document",
        &json!({ "documentId": "doc-1", "title": "Plan" }),
    )
}

fn touch() -> Value {
    call(
        3,
        "touch-document",
        &json!({ "documentId": "doc-1", "reader": "reviewer-7" }),
    )
}

fn list() -> Value {
    json!({ "jsonrpc": "2.0", "id": 4, "method": "tools/list", "params": { "_meta": meta() } })
}

fn notification() -> Value {
    json!({ "jsonrpc": "2.0", "method": "notifications/cancelled", "params": {} })
}

fn headers_of(message: &Value, tool: Option<&str>) -> Vec<(String, String)> {
    let mut headers = vec![
        ("Content-Type".to_owned(), "application/json".to_owned()),
        ("MCP-Protocol-Version".to_owned(), "2026-07-28".to_owned()),
        (
            "Mcp-Method".to_owned(),
            message["method"].as_str().unwrap().to_owned(),
        ),
    ];
    if let Some(named) = tool.or_else(|| message["params"]["name"].as_str()) {
        headers.push(("Mcp-Name".to_owned(), named.to_owned()));
    }
    headers
}

fn row(name: &'static str, scheme: Scheme, message: &Value) -> AuthRow {
    AuthRow {
        body: message.to_string(),
        headers: headers_of(message, None),
        method: "POST",
        name,
        path: "/mcp",
        scheme,
    }
}

fn with(mut row: AuthRow, name: &str, value: &str) -> AuthRow {
    row.headers.push((name.to_owned(), value.to_owned()));
    row
}

fn named(mut row: AuthRow, value: &str) -> AuthRow {
    row.headers.retain(|(name, _)| name != "Mcp-Name");
    row.headers.push(("Mcp-Name".to_owned(), value.to_owned()));
    row
}

fn bearing(row: AuthRow, token: &str) -> AuthRow {
    with(row, "Authorization", &format!("Bearer {token}"))
}

fn metadata(name: &'static str, scheme: Scheme, path: &'static str) -> AuthRow {
    AuthRow {
        body: String::new(),
        headers: Vec::new(),
        method: "GET",
        name,
        path,
        scheme,
    }
}

/// A `tools/list` over HTTP.
pub fn list_row() -> AuthRow {
    row("a list", Scheme::Keys, &list())
}

fn oauth_rows() -> Vec<AuthRow> {
    let oauth = Scheme::Oauth;
    let mut query = metadata(
        "the metadata document with a query and an origin that is not allowed",
        oauth,
        "/.well-known/oauth-protected-resource/mcp?probe=1",
    );
    query.headers.push(("Origin".to_owned(), EVIL.to_owned()));
    vec![
        metadata("a GET at the metadata path", oauth, METADATA_PATH),
        query,
        metadata("a GET elsewhere", oauth, "/mcp"),
        row("no credentials for a read tool", oauth, &get_version()),
        row("no credentials for a write tool", oauth, &rename()),
        row("no credentials for a list", oauth, &list()),
        row("a notification with no credentials", oauth, &notification()),
        bearing(row("a forged token", oauth, &get_version()), "forged"),
        bearing(row("enough scopes", oauth, &get_version()), "read-token"),
        bearing(row("a write token", oauth, &rename()), "write-token"),
        bearing(row("too few scopes", oauth, &rename()), "read-token"),
        bearing(
            named(
                row("too few scopes, a base64 name", oauth, &rename()),
                "=?base64?cmVuYW1lLWRvY3VtZW50?=",
            ),
            "read-token",
        ),
        bearing(row("too few scopes of two", oauth, &touch()), "read-token"),
        bearing(
            row("a forbidden caller", oauth, &get_version()),
            "forbidden-token",
        ),
        with(
            row(
                "an origin that is not allowed, no credentials",
                oauth,
                &get_version(),
            ),
            "Origin",
            EVIL,
        ),
        bearing(
            with(
                row("an allowed origin with credentials", oauth, &get_version()),
                "Origin",
                ORIGIN,
            ),
            "read-token",
        ),
        bearing(
            named(
                row(
                    "a name that names a read tool over a write body",
                    oauth,
                    &rename(),
                ),
                "get-version",
            ),
            "read-token",
        ),
        named(
            row(
                "a name that names a write tool, no credentials",
                oauth,
                &get_version(),
            ),
            "rename-document",
        ),
    ]
}

fn key_rows() -> Vec<AuthRow> {
    let keys = Scheme::Keys;
    let keyed = |name, key: &str, secret: &str| {
        with(
            with(row(name, keys, &get_version()), "x-api-key", key),
            "x-api-secret",
            secret,
        )
    };
    vec![
        metadata(
            "a GET at the metadata path with no OAuth",
            keys,
            METADATA_PATH,
        ),
        row("no headers", keys, &get_version()),
        with(
            row("a key without a secret", keys, &get_version()),
            "x-api-key",
            "admin-key",
        ),
        keyed("an unknown key", "unknown", "unknown"),
        keyed(
            "a key without the permission",
            "member-key",
            "member-secret",
        ),
        keyed("the right key", "admin-key", "admin-secret"),
        with(
            keyed(
                "the right key from the same origin",
                "admin-key",
                "admin-secret",
            ),
            "Origin",
            ORIGIN,
        ),
        with(
            keyed(
                "the right key from another origin",
                "admin-key",
                "admin-secret",
            ),
            "Origin",
            EVIL,
        ),
    ]
}

pub fn auth_rows() -> Vec<AuthRow> {
    let mut rows = oauth_rows();
    rows.extend(key_rows());
    rows
}
