#### The `mcp` Transport

`#[service_schema(transports = ["mcp"])]` asks for a Model Context Protocol server: each operation becomes one tool (unless a resource flag below makes it something else), listed in the order the trait declares them, and the trait's rustdoc becomes the server's `instructions`. It sits beside the other transports (`transports = ["mcp", "ws_rpc"]`), and every operation keeps its handler, message, success and error on each of them. The macro writes the tool list, a Rust dispatcher macro and a Rust client macro; the servers and clients of the other languages are separate parts of the transport.

```rust,ignore
/// Reads and renames documents in the caller's workspace.
#[service_schema(transports = ["mcp"])]
pub trait DocumentService<Ctx> {
    /// Answers the current version of one document.
    #[service_schema_op(mcp(title = "Current Document Version", read_only))]
    async fn get_version(&self, ctx: &Ctx, req: GetVersionRequest) -> Result<VersionResponse, VersionError>;

    /// Answers the current version of the document the identifier names.
    #[service_schema_op(mcp(name = "lookup-document"))]
    async fn lookup(&self, ctx: &Ctx, id: DocumentId) -> Result<VersionResponse, VersionError>;
}

assert_eq!(DocumentServiceSchema::mcp_tools()[0]["name"], "get-version");
assert_eq!(DocumentServiceSchema::mcp_tools()[0]["annotations"]["readOnlyHint"], true);
```

A tool takes its fields from the operation:

| Tool field | Taken from |
|---|---|
| `name` | `mcp(name = "...")`, otherwise the operation's wire name |
| `title` | `mcp(title = "...")`, otherwise the method's name in Title Case (`get_version` becomes `Get Version`) |
| `description` | the method's rustdoc, each `///` line with the space after it removed, joined with newlines |
| `inputSchema` | the arguments object, built from the JSON Schema of the message |
| `outputSchema` | the JSON Schema of the success, left out for a unit success and a one-way operation |
| `annotations` | `read_only`, `destructive = <bool>`, `idempotent` and `open_world = <bool>`: the `readOnlyHint`, `destructiveHint`, `idempotentHint` and `openWorldHint` written |
| `icons` | `icon = "<url>"` |

`header(<argument> = "<Header-Name>")` marks the property of that argument `x-mcp-header`, for an argument of a message the macro declares or one a `header_in` binds. Every other key in `mcp(...)`, and an empty `title`, `name` or `icon`, is refused naming the operation, whatever transports the service asks for.

Each `inputSchema` and `outputSchema` is a document of its own, with its own `$defs`: a named type used two or more times in it is written there once and referenced at each use, and one used once is written in place, as [Nested Types](nested-recursive-and-aliased-types.md#nested-types) says.

**The arguments object is always a JSON object.** A message that serde writes as an object is the arguments object itself. One that serde writes as anything else (a branded newtype over a `String`, a plain enum, a list) is wrapped as the property `value` of an object that allows no other property. Each `header_in` binding adds one property named by the header as written, and a message that is an operation's one argument is wrapped whenever the operation binds one. A `header_out` value is a property of the `outputSchema` beside the success, under `value`.

`#[model_schema()]` writes `needs_wrapper()` on every model type and on its `{type}_schema::Schema`, a `const fn` that answers `true` where serde writes no JSON object, and `<Service>Schema::wrapped_{operation}()` answers it for each operation. A type that writes what fills its type parameter answers `true`, which is always valid. `<Service>Schema::MCP_NAMES` lists `("tool", "<name>")` for every tool, `("resource", "<uri>")` for every resource and `("resource template", "<uri template>")` for every template, and `<Service>Schema::MCP_INSTRUCTIONS` holds the trait's rustdoc.

On a service that asks for `mcp`, the macro refuses, naming the operation: a method without rustdoc, a tool name over 128 characters or holding a character outside `A-Z`, `a-z`, `0-9`, `_`, `-` and `.`, two tools of one name, a `header_out` or `error_header_out` named `value`, a `header_in` named `value` on an operation whose message is its one argument, a `header_in` named like a key of the generated message, and an operation only `http_rest` carries (a `body = "stream"` reply or a multipart file part). Every model type a message, a `header_in` argument, a `header_out` value or a success names lists `enable = ["jsonschema"]`, as for `openapi`; the macro lists it on the messages it generates. The tool list is built at the trait's scope, so a trait that names a type declared in another module imports that module's `{type}_schema` beside the type.

**Serving tools.** `{service}_mcp_dispatcher!()` goes in a module of its own file, as the other dispatcher macros do, and emits `SERVER_NAME`, `INSTRUCTIONS`, `catalog()`, `route`, `McpEndpoint`, `serve`, `answer`, `McpHttpRequest`, `McpHttpResponse`, `McpHttpStream`, `McpEventWriter`, `serve_http` and `answer_http`. A crate that invokes it names `serde`, `serde_json` and `tracing`, and reaches the service as `$crate::<Service>Schema`, so a service declared in a submodule re-exports its `<Service>Schema` at the crate root. The application owns the stdio loop and the HTTP server:

```rust,ignore
// stdio: one task per line, so a cancellation is read while a handler runs
let stdout = Lines::default();
let in_flight = document_mcp::McpInFlight::default();
while let Some(line) = lines.next_line().await? {
    let (stdout, in_flight, store) = (stdout.clone(), in_flight.clone(), store.clone());
    running.spawn_local(async move {
        let reply = document_mcp::answer(&line, &*store, |call| RequestContext { call }, &stdout, &in_flight).await;
        if let Some(reply) = reply {
            stdout.write(reply).await.ok();
        }
    });
}

// HTTP: build `McpHttpRequest::new(method, path, headers, body)` from the framework's request.
// An event-stream answer is written to the stream the host implements, and the call answers
// `None`; any other answer is a `McpHttpResponse`, read through `status()`, `headers()` and
// `body()`.
let answered = document_mcp::answer_http(&request, &settings, &stream, &documents, admit).await;
```

**Streaming over HTTP.** `McpHttpStream` is `McpEventWriter` plus `begin(status, headers)`, which the dispatcher calls once, before the first event, with `200` and the `Content-Type: text/event-stream` and `X-Accel-Buffering: no` headers. `write` then takes the text of each event, framed (`event: message\ndata: ...\n\n`), and of each comment line `:\n\n` that keeps the stream open, as soon as the transport writes it; a call that answers `Err(McpClientGone::new())` marks the request cancelled (`McpClientGone` is `#[non_exhaustive]`, so a writer in another crate builds it with `new()`), and a stream that cannot `begin` cancels it before its handler runs. When the call answers `None` the stream has received everything and the host ends its response. A host wires the stream to its framework's streaming body, here Axum:

```rust,ignore
struct Sse {
    events: mpsc::Sender<Result<String, Infallible>>,
    head: Mutex<Option<oneshot::Sender<(u16, Vec<(String, String)>)>>>,
}

impl document_mcp::McpEventWriter for Sse {
    async fn write(&self, event: String) -> Result<(), document_mcp::McpClientGone> {
        self.events.send(Ok(event)).await.map_err(|_| document_mcp::McpClientGone::new())
    }
}

impl document_mcp::McpHttpStream for Sse {
    async fn begin(&self, status: u16, headers: &[(String, String)]) -> Result<(), document_mcp::McpClientGone> {
        let head = self.head.lock().unwrap().take();
        head.ok_or(document_mcp::McpClientGone::new())?
            .send((status, headers.to_vec()))
            .map_err(|_| document_mcp::McpClientGone::new())
    }
}

async fn mcp(State(app): State<App>, request: Request) -> Response {
    let (events, body) = mpsc::channel(16);
    let (began, head) = oneshot::channel();
    let stream = Sse { events, head: Mutex::new(Some(began)) };
    let asked = mcp_request(request).await;
    let answering = tokio::spawn(async move {
        document_mcp::answer_http(&asked, &SETTINGS, &stream, &*app.documents, admit).await
    });
    tokio::select! {
        Ok((status, headers)) = head => sse_response(status, headers, Body::from_stream(ReceiverStream::new(body))),
        answered = answering => one_object_response(answered.unwrap().unwrap()),
    }
}
```

**Progress and cancellation.** `answer` and `serve` take `ctx`, a closure that builds the handler's `Ctx` from the `McpCall` of the request, a writer (`McpEventWriter`, whose `write` answers `Err(McpClientGone::new())` once the client has gone) and a `McpInFlight` table; `serve` takes the writer and the table together as `McpHost::new(in_flight, writer)`. Every request a service answers (`tools/call`, `resources/read`, `resources/list`, `prompts/get`, `completion/complete`, `skills/get`, the three task methods and `subscriptions/listen`) is built a `Ctx` this way. A handler calls `ctx.call.progress(1.0, Some(3.0), Some("exported 1 of 3 files"))`, which queues `notifications/progress` where the request carries `_meta.progressToken` and does nothing otherwise; the transport writes each between two polls of the handler, naming no async runtime. A failed write marks the call cancelled, and so does `notifications/cancelled` on stdio, which finds the request in `McpInFlight` by `requestId`; a handler reads `ctx.call.cancelled()` and stops when it chooses, and the transport drops the answer and writes nothing more for that request. A host that waits for each answer before it reads the next line never reads the cancellation, so it starts each `answer` as its own task. Over HTTP, `admit` receives the call, and a request with a `progressToken` answers an event stream written to the `McpHttpStream` as it happens: `begin`, one event per notification between two polls of the handler, then the answer; without a token the answer is returned as one JSON object and `begin` is never called. `serve`'s `route` receives the `Ctx` that `ctx` built, as `serve_http`'s receives the one `admit` answered.

**Subscriptions.** A host keeps an old copy of a list or a resource until its `ttlMs` ends, unless the server announces the change. `answer_listening`, `serve_listening`, `answer_http_listening` and `serve_http_listening` are `answer`, `serve`, `answer_http` and `serve_http` for an application that hands in its source of changes, `changes: &mut impl AsyncFnMut() -> Option<McpChange>`; `serve_listening` and `serve_http_listening` take the server's `endpoint`, `instructions` and `name` together as `&McpServer`. `McpChange` is `ResourcesListChanged`, `ResourceUpdated { uri }`, `Task { task_id, state }` and `KeepAlive`. A `subscriptions/listen` writes `notifications/subscriptions/acknowledged` first, carrying the subset of the client's filter the server honors (`resourcesListChanged` and `resourceSubscriptions`, where a service declares resources, and `taskIds`, where the endpoint offers tasks; the tool and prompt lists are fixed at compile time, so `toolsListChanged` and `promptsListChanged` are never acknowledged), then each change the client asked for as `notifications/resources/list_changed`, `notifications/resources/updated` or `notifications/tasks`, each carrying `_meta["io.modelcontextprotocol/subscriptionId"]`, the id of the request. `KeepAlive` writes the comment line `:` on HTTP and nothing on stdio, and `McpEventWriter::keep_alive` is the place to write it elsewhere. When `changes` answers `None` the stream ends with the graceful `SubscriptionsListenResult`; a `notifications/cancelled` for the request, or a failed write, ends it with no answer. `server/discover` adds `"listChanged": true` and `"subscribe": true` to `resources` only through the `_listening` functions. Over HTTP the acknowledgment and each change are written to the `McpHttpStream` as `changes` yields them, and the stream ends, with the graceful answer written last, once `changes` has answered `None`.

The server answers revision 2026-07-28 only: `server/discover`, `tools/list`, `tools/call`, `resources/list`, `resources/read`, `resources/templates/list`, `prompts/list`, `prompts/get`, `completion/complete`, `skills/list` and `skills/get`, `subscriptions/listen` where the host hands in its changes, and `tasks/get`, `tasks/cancel` and `tasks/update` where a service of the endpoint declares a task. It checks each message in this order: text that is no JSON-RPC message answers `-32600` with no `id` member, as the official schema requires; a notification answers nothing; a missing `_meta.io.modelcontextprotocol/protocolVersion` or `clientCapabilities` answers `-32602`; another version answers `-32022` naming both; an unknown method answers `-32601`. A call is read through the operation's `unjudged_` reader and judged by its `validated_` validator, so `preprocess`, bounds and checks run as on every other transport. A success answers `structuredContent` and the same JSON as one text block (an empty `content` list for a unit success and for a one-way operation, once its handler has returned); a declared error answers `isError: true` with its JSON as the text; arguments that are not a JSON object (`undeserializable-payload`), arguments that miss a key or hold a value of the wrong type (`failed-validation`, naming the path of the value), a field the validator refuses (`failed-validation`, naming the field), each naming the tool the caller called, on every server alike, a missing `header_in` property and an unknown tool answer the fault envelope `{"isServiceFault": true, "fault": ...}`, as the text of a tool error or, for an unknown tool and a handler panic, in the `data` of `-32602` and `-32603`. Every answer is compact JSON with the keys of each object in order, and every result carries `serverInfo`, the trait's name and the version of the crate that invoked the macro. `answer` and `answer_http` take a service that is `Sync` and a `Ctx` that is `Send` and `Sync`, and their futures are `Send` where those are.

`serve_http` applies, in order, the Streamable HTTP rules: a `GET` at the path of the metadata document answers it (`405` where `oauth` is `None`), `405` for a method other than `POST`, `403` for an `Origin` not in `settings.allowed_origins`, then `admit`, then `400` with `-32600` for an unreadable body, `202` for a notification, `400` with `-32020` when `MCP-Protocol-Version`, `Mcp-Method` or (for `tools/call`, `resources/read` and `prompts/get`) `Mcp-Name` is missing or differs from the body (header names compared in any case, `Mcp-Name` decoded from `=?base64?...?=`), `400` with `-32602` and `-32022` as above, `404` with `-32601` for an unknown method, and otherwise `200` with `Content-Type: application/json`, or, for a request with a `progressToken` and for a `subscriptions/listen`, the event stream written to the `McpHttpStream`, where `serve_http` answers `None`.

**Authorization.** `McpHttpSettings::new(allowed_origins, oauth)` and an `admit` closure, `async |request: &McpHttpRequest, call: McpCall| -> Result<McpAdmitted<Ctx>, McpDenied>`, replace the origins and the `Ctx` of `answer_http` and `serve_http`; `serve_http`'s `route` receives the `Ctx` `admit` answered as its first argument. tixschema checks no token: `admit` verifies the signature, expiry and audience, or the application's own headers, and answers a `McpAdmitted<Ctx>` or a denial. `McpAdmitted::new(ctx)` answers as the name of the server. `McpAdmitted::named(ctx, name)` names the server for that request: every answer of it carries `name` in `serverInfo`, and an answer that is `"cacheScope": "public"` becomes `"private"`, because another caller gets another name. A denial answers as before. A host whose server name depends on the caller reads the credentials once, in `admit`:

```rust,ignore
let server = workspace_mcp::McpServer::new(&ENDPOINT, Some(INSTRUCTIONS), "Workspace");
let answered = workspace_mcp::serve_http(
    &request,
    &settings,
    &stream,
    &server,
    async |request, _call| {
        let account = check_api_key(request).await?; // McpDenied::NoValidCredentials when it does not hold
        let name = format!("Workspace-{}", account.organization);
        Ok(workspace_mcp::McpAdmitted::named(account, name))
    },
    &route,
)
.await;
```

For the organization `acme`, `server/discover` answers `"name": "Workspace-acme"` and `"cacheScope": "private"`. `request.header(name)` compares names in any case, `request.bearer_token()` reads `Authorization: Bearer`, `request.mcp_name()` decodes `Mcp-Name`, and `request.path()` is the request's path. The structs the dispatcher publishes keep their fields private, as `http_rest`'s do, so a consumer that exports the module passes `clippy::exhaustive_structs`: `McpHttpRequest::new(method, path, headers, body)` with `method()`, `path()`, `headers()` and `body()`; `McpHttpResponse::new(status, headers, body)` with `status()`, `headers()` and `body()`; `McpHttpSettings::new(allowed_origins, oauth)` with `allowed_origins()` and `oauth()`; `McpOAuth::new(authorization_servers, resource, resource_metadata_url, scopes_supported)` with an accessor of each name; `McpServer::new(endpoint, instructions, name)` and `McpHost::new(in_flight, writer)`, each with an accessor of each name; and `McpEndpoint::new(names, catalogs)`, a `const fn`, with `catalogs()`. `McpDenied`, `McpChange` and `McpClientGone` are `#[non_exhaustive]`, and the closure that builds `Ctx` is a named generic parameter, so a module that exports the dispatcher and the client passes the repository's clippy table (`examples/strict_consumer_models` is linted that way). `admit` runs before the body is read, and sees `Mcp-Method` and `Mcp-Name` before the rule that holds them to the body. A denial answers: `NoValidCredentials { scope }` `401` with `WWW-Authenticate: Bearer resource_metadata="..."`, plus `scope="..."` where it names one and `error="invalid_token"` where the request carried a token; `InsufficientScope { scope }` `403` with `Bearer error="insufficient_scope", scope="...", resource_metadata="..."`; `Forbidden` `403` with no challenge. With `oauth: None` every denial answers its status with no challenge. `oauth: Some(McpOAuth { resource, resource_metadata_url, authorization_servers, scopes_supported })` publishes the protected-resource metadata document at the path of `resource_metadata_url`, leaving out `scopes_supported` where it is empty. An operation declares what it needs with `#[service_schema_op(mcp(scopes = ["documents:read"]))]`, and `<Service>Schema::mcp_scopes(method, name)` answers it for the `Mcp-Method` and decoded `Mcp-Name` of a request: a tool for `tools/call` and its name, a resource for `resources/read` and the URI the request names (a fixed URI first, then the templates in declaration order, as `resources/read` routes it), the listing operation for `resources/list`, and a completion for `completion/complete` and its template's URI (`[]` for an operation with none and for `tools/list` and `resources/templates/list`). A scope is refused at expansion when it is empty, holds a space, `"`, `\` or a control character, or is written twice on one operation.

**Several services behind one endpoint.** Each dispatcher publishes `catalog()` and a `route` that answers only what its service owns. An application declares the endpoint as a `const`, numbers its services in the same order in the endpoint and in `route`, and hands both to `serve` or `serve_http`:

```rust,ignore
const WORKSPACE: document_mcp::McpEndpoint = document_mcp::McpEndpoint::new(
    &[DocumentServiceSchema::MCP_NAMES, BillingServiceSchema::MCP_NAMES],
    &[document_mcp::catalog, billing_mcp::catalog],
);

let host = document_mcp::McpHost::new(&in_flight, &stdout);
let reply = document_mcp::serve(&text, "Workspace", None, &WORKSPACE, |call| RequestContext { call }, host, &|ctx: &RequestContext, part, method: &str, params: &Value| {
    let (documents, billing) = (documents.clone(), billing.clone());
    Box::pin(async move {
        match part {
            0 => document_mcp::route(method, params, &documents, ctx).await,
            1 => billing_mcp::route(method, params, &billing, ctx).await,
            _ => None,
        }
    })
})
.await;
```

`route` is a `&dyn Fn` that answers a boxed `Send` future, so the future of `serve` and `serve_http` can be spawned on a multi-threaded runtime (a function bounded `Future + Send + 'static`). The boxed future may borrow only its arguments, so it owns what it routes to: `documents` and `billing` here are handles that clone cheaply (an `Arc`, or a pool), and a store borrowed from the caller's stack is refused with `borrowed for 'static`. `answer` and `answer_http` take the store by reference and need no clone.

`tools/list` concatenates the catalogs in the endpoint's order, `server/discover` answers the union of their capabilities with the extensions under `capabilities.extensions`, and a `tools/call` asks each service's `route` in turn. A tool name two services share fails the build when the endpoint is a `const`: `McpEndpoint::new` panics on a name of one kind two services share (a tool and a prompt of one name do not collide), and rustc prints `error[E0080]: evaluation panicked: get-version`. Resolve it with `mcp(name = "...")` on one of the operations.

**Resources, templates and completion.** An operation with no flag in `mcp(...)` is a tool; a flag makes it something else, and it stays an ordinary operation on every other transport. `resource = "<uri>"` makes it one resource when the URI holds no placeholder (listed by `resources/list`) and a template when it holds a `{placeholder}` (listed by `resources/templates/list`, with the name, rustdoc and `mime_type` written). `list_resources` makes it the service's listing of resources that change, `completes(resource = "<operation>", parameter = "<placeholder>")` makes it the suggestions for one placeholder of a template, and none of the three is a tool:

```rust,ignore
#[service_schema_op(mcp(resource = "guide://configure-organization", mime_type = "text/markdown"))]
async fn configuration_guide(&self, ctx: &Ctx) -> Result<String, DocumentError>;

#[service_schema_op(mcp(list_resources))]
async fn list_documents(&self, ctx: &Ctx, cursor: Option<String>)
    -> Result<document_service_schema::McpResourcePage, DocumentError>;

#[service_schema_op(mcp(resource = "workspace://documents/{document_id}"))]
async fn document_file(&self, ctx: &Ctx, document_id: String)
    -> Result<(Vec<u8>, String), DocumentError>;

#[service_schema_op(mcp(completes(resource = "document_file", parameter = "document_id")))]
async fn suggest_documents(&self, ctx: &Ctx, typed: String, chosen: HashMap<String, String>)
    -> Result<Vec<String>, DocumentError>;
```

- A placeholder names an argument of the operation and takes one non-empty run of text up to the next `/`, as an `http(...)` path placeholder does; `resources/read` builds the message from what the URI captured (a message of several arguments gets one field per placeholder) and reads and judges it as a tool's arguments are. A fixed URI is tried ahead of a template, and templates in declaration order.
- A `String` success answers `text` with the `mime_type` written on the operation; a `(Vec<u8>, String)` success answers `blob` in base64 with the content type the pair holds. A declared error answers `-32602` with the error's JSON in `data`, a message that does not read or validate answers `-32602` with the fault envelope in `data`, and a panic answers `-32603`.
- `McpResourcePage { resources, next_cursor }` and `McpResource { uri, name, title, description, mime_type, size }` are written into the service module beside `ServiceFault`, on a service that asks for `mcp` and declares a listing. The first page of `resources/list` holds the resources with a fixed URI and the first page of the first service that lists; a cursor is `"<service>:<cursor>"`, naming the service's place in the endpoint and its own cursor, and one that names no service answers `-32602`. A service lists its resources through one operation.
- A read and a listing answer `ttlMs: 0` and `cacheScope: "private"`, because the set can vary by the caller; `ttl_ms = <number>` and `cache_scope = "public"` change that for one operation. `resources/templates/list` is fixed at compile time and answers what `tools/list` answers.
- `completion/complete` finds the operation by the template's URI and the placeholder's name, calls it with `typed` from `argument.value` and `chosen` from `context.arguments` (the operation takes those two, in that order), and answers at most 100 values; a longer list answers its first 100 with `total` and `hasMore: true`.
- `server/discover` adds `"resources": {}` where a service declares a resource, a template or a listing, and `"completions": {}` where it declares a completion. The Rust client has no method for these operations, which are not tools.
- The macro refuses, naming the operation: a placeholder that names no argument, two operations with one URI (templates that differ only in their placeholders' names included), a success a resource cannot answer (anything but a `String` or a `(Vec<u8>, String)`), a `completes` that names no resource template of the service or a placeholder it lacks, a listing or a completion of the wrong shape, an operation that is more than one of the three, and a `cache_scope` other than `"public"` or `"private"`.

**Asking the user.** A handler asks by answering an arm of its declared error, and the operation names the arm with `interview(...)`, for a form or for a link. The service macro cannot read the enum, so the generated match on `DeleteError::NeedsConfirmation { .. }` fails to compile where no such arm exists:

```rust
/// Deletes one document after the user confirms it.
#[service_schema_op(mcp(interview(arm = "NeedsConfirmation", answers = confirmation)))]
async fn delete_document(&self, ctx: &Ctx, document_id: String, confirmation: Option<DeleteConfirmation>)
    -> Result<(), DeleteError>;

/// Imports one file from the caller's Drive.
#[service_schema_op(mcp(interview(arm = "NeedsDriveConnection", url)))]
async fn import_from_drive(&self, ctx: &Ctx, file_id: String) -> Result<ImportedDocument, ImportError>;
```

- **A form.** `answers` names an argument of the operation, of type `Option<Form>`, where `Form` lists `enable = ["jsonschema"]`; `tools/list` leaves it out of `inputSchema`. The first call runs the handler with `None`. When the handler answers the arm and the client declared `elicitation`, the call answers `input_required` keyed by the argument's wire name, with the form's exported schema (an `Option` field is a property that is not required, and every type is written in place at each use, with no `$defs`) and the `message` key of the arm's JSON, or the operation's rustdoc where the arm has none. The retried call carries `inputResponses`: an `accept` is read as the form type (an answer that does not read answers the `failed-validation` fault as a tool execution error), judged with the message and passed to the handler as `Some`; a `decline` or `cancel` runs the handler with `None`, and its arm answers as a tool execution error. A client that did not declare `elicitation` receives the arm as the declared error, and so does a generated client, which declares no capabilities and calls again with the answer among its arguments.
- **A link.** The arm holds `url` and `message` as `String`, and `interview(arm = "...", url)` is for a secret or a sign-in to another service, which the specification keeps out of a form. A client that declared `elicitation.url` receives `input_required` in URL mode, keyed by the name serde writes the arm under (`mcp_arm_wire_{operation}()`, so `#[serde(rename)]` and `rename_all` decide it); the retried call runs the handler again, and an arm answered again asks again. A `decline` or `cancel` of the link, and a client without `elicitation.url`, receive the arm as the declared error, whose text holds the link and the message. The page behind the link and the token store are the application's.
- `interview` takes `answers` or `url`, not both, and is refused on a one way operation, on an operation that binds `error_header_out`, and for an `answers` that is no argument of the operation or is no `Option`.
- **A form that is not flat does not build.** `#[model_schema]` writes `mcp_form_value()` (true for an enum whose variants hold no data) and `mcp_form_flat()` (true for a struct whose every field is a `String`, a `bool`, an integer, a float, a `mcp_form_value()` type, a `Vec` of such a type, or an `Option` of any of these; false with a `#[serde(flatten)]` field) on every struct and enum. The service macro asserts `mcp_form_flat()` for the form's type, and rustc prints `error[E0080]: evaluation panicked: `ship_document` asks with `shipping`, whose type `ShippingForm` is not a flat form: MCP takes text, numbers, yes or no, and choices`. A type that is no model type has no `mcp_form_flat()` and fails as well. A field's type is read as the answer recorded when that type was expanded, so declare a form's enums above the form.

**Content.** `mcp(content)` makes a tool answer `content` blocks, with no `outputSchema` and no `structuredContent`. A `(Vec<u8>, String)` success answers an `image` block where the content type starts with `image/` and an `audio` block where it starts with `audio/`; any other content type answers the pair as an operation with no flag does. A `Vec<McpContent>` success answers its blocks in order; `McpContent` (`Text`, `Image`, `Audio`, `ResourceLink`, `Resource`) and `McpResourceContents` (`Text`, `Blob`) are written into the service module, on a service that declares such an operation. The Rust client reads the blocks back into the pair or the list. The macro refuses a `content` operation that is one way, that answers anything else, that binds `header_out`, or that is also a resource, a listing or a completion.

**Asking and content on the TypeScript, Dart and Kotlin servers and every client.** Each of those servers answers the bytes the Rust dispatcher answers to the same requests: the first call, the retried form call with `accept`, `decline` and `cancel`, URL mode, a client that declared no `elicitation`, and the content tools. A handler that asks answers the arm of its declared error as the language writes it (a thrown error in Dart, `Declared` in Kotlin, `{ ok: false, error }` in TypeScript), and the server finds the arm by the tag key and the name serde writes it under, which `#[model_schema]` publishes on the error enum as `mcp_variant_tag("Arm")` and the accessor writes into the text when it runs in your program, so a `#[serde(rename)]` on the arm, a `rename_all` and a tag key other than `errorCode` find the arm the Rust server finds. An error enum with no tag (`External`) holds the arm's members under its name, one tagged with a content key (`#[serde(tag = "kind", content = "data")]`) holds them under that key, which `mcp_variant_tag` answers as its third element (`mcp_arm_content_{operation}()`, empty where the enum has none), and one that is untagged has no arm the build accepts. The form's schema is `mcp_form_{operation}()`, written into the text when the accessor runs in your program. The answers of a retried form call reach the handler through the read the message has anyway, so an answer its type refuses answers a fault naming the argument, `failed-validation` or, where the language reads a wrong type as one, `undeserializable-payload`. A generated client declares no capabilities, so it receives the arm as its declared error and calls again with the argument filled.

A service with a `Vec<McpContent>` success, or a prompt that answers a `Vec<McpPromptMessage>`, also publishes the types that list is written in (`McpContent`, and with it `McpRole` and `McpPromptMessage`), once per bundle: `ts_mcp_content()` (a union of objects with the specification's `type`, the bytes a `Uint8Array`), `dart_mcp_content()` (a sealed `McpContent` with `McpContentText`, `McpContentImage`, `McpContentAudio`, `McpContentResourceLink` and `McpContentResource`; import `dart:convert` and `dart:typed_data`), `kotlin_mcp_content()` (a sealed `McpContent` serialized as the block, the bytes a `List<UByte>`) and `swift_mcp_content()` (an `enum McpContent`, the bytes a `[UInt8]`). A client reads an image or audio block back into the pair (`[Uint8Array, string]`, `(Uint8List, String)`, the tuple class `kotlin_http_client()` declares, `([UInt8], String)`) and the blocks into the list. On every other transport the service asks for (`ws_rpc`, `http_rest`, `amqp_rpc`), an operation answering `Vec<McpContent>` or `Vec<McpPromptMessage>` stays an ordinary operation with the same handler and success, written in the block form the `mcp` transport writes: `{"type": "text", "text": ...}`, `{"type": "image", "data": <base64>, "mimeType": ...}`, `{"type": "audio", ...}`, `{"type": "resource_link", ...}`, `{"type": "resource", "resource": {...}}` and, for a prompt, `{"content": <block>, "role": "user" | "assistant"}`, keys in sorted order. The service module writes `Serialize` and `Deserialize` for `McpContent`, `McpRole` and `McpPromptMessage` in that form, so the Rust dispatchers and clients of those transports need nothing more; in TypeScript the same accessor `ts_mcp_content()` also publishes `McpContent$Schema` and `McpPromptMessage$Schema` (what the clients read a reply with) and `McpContent$Wire` and `McpPromptMessage$Wire` (what the dispatcher writes a success with), and the Dart, Kotlin and Swift accessors read and write that form in their own types. Every bundle of a service with `mcp(list_resources)` includes that accessor beside its clients and servers, since it also publishes the service's page types (below). A tuple success is read by Swift's `swift_http_client()` through an unkeyed-container struct it declares, and answered as the pair. On Kotlin the tuple class is named by its shape, `kotlin_http_client()` declares it once, and `kotlin_ws_client()`, `kotlin_mcp_service()` and `kotlin_mcp_client()` name it, so a program holding the four accessors of a service with a `(Vec<u8>, String)` operation compiles with no edit, and the Kotlin server and client read and answer the pair.

**Prompts.** `mcp(prompt)` makes an operation a prompt, an entry the user picks from the host's menu. Its `name` and `title` follow a tool's rules, its `description` is the method's rustdoc, and `prompts/list` lists it where `tools/list` leaves it out. Each argument is one entry of `arguments`, named by its camelCase name and required unless its type is `Option<String>`; `server/discover` adds `"prompts": {}`. `prompts/get` reads and judges the arguments as a tool's are, and a `String` success answers one `user` message while a `Vec<McpPromptMessage>` answers its messages in order. `McpPromptMessage { role: McpRole, content: McpContent }` and `McpRole` (`User`, `Assistant`) are written into the service module, beside `McpContent`. A prompt's result has no `isError`, so every failure is a JSON-RPC error: `-32602` with the `unknown-operation`, `undeserializable-payload` or `failed-validation` fault in `data`, `-32602` with the declared error's JSON, or `-32603` with the `handler-panic` fault. `completes(prompt = "<operation>", parameter = "<argument>")` makes an operation the suggestions for one argument of a prompt, which `completion/complete` asks for with a `ref/prompt` naming the prompt and the argument by its camelCase name. `prompts/get` carries the `Mcp-Name` header over HTTP, and `scopes` on a prompt, or on its completion, reach `mcp_scopes`. The generated clients have one method per prompt: it sends `prompts/get` with the message's text members, and reads a `String` success from the text of the first message and a list into `Vec<McpPromptMessage>`; a `-32602` whose `data` holds no fault envelope is the declared error. The macro refuses a prompt argument that is not a `String` or an `Option<String>`, a success that is neither a `String` nor a `Vec<McpPromptMessage>`, a one way operation, a header binding, any key beside `prompt` but `title`, `name`, `icon` and `scopes`, a name that breaks a tool name's rules and two prompts of one name.

**Tasks.** `mcp(task(get = "export_status", cancel = "cancel_export", update = "answer_export"))` makes a tool answer the handle of a long job, and the operations it names answer `tasks/get`, `tasks/cancel` and `tasks/update`; `get` is required and the other two are optional. The application starts the work its own way and keeps the task in its own store; tixschema keeps none. The service module declares `McpTask { task_id, created_at, last_updated_at, ttl_ms, poll_interval_ms }` (the times are text the application writes, and `ttl_ms` is always written, `null` for no limit) and `McpTaskState<T>`, an enum of `Working`, `InputRequired { input_requests }`, `Completed { result }`, `Failed { error: McpTaskError { code, message, data } }` and `Cancelled`, each holding its `McpTask`. The start operation answers `Result<McpTask, E>`; the `get` operation takes the task's id as one `String` and answers `Result<McpTaskState<T>, E>`; the `cancel` operation takes the id and answers `Result<(), E>`; the `update` operation takes `task_id`, `key`, `action` (`accept`, `decline` or `cancel`) and `content: Option<String>`, the JSON text of the user's answer to the request under `key`, and answers `Result<(), E>`. A client that declared `io.modelcontextprotocol/tasks` in `_meta["io.modelcontextprotocol/clientCapabilities"].extensions` receives `resultType: "task"` with `status: "working"` and a `taskId` that carries the tool's name ahead of the application's id (`start-export:7f3a`), so `tasks/get`, which carries only the `taskId`, finds its operation with no state; any other client receives the handle as an ordinary success, with the application's own id and `status: "working"`, and `export-status`, `cancel-export` and the update operation are tools too (with no `outputSchema`). `tasks/get` answers the state with `resultType: "complete"`, the full `taskId` and, for `Completed`, a `result` that is the `CallToolResult` the starting tool would have answered; `tasks/cancel` and `tasks/update` answer an empty result, and `tasks/update` calls the operation once for each key of `inputResponses`, in key order. A declared error or a fault of the operation a task method calls is the `-32602` a prompt answers, and an id with no `:` or naming a tool that starts no task is `-32602` with the `unknown-operation` fault. `server/discover` adds `"io.modelcontextprotocol/tasks": {}` under `capabilities.extensions`; an endpoint that offers it nowhere answers the three methods as unknown (`404` over HTTP). `notifications/tasks`, whose params are the task's state under the full `taskId` (the `state` the change carries, as `McpTaskState` writes it, with a completed task's `result` as in `tasks/get`), rides on `subscriptions/listen` for each `McpChange::Task { task_id }` the application hands in whose id the client listed in `notifications.taskIds`, and the acknowledgment carries that list where the endpoint offers the extension; polling works without it. The three methods carry no `Mcp-Name` header, and `<Service>Schema::mcp_scopes("tasks/get", Some(name))` (also `tasks/cancel` and `tasks/update`) answers the `scopes` of the operation each names, `name` being the `taskId` or the tool's name alone. Every task message is held, in the tests, to the official schema of the extension, vendored as `tests/mcp_declaration_tests/mcp-tasks-schema-2026-07-28.json`. The generated clients declare no extension, so each method of the three operations reads the ordinary success into `McpTask` or `McpTaskState<T>`, which every language publishes once per bundle in its value-type accessor, `ts_mcp_content()`, `dart_mcp_content()`, `kotlin_mcp_content()` and `swift_mcp_content()`, beside the content, prompt and listing types, so a bundle names that one accessor and each type is declared once. The macro refuses a `task(...)` with no `get`, a key it does not take, a name that is no operation of the trait, an operation of the wrong shape, an `McpTask` or `McpTaskState` answered by an operation no task names, and a task start that is also a resource, a prompt, a listing, a completion, a skill or a content tool.

```rust,ignore
/// Reviews one document type against the naming guide.
#[service_schema_op(mcp(prompt))]
async fn review_document_type(&self, ctx: &Ctx, document_type: String, focus: Option<String>)
    -> Result<Vec<McpPromptMessage>, SetupError>;

/// Suggests the document types that start with what the user typed.
#[service_schema_op(mcp(completes(prompt = "review_document_type", parameter = "document_type")))]
async fn suggest_document_types(&self, ctx: &Ctx, typed: String, chosen: HashMap<String, String>)
    -> Result<Vec<String>, SetupError>;
```

**Calling tools.** `{service}_mcp_client!()` emits `Transport`, `McpMessage` and `{Service}Client<T>`, with one method per tool answering what the other Rust clients answer (`Result<Success, CallError<Error>>`, and `Result<(), ServiceFault>` for a one-way operation). `Transport::send` is handed a `McpMessage` whose `text()` is the JSON-RPC text and whose `protocol_version()`, `method()` and `name()` are the `MCP-Protocol-Version`, `Mcp-Method` and `Mcp-Name` an HTTP seam writes as headers; it answers the server's text, or `Err` for a call that did not travel. A message the validator refuses is refused before `send` is called. The client reads each answer as follows: a result with `isError: false` is the success (from `structuredContent`, or its `value` and the `header_out` properties beside it); `isError: true` with `"isServiceFault": true` is that fault; any other `isError: true` is the declared error; a JSON-RPC error that carries the fault envelope is that fault; any other JSON-RPC error and a failing `send` are a `transport-failure` fault; text that is no JSON, or a value its read refuses, is an `undeserializable-payload` fault.

```rust,ignore
let client = DocumentServiceClient::new(HttpMcpTransport::new(url, api_key));
let version = client.get_version(GetVersionRequest { document_id: "doc-1".into() }).await;
```

**TypeScript server and client.** A service that asks for `mcp` also publishes `<Service>Schema::ts_mcp_service()` and `ts_mcp_client()`, beside `ts_http_service()` and `ts_http_client()`. The server takes the `{Service}Impl<Ctx>` that `create{Service}Dispatcher` takes and calls that dispatcher, so the generated Zod reads judge every message. It answers every method the Rust server answers, with the bytes the Rust server answers: keys in order, and each tool kept as the JSON text Rust wrote, so a schema keeps its digits.

```ts
const route = createDocumentServiceMcpRoute(impl);
const workspace = documentServiceMcpEndpoint(documentServiceMcpPart, billingServiceMcpPart);
const host = {
  writer: async (message: string) => { process.stdout.write(`${message}\n`); },
  inFlight: new DocumentServiceMcpInFlight(),
};
const running: Array<Promise<void>> = [];
for await (const line of lines) {
  running.push(
    documentServiceMcpServe(line, "Workspace", undefined, workspace, (call) => ({ call }), host,
      async (ctx, part, method, params) => (part === 0 ? route(ctx, method, params) : undefined),
    ).then(async (reply) => { if (reply !== undefined) await host.writer(reply); }),
  );
}
await Promise.all(running);
```

A stdio host starts each line's answer and leaves it running, as the loop above does, because a host that waits for one answer before it reads the next line leaves a `notifications/cancelled` unread behind the handler it is meant to stop.

**Progress and cancellation on the TypeScript server.** `documentServiceMcpServe` and the dispatcher of one service take `ctx`, a function that builds the handler's `Ctx` from the `DocumentServiceMcpCall` of the request, and the writer (`(message: string) => Promise<void>`) and the `DocumentServiceMcpInFlight` table, which `serve` takes together as `{ writer, inFlight }`. A handler awaits `ctx.call.progress(1, 3, "exported 1 of 3 files")`, which writes `notifications/progress` through the writer where the request carries `_meta.progressToken` and does nothing otherwise; a write that rejects marks the call cancelled, and so does `notifications/cancelled` on stdio, which finds the request in the table by `requestId`. A handler reads `ctx.call.cancelled` and stops when it chooses, and the server drops the answer of a cancelled request. Over HTTP, `admit` receives the call, and a routed request with a `progressToken` answers an event stream written to the `stream` the host hands in, as it happens: `await stream.begin(200, headers)` once, before the first event, then `await stream.write(event)` for each notification between two steps of the handler and for the answer, where `event` is the framed text (`event: message\ndata: ...\n\n`) and the comment line `:\n\n` that keeps the stream open. A call that rejects marks the request cancelled, and a `begin` that rejects does so before the handler runs. The server writes the same messages in the same order as the Rust dispatcher, with a number written as JavaScript writes it (`1` where Rust writes `1.0`).

**Subscriptions on the TypeScript server.** `host.changes` of `serve`, the optional `changes` argument of the dispatcher of one service and the `changes` argument of the HTTP dispatcher are the application's source of changes, `() => Promise<DocumentServiceMcpChange | undefined>`: each call answers the next change, a `{ kind: "resourcesListChanged" }`, `{ kind: "resourceUpdated", uri }`, `{ kind: "task", taskId, state }` or `{ kind: "keepAlive" }`, and `undefined` ends the stream. Where it is handed in, `subscriptions/listen` writes the acknowledgment first, then only the changes the client asked for, and `server/discover` adds `"listChanged": true` and `"subscribe": true` to `resources`; a call that is cancelled, or whose write rejects, ends the stream with no answer, and `DocumentServiceMcpCall.stopped` resolves then. Over HTTP the acknowledgment, each change, the comment line `:` for each `keepAlive` and the end are written to the `stream` as they come. The server writes the same messages in the same order as the Rust dispatcher.

`createDocumentServiceMcpDispatcher(impl)` answers one line of stdio for the service alone, `(text, ctx, writer, inFlight) => Promise<string | undefined>`, and `createDocumentServiceMcpHttpDispatcher(impl, settings, admit)` applies the Streamable HTTP rules around it, in the order and with the answers the Rust dispatcher gives (`405` carries `Allow: POST`). It answers `(request, stream) => Promise<DocumentServiceMcpHttpResponse | undefined>`, where `stream` is a `DocumentServiceMcpHttpStream`, `{ begin(status, headers): Promise<void>, write(event): Promise<void> }`, and `undefined` means the answer went to the stream and the host ends its response. A Node host wires it to `res`:

```ts
const dispatch = createDocumentServiceMcpHttpDispatcher(impl, settings, admit);

http.createServer(async (req, res) => {
  const response = await dispatch(await mcpRequest(req), {
    async begin(status, headers) {
      res.writeHead(status, headers.map(([name, value]) => [name, value]).flat());
    },
    write: (event) =>
      new Promise((resolve, reject) => res.write(event, (error) => (error ? reject(error) : resolve()))),
  });
  if (response === undefined) res.end();
  else res.writeHead(response.status, response.headers.flat()).end(response.body);
});
```

**Authorization on the TypeScript server.** `settings` is `{ allowedOrigins, oauth? }`, where `oauth` is `{ authorizationServers, resource, resourceMetadataUrl, scopesSupported }` and, absent, publishes no metadata document and writes no challenge. `admit` is `(request, call) => Promise<{ ctx, serverName? } | { denied }>`; a `serverName` names the server for that request, so every answer of it carries the name in `serverInfo` and a `public` `cacheScope` becomes `private`, as in the Rust `McpAdmitted::named`. `denied` is `{ kind: "noValidCredentials", scope? }`, `{ kind: "insufficientScope", scope }` or `{ kind: "forbidden" }`, answered with the statuses, `WWW-Authenticate` challenges and metadata document of the Rust `McpDenied`. A `DocumentServiceMcpHttpRequest` carries `path` beside `method`, `headers` and `body`, and `documentServiceMcpHeader(request, name)`, `documentServiceMcpBearerToken(request)` and `documentServiceMcpName(request)` read it as the Rust request's helpers do. `documentServiceMcpScopes(method, name)` answers what `mcp_scopes(method, name)` answers, written at expansion from `mcp(scopes = [...])`. `call` is the `DocumentServiceMcpCall` of the request in flight. `documentServiceMcpServeHttp` takes the same `settings` and `admit` ahead of its `route`, which receives the `ctx` `admit` answered as its first argument. An endpoint of several services fails to type-check with `twoServicesShare` naming a tool, prompt or resource two parts share; a part whose names `tsc` cannot read as literals throws when the endpoint is built. The client, `createDocumentServiceMcpClient(transport)`, has one method per operation answering `{Service}{Operation}Result`; its `transport.send` is handed the JSON-RPC text and the values of the `MCP-Protocol-Version`, `Mcp-Method` and `Mcp-Name` headers, and answers the text of the reply.

##### The Dart server and client

`<Service>Schema::dart_mcp_service()` writes the Dart server, and `dart_mcp_client()` the Dart client; a service publishes both only where it asks for `mcp`. The server takes the `{Service}Handlers<Ctx>` that `dart_ws_client()` declares, reads each message with the read that attachment uses (which judges it), and answers the bytes the Rust server answers; a bundle names `dart_definition()` and `dart_ws_client()` ahead of them. An endpoint of several services is a `const`, so a tool, prompt or resource name two services share does not compile:

```dart
const workspace = DocumentServiceMcpEndpoint(
  {...documentServiceMcpNames, ...billingServiceMcpNames},
  [documentServiceMcpCatalog, billingServiceMcpCatalog],
);
final host = DocumentServiceMcpHost(
  writer: (message) async => stdout.writeln(message),
  inFlight: DocumentServiceMcpInFlight(),
);
final running = <Future<void>>[];
await for (final line in lines) {
  running.add(documentServiceMcpServe<RequestContext>(
    line, 'Workspace', null, workspace, (call) => RequestContext(call), host,
    (ctx, part, method, params) => switch (part) {
      0 => documents(ctx, method, params),
      1 => billing(ctx, method, params),
      _ => Future.value(null),
    },
  ).then((reply) async {
    if (reply != null) await host.writer(reply);
  }));
}
await Future.wait(running);
```

A stdio host leaves each line's future running, as the loop above does, because a host that waits for one answer before it reads the next line leaves a `notifications/cancelled` unread behind the handler it is meant to stop. A handler awaits `ctx.call.progress(1, total: 3, message: 'exported 1 of 3 files')`, which writes `notifications/progress` through the writer where the request carries `_meta.progressToken` and does nothing otherwise. A write that throws marks the call cancelled, and so does `notifications/cancelled`, which finds the request in the `DocumentServiceMcpInFlight` table by `requestId`; a handler reads `ctx.call.cancelled`, and the server drops the answer of a cancelled request. `DocumentServiceMcpWriter` is `Future<void> Function(String message)`. Over HTTP, `admit` receives the call, and a routed request with a `progressToken` answers an event stream written to the `DocumentServiceMcpHttpStream` the host hands in, as it happens: `begin(status, headers)` once, before the first event, then `write(event)` for each notification between two steps of the handler and for the answer, where `event` is the framed text (`event: message\ndata: ...\n\n`) and the comment line `:\n\n` that keeps the stream open. A call that throws marks the request cancelled, and a `begin` that throws does so before the handler runs.

**A one-way operation that panics.** The Rust dispatcher records the panic of a one-way handler through `tracing` and answers the empty success. The TypeScript, Dart and Kotlin servers hand the `handler-panic` fault to a callback you choose and answer the empty success: `createDocumentServiceMcpRoute(impl, onFault)`, `createDocumentServiceMcpDispatcher(impl, onFault)` and, after `changes`, `createDocumentServiceMcpHttpDispatcher(impl, settings, admit, changes, onFault)` in TypeScript; the optional last positional argument of the same functions in Dart, `void Function(DocumentServiceFaultFields)?`; and in Kotlin `createDocumentServiceMcpRoute(handlers, onFault)`, `createDocumentServiceMcpDispatcher(changes, onFault) { call -> handlers }` and `createDocumentServiceMcpHttpDispatcher(settings, changes, onFault, admit)`, where a cancelled handler is rethrown and reaches no callback. Without one the fault is written to `console.error` (TypeScript) or to standard error as JSON (Dart, Kotlin); a Dart file that holds a one-way operation imports `dart:io` for it. A replying operation's panic is still `-32603`.

A message that is a `Vec`, a `HashMap` or an `Option` of a model is read element by element with no judgement and validated element by element on every Rust transport, so an item that breaks a bound answers `failed-validation` naming the field the model's validator names, and an item that does not read answers `failed-validation` naming the path serde reached, with its index. The TypeScript server names the same field, without the index or the key it found the item at.

**Subscriptions on the Dart server.** `DocumentServiceMcpHost.changes`, the optional `changes` argument of the dispatcher of one service and of the HTTP dispatcher are the application's source of changes, `Future<DocumentServiceMcpChange?> Function()`: each call answers the next change, a `DocumentServiceMcpResourcesListChanged`, `DocumentServiceMcpResourceUpdated(uri)`, `DocumentServiceMcpTaskMoved(taskId, state)` or `DocumentServiceMcpKeepAlive`, and `null` ends the stream. The server names `Completer`, so the Dart file imports `dart:async`. Where it is handed in, `subscriptions/listen` writes the acknowledgment first, then only the changes the client asked for, and `server/discover` adds `"listChanged": true` and `"subscribe": true` to `resources`; a cancelled call, or a write that throws, ends the stream with no answer. Over HTTP the acknowledgment, each change, the comment line `:` for each keep-alive and the end are written to the stream as they come. The server writes the same messages in the same order as the Rust dispatcher.

`documentServiceMcpServe` answers one JSON-RPC text (or `null` for a notification), so any program that reads and writes lines hosts it over stdio; `documentServiceMcpServeHttp` adds the Streamable HTTP rules around it for a program that answers an HTTP `POST`. `createDocumentServiceMcpDispatcher(handlers)` serves the one service with `(text, ctx, writer, inFlight)`, and `createDocumentServiceMcpHttpDispatcher(handlers, settings, admit)` over HTTP, which answers `Future<DocumentServiceMcpHttpResponse?> Function(DocumentServiceMcpHttpRequest, DocumentServiceMcpHttpStream)`: `null` means the answer went to the stream and the host ends its response. A `dart:io` host wires it to the `HttpResponse`:

```dart
class Sse implements DocumentServiceMcpHttpStream {
  Sse(this.out);
  final HttpResponse out;

  @override
  Future<void> begin(int status, List<(String, String)> headers) async {
    out.statusCode = status;
    for (final (name, value) in headers) {
      out.headers.add(name, value);
    }
  }

  @override
  Future<void> write(String event) async {
    out.add(utf8.encode(event));
    await out.flush();
  }
}

final response = await dispatch(request, Sse(out));
if (response == null) {
  await out.close();
} else {
  out.statusCode = response.status;
  for (final (name, value) in response.headers) {
    out.headers.add(name, value);
  }
  out.add(response.body);
  await out.close();
}
```

**Authorization on the Dart server.** `settings` is `DocumentServiceMcpHttpSettings(allowedOrigins: [...], oauth: DocumentServiceMcpOAuth(authorizationServers:, resource:, resourceMetadataUrl:, scopesSupported:))`, and without `oauth` the server publishes no metadata document and writes no challenge. `admit` is a `DocumentServiceMcpAdmit<Ctx>`, `(request, call) async => (ctx: ..., denied: null, serverName: null)` or `(ctx: null, denied: DocumentServiceMcpNoValidCredentials(scope: ...), serverName: null)`; a `serverName` that is not `null` names the server for that request, so every answer of it carries the name in `serverInfo` and a `public` `cacheScope` becomes `private`, as in the Rust `McpAdmitted::named`. The other denials are `DocumentServiceMcpInsufficientScope(scope: ...)` and `DocumentServiceMcpForbidden()`; each answers the status, challenge and metadata document of the Rust `McpDenied`. `DocumentServiceMcpHttpRequest` carries `path`, and `request.header(name)`, `request.bearerToken` and `request.mcpName` read it as the Rust request's helpers do. `documentServiceMcpScopes(method, name)` answers what `mcp_scopes(method, name)` answers. `call` is the `DocumentServiceMcpCall` of the request in flight. A response now writes `Content-Type` and `Allow` in the case the Rust dispatcher writes them. Declared `final` in place of `const`, an endpoint drops a repeated name without a word, so declare it `const`.

The client is `DocumentServiceMcpClient(transport)`, with one method per operation answering the result type the other Dart clients answer. A consumer implements `DocumentServiceMcpTransport.send`, which is handed the JSON-RPC text and the values of the `MCP-Protocol-Version`, `Mcp-Method` and `Mcp-Name` headers; the URL and the credentials live in it. A one-way method throws `DocumentServiceMcpRefusal`.

##### The Kotlin server and the Kotlin and Swift clients

**On Kotlin and Swift.** On a service that asks for `mcp`, `<Service>Schema::kotlin_mcp_service()`, `kotlin_mcp_client()` and `swift_mcp_client()` publish the Kotlin server and the Kotlin and Swift clients, behind the `kotlin` and `swift` features. They read `mcp_tools()` and `wrapped_{operation}()` when the accessor runs in your program, so a tool list keeps the digits Rust wrote. A bundle names the accessor that holds the result types first: `kotlin_http_client()` and `swift_http_client()`, and for the server `kotlin_ws_client()`, which holds the `DocumentServiceHandlers` the server takes.

The Kotlin server answers the bytes the Rust dispatcher answers, with the keys of every object in order. It takes the `DocumentServiceHandlers` the WebSocket attachment takes, so a handler answers its declared error as `Declared`; a message is read with the read that attachment uses, which judges it. A message that does not read, and one the validator refuses, answer `failed-validation`, naming the field. A message that is not a JSON-RPC request answers `-32600` with no `id`, because the official schema allows an `id` that is a string or an integer and nothing else.

```kotlin
val workspace = DocumentServiceMcpEndpoint(documentServiceMcpCatalog, billingServiceMcpCatalog)
val host = DocumentServiceMcpHost({ message -> println(message) }, DocumentServiceMcpInFlight())
coroutineScope {
    for (line in lines) {
        launch {
            val reply = documentServiceMcpServe(line, "Workspace", null, workspace, host) { call, part, method, params ->
                when (part) { 0 -> createDocumentServiceMcpRoute(DocumentHandlers(call))(method, params); else -> null }
            }
            if (reply != null) host.writer(reply)
        }
    }
}
```

A stdio host `launch`es each line's answer, as the loop above does, because a host that waits for one answer before it reads the next line leaves a `notifications/cancelled` unread behind the handler it is meant to stop. `DocumentServiceHandlers` takes no `Ctx` that could hold the call, so `route` receives the `DocumentServiceMcpCall` of the request in flight and builds its handlers from it (`createDocumentServiceMcpRoute(DocumentHandlers(call))`). A handler calls `call.progress(1.0, 3.0, "exported 1 of 3 files")`, which writes `notifications/progress` through the writer (`suspend (message: String) -> Unit`) where the request carries `_meta.progressToken` and does nothing otherwise. A write that throws marks the call cancelled, and so does `notifications/cancelled`, which finds the request in the `DocumentServiceMcpInFlight` table by `requestId`; a handler reads `call.cancelled`, and the server drops the answer of a cancelled request. Over HTTP, `admit` receives the call, and a routed request with a `progressToken` answers an event stream written to the `DocumentServiceMcpHttpStream` the host hands in, as it happens: `begin(status, headers)` once, before the first event, then `write(event)` for each notification between two steps of the handler and for the answer, where `event` is the framed text (`event: message\ndata: ...\n\n`) and the comment line `:\n\n` that keeps the stream open. A call that throws marks the request cancelled, and a `begin` that throws does so before the handler runs.

**Subscriptions on the Kotlin server.** `createDocumentServiceMcpDispatcher(changes) { call -> handlers }`, `createDocumentServiceMcpHttpDispatcher(settings, changes) { request, call -> admitted }` and `DocumentServiceMcpHost(writer, inFlight, changes)` take the application's source of changes, `suspend () -> DocumentServiceMcpChange?`: each call answers the next change, a `DocumentServiceMcpChange.ResourcesListChanged`, `ResourceUpdated(uri)`, `Task(taskId, state)` or `KeepAlive`, and `null` ends the stream. The form without `changes` answers no `subscriptions/listen`. Where it is handed in, `subscriptions/listen` writes the acknowledgment first, then only the changes the client asked for, and `server/discover` adds `"listChanged": true` and `"subscribe": true` to `resources`; a cancelled call, or a write that throws, ends the stream with no answer (the wait for the next change is cancelled). Over HTTP the acknowledgment, each change, the comment line `:` for each keep-alive and the end are written to the stream as they come. The server writes the same messages in the same order as the Rust dispatcher.

`createDocumentServiceMcpDispatcher { call -> handlers }` serves one service from text, as `suspend (text, writer, inFlight) -> String?`, `createDocumentServiceMcpHttpDispatcher(settings, admit)` from `DocumentServiceMcpHttpRequest` and a `DocumentServiceMcpHttpStream` to `DocumentServiceMcpHttpResponse?` under the Streamable HTTP rules above (`null` means the answer went to the stream and the host ends its response), and `createDocumentServiceMcpRoute(handlers)` answers the requests of one service on an endpoint shared by several. Kotlin compares no list at compile time, because a `const val` holds only a primitive or a `String`: `DocumentServiceMcpEndpoint(a, b)` throws `IllegalStateException("two services share the MCP name tool:get-version")` when it is built. The answer a route gives is a `JsonObject` holding exactly one member, `result` or `error`, so a `route` over two services returns each service's answer as it is: `when (part) { 0 -> documents(method, params); 1 -> billing(method, params); else -> null }`. A route answer with neither member or with both answers `-32603` with a `handler-panic` fault naming the method.

**Authorization on the Kotlin server.** `DocumentServiceMcpHttpSettings(allowedOrigins, oauth = null)` holds the origins and, for OAuth, a `DocumentServiceMcpOAuth(authorizationServers, resource, resourceMetadataUrl, scopesSupported)`; without `oauth` the server publishes no metadata document and writes no challenge. `admit` is `suspend (DocumentServiceMcpHttpRequest, DocumentServiceMcpCall) -> DocumentServiceMcpAdmitted`: `DocumentServiceMcpAdmitted.Caller(handlers)` answers the handlers built for that caller, and `DocumentServiceMcpAdmitted.Caller(handlers, serverName = "Workspace-acme")` also names the server for that request: every answer of it carries the name in `serverInfo` and a `public` `cacheScope` becomes `private`, as in the Rust `McpAdmitted::named`; and `DocumentServiceMcpAdmitted.Denied(DocumentServiceMcpDenied.NoValidCredentials(scope))`, `.InsufficientScope(scope)` or `.Forbidden` refuse it, with the status, challenge and metadata document of the Rust `McpDenied`. `DocumentServiceMcpHttpRequest(method, path, headers, body)` reads `header(name)`, `bearerToken()` and `mcpName()` as the Rust request does, and `documentServiceMcpScopes(method, name)` answers what `mcp_scopes(method, name)` answers. `call` is the `DocumentServiceMcpCall` of the request in flight, which `admit` may build its handlers from. `documentServiceMcpServeHttp` takes `settings` and `admit` ahead of its `route`, whose first argument is the handlers `admit` built for the caller.

A Ktor host bridges the stream to the response with a channel:

```kotlin
post("/mcp") {
    val head = CompletableDeferred<Pair<Int, List<Pair<String, String>>>>()
    val events = Channel<String>(Channel.UNLIMITED)
    val stream = object : DocumentServiceMcpHttpStream {
        override suspend fun begin(status: Int, headers: List<Pair<String, String>>) {
            head.complete(status to headers)
        }

        override suspend fun write(event: String) = events.send(event)
    }
    val answering = async { dispatch(documentRequest(call), stream).also { events.close(); head.cancel() } }
    val began = runCatching { head.await() }.getOrNull()
    if (began == null) {
        respondWith(call, answering.await()!!)
    } else {
        began.second.forEach { (name, value) -> call.response.headers.append(name, value, safeOnly = false) }
        call.respondTextWriter(status = HttpStatusCode.fromValue(began.first)) {
            try {
                for (event in events) {
                    write(event)
                    flush()
                }
            } finally {
                events.cancel()
            }
        }
    }
}
```

The Kotlin and Swift clients write only `send`. `DocumentServiceMcpTransport.send` is handed a `DocumentServiceMcpMessage` holding the JSON-RPC text and the `MCP-Protocol-Version`, `Mcp-Method` and `Mcp-Name` values, and answers the server's text. Each method answers the sealed result `kotlin_http_client()` declares, or `Result<Success, {Service}{Operation}Failure>` in Swift: a result with `isError: false` is the success, the declared error is `Declared` (`.declared`), a fault in the answer, a text that is no JSON and a failing `send` are the `Fault` arm (`.fault`) with the `undeserializable-payload` or `transport-failure` kind, and a one-way method throws `DocumentServiceMcpRefusal` in Kotlin and `DocumentServiceRefusal` in Swift. The Kotlin client numbers its requests under `synchronized`, and the Swift client is an actor that numbers them in its own state. `_meta` names the client as `DocumentServiceMcpClient`, with the version of the crate that declares the service.

**Resources, templates and completion on the TypeScript, Dart and Kotlin servers.** Each server answers `resources/read`, `resources/list`, `resources/templates/list` and `completion/complete` with the bytes the Rust dispatcher answers, for a service that declares resources and for one that declares none (a routed request no service owns answers `-32602` with the `unknown-operation` fault, `resources/templates/list` answers an empty list, and `resources/list` answers the fixed resources of the endpoint and no cursor). A resource or a template is an operation of the service's own handlers: its message is built from the URI as in Rust, it is read and judged as a tool's arguments are, and a declared error answers `-32602` with the error in `data`. A `String` success answers `text` with the operation's `mime_type`, a `(Vec<u8>, String)` success answers `blob` in base64 with the content type the pair holds (`Uint8Array` in TypeScript, `Uint8List` in Dart, and in Kotlin the tuple class the clients declare, whose `slot0` holds the bytes), and a completion answers at most 100 values with `total` and `hasMore`. A cursor of `resources/list` is `"<service>:<cursor>"` on every server, `mcp_tools()`, `mcp_resources()` and `mcp_resource_templates()` write the members of each entry in key order whatever `serde_json` features the application builds with, and an endpoint refuses a resource or a template two services share (`resource:<uri>` and `resource template:<uri>` are the names it compares, so a tool of the same text does not collide).

`{Service}McpResource` and `{Service}McpResourcePage` (`LibraryServiceMcpResourcePage`) are declared by the value-type accessor of a service that declares a listing, `ts_mcp_content()` as TypeScript interfaces with their Zod schemas, `dart_mcp_content()` as Dart classes with `fromJson` and `toJson`, `kotlin_mcp_content()` as `@Serializable` data classes and `swift_mcp_content()` as `Codable` structs. Each carries its service's name, so a bundle of several services may list in every language; a handler's declared success stays `McpResourcePage` in the Rust service module, and names the prefixed type in the other three. In TypeScript, `{service}McpOperations` lists each operation by `kind` (`"tool"`, `"resource"`, `"listing"` or `"completion"`). A service that declares no tool gets no `tools/call` arm. In Kotlin the `(Vec<u8>, String)` wrapper is the `KotlinTupleOf{hash}` that `kotlin_http_client()` declares.

**Skills.** A server ships skills, sets of instructions a host loads into the model's context, in two ways, each answering `skills/list`, `skills/get` and `resources/read` of `skill://<name>/<path>` on the Rust, TypeScript, Dart and Kotlin servers, with the same bytes. `server/discover` then adds `"resources": {}` and `"io.modelcontextprotocol/skills": {}` under `capabilities` and `capabilities.extensions`.

A folder named on the service holds one directory per skill, each with a `SKILL.md` and its supporting files:

```rust,ignore
#[service_schema(transports = ["mcp"], mcp(skills = "skills"))]
pub trait DocumentService<Ctx> { /* ... */ }
// skills/code-review/SKILL.md
// skills/code-review/references/checklist.md
```

The path is relative to the crate's `Cargo.toml`. The macro reads the folder when it expands, with each `SKILL.md` frontmatter (`name` and `description`, plain, quoted or `|` and `>` block scalars; any other key must be a scalar too) and the SHA-256 digest and size of every file, and embeds the files with `include_bytes!`, so cargo rebuilds when one changes. `skills/list` answers `{ uri, frontmatter, resources: [{ uri, digest, size }] }` for each skill, in name order, and a file reads as `text` when it is UTF-8 and as a base64 `blob` otherwise. The directory name is the skill's name, must follow the Agent Skills rules, and must equal `name:` in the frontmatter; a `description` is required and holds at most 1024 characters. `sha2` is a dependency of the macro alone, used at compile time; nothing reaches a consumer at run time.

An operation writes a skill at run time, for content that varies by caller:

```rust,ignore
/// Writes the release notes of the caller's workspace: "what changed", from its latest changes.
#[service_schema_op(mcp(skill = "release-notes"))]
async fn release_notes_skill(&self, ctx: &Ctx, path: String) -> Result<String, DocumentError>;
```

It is listed with `"resources": "dynamic"`, its rustdoc as the description, `ttlMs: 0` and `cacheScope: "private"`, and `resources/read` of `skill://release-notes/<path>` calls the operation with `path` relative to the skill's root (`SKILL.md` for the instructions). A `String` success answers `text` as `text/markdown`; a `(Vec<u8>, String)` answers a `blob` with the content type of the pair. The transport writes the frontmatter of `SKILL.md` itself, ahead of the handler's text (`name`, and `description` as a JSON string, which YAML reads as the same text), so the file a host reads always equals the entry it compares it with. A declared error answers `-32602` with the error's JSON in `data`, and a panic answers `-32603`. `skills/get` takes the skill's `name` or its `uri` and answers `{ skill: <entry> }`; a skill nobody has answers `-32602` with the `unknown-operation` fault. A listing that holds a dynamic skill answers `ttlMs: 0` and `cacheScope: "private"`, and one of folder skills alone answers the five minutes and `public` of the other lists. An operation stays an ordinary operation on every other transport, and `MCP_NAMES` holds `("skill", name)` for both kinds, so two services of one endpoint cannot share a skill name.

The macro refuses, naming the operation: a skill name outside the Agent Skills rules (1 to 64 characters of `a-z`, `0-9` and `-`, with no `-` at either end and no two together), a rustdoc over 1024 characters, a name the folder also holds or another operation writes, arguments other than one `String`, a success other than `String` or `(Vec<u8>, String)`, a one-way operation, a header binding, and `scopes` (a skill is listed and read for every caller). `mcp(skills = "...")` on a service that does not ask for `mcp`, and a folder that cannot be read, holds no skill, holds a skill without a `SKILL.md` or whose frontmatter is not flat scalars, are refused as well. The TypeScript, Dart and Kotlin servers embed the entries and the files through their accessors, which read `mcp_skills()` and `mcp_skill_files()` when they run in your program.
