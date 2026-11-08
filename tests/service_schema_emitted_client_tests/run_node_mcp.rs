//! The emitted `mcp` server and client run by node: the requests of the design answered with the
//! bytes it records, every answer held to the official 2026-07-28 schema, two services behind one
//! endpoint, a schema's digits kept, and the client reading each outcome.

use super::mcp_auth_rows::{
    AUTHORIZATION_SERVER, AuthRow, METADATA_URL, ORIGIN, RESOURCE, SCOPE_QUERIES, SCOPES_SUPPORTED,
    auth_rows,
};
use super::mcp_node_document as document_mcp;
use super::runtime::{compiled_with_modules, node_modules, ran_with_modules, stand_down_modules};
use alloc::collections::BTreeMap;
use core::future::{Future, ready};
use core::pin::pin;
use core::task::{Context as PollContext, Poll, Waker};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tixschema::{model_schema, service_schema};

/// Names the runtime to run, for a machine that has one somewhere other than `PATH`.
const RUNTIME_VAR: &str = "TIXSCHEMA_NODE";

/// Names the compiler the emitted text is type-checked with.
const COMPILER_VAR: &str = "TIXSCHEMA_TSC";

/// The official 2026-07-28 schema of the Model Context Protocol.
const MCP_SCHEMA: &str = include_str!("../mcp_declaration_tests/mcp-schema-2026-07-28.json");

/// The head of every driver: the requests it sends, and the implementation it serves.
const REQUESTS: &str = r#"
const meta = {
  "io.modelcontextprotocol/protocolVersion": "2026-07-28",
  "io.modelcontextprotocol/clientCapabilities": {},
};
const request = (id, method, params) => JSON.stringify({ jsonrpc: "2.0", id, method, params: { ...params, _meta: meta } });
const call = (id, name, args) => request(id, "tools/call", { name, arguments: args });

const impl = {
  async getTagged(ctx, id, knownTag) {
    if (knownTag === "\"v7\"") return { ok: false, error: { errorCode: "not-found" } };
    return { ok: true, value: [{ version: 7 }, "\"v7\""] };
  },
  async getVersion(ctx, req) {
    if (req.document_id === "missing") return { ok: false, error: { errorCode: "not-found" } };
    if (req.document_id === "boom") throw new Error("the handler came apart");
    return { ok: true, value: { version: 7 } };
  },
  async lookup(ctx, id) {
    return { ok: true, value: { version: 7 } };
  },
  async renameDocument(ctx, req) {
    return { ok: true };
  },
  async touchDocument(ctx, req) {
    if (req.reader === "boom") throw new Error("the handler came apart");
  },
};
"#;

/// The requests of the design's wire section, and the refusals around them, in the order the tests
/// below read their answers.
const WIRE_DRIVER: &str = r#"
async function main() {
  const dispatch = createDocumentServiceMcpDispatcher(impl);
  const requests = [
    ["server/discover", JSON.stringify({ jsonrpc: "2.0", id: "discover-1", method: "server/discover", params: { _meta: meta } })],
    ["tools/list", request(1, "tools/list", {})],
    ["tools/call", call(2, "get-version", { document_id: "doc-1" })],
    ["tools/call", call(3, "rename-document", { documentId: "doc-1", title: "Plan" })],
    ["tools/call", call(4, "get-version", { document_id: "missing" })],
    ["tools/call", call(5, "get-version", { document_id: 5 })],
    ["tools/call", call(6, "get-version", { document_id: "" })],
    ["tools/call", call(7, "delete-everything", {})],
    ["tools/call", JSON.stringify({ jsonrpc: "2.0", id: 8, method: "tools/call", params: { name: "get-version", arguments: { document_id: "doc-1" }, _meta: { ...meta, "io.modelcontextprotocol/protocolVersion": "2025-11-25" } } })],
    ["tools/call", call(9, "get-version", { document_id: "boom" })],
    ["tools/call", call(10, "lookup-document", { value: "doc-1" })],
    ["tools/call", call(11, "touch-document", { documentId: "doc-1", reader: "reviewer-7" })],
    ["tools/call", call(12, "get-tagged", { value: "doc-1", "if-none-match": "\"v6\"" })],
    ["tools/call", call(13, "get-tagged", { value: "doc-1" })],
    ["tools/call", call(14, "get-tagged", { "if-none-match": "\"v7\"", value: "doc-1" })],
    ["tools/call", call(15, "touch-document", { documentId: "doc-1", reader: "boom" })],
    ["tools/call", call(16, "lookup-document", { value: 5 })],
    ["tools/call", call(17, "get-tagged", { value: "doc-1", "if-none-match": 5 })],
    ["tools/list", JSON.stringify({ jsonrpc: "2.0", method: "notifications/initialized" })],
    ["tools/list", "not json"],
    ["tools/list", request(18, "resources/list", {})],
    ["tools/list", JSON.stringify({ jsonrpc: "2.0", id: 19, method: "tools/list", params: {} })],
  ];
  const answers = [];
  for (const [method, text] of requests) {
    const reply = await dispatch({}, text);
    answers.push({ method, reply: reply === undefined ? null : reply });
  }
  console.log(JSON.stringify(answers));
  process.exit(0);
}
main().catch((error) => { console.error(error); process.exit(1); });
"#;

/// The HTTP rules in the order the design lists them, one request for each.
const HTTP_DRIVER: &str = r#"
async function main() {
  const dispatch = createDocumentServiceMcpHttpDispatcher(impl, { allowedOrigins: ["https://app.example.com"] }, async () => ({ ctx: {} }));
  const encode = (text) => new TextEncoder().encode(text);
  const post = (text, headers = [], method = "POST") => ({ method, path: "/mcp", headers, body: encode(text) });
  const agreeing = (text, method, name) => {
    const headers = [["MCP-Protocol-Version", "2026-07-28"], ["Mcp-Method", method]];
    if (name !== undefined) headers.push(["mcp-name", name]);
    return post(text, headers);
  };
  const toolCall = call(1, "get-version", { document_id: "doc-1" });
  const requests = [
    ["tools/call", post(toolCall, [], "GET")],
    ["tools/call", post(toolCall, [["Origin", "https://elsewhere.example.com"]])],
    ["tools/call", post("not json")],
    ["tools/call", { method: "POST", path: "/mcp", headers: [], body: new Uint8Array([0xff, 0xfe]) }],
    ["tools/call", post(JSON.stringify({ jsonrpc: "2.0", method: "notifications/initialized" }))],
    ["tools/call", post(toolCall, [["Mcp-Method", "tools/call"], ["Mcp-Name", "get-version"]])],
    ["tools/call", agreeing(toolCall, "tools/list", "get-version")],
    ["tools/call", agreeing(toolCall, "tools/call", "lookup-document")],
    ["tools/call", agreeing(toolCall, "tools/call", undefined)],
    ["tools/call", agreeing(toolCall, "tools/call", "get-version")],
    ["tools/call", { ...agreeing(toolCall, "tools/call", "get-version"), headers: [["mcp-protocol-version", "2026-07-28"], ["MCP-METHOD", "tools/call"], ["MCP-NAME", "=?base64?Z2V0LXZlcnNpb24=?="], ["Origin", "https://app.example.com"]] }],
    ["tools/call", agreeing(call(2, "delete-everything", {}), "tools/call", "delete-everything")],
    ["tools/list", agreeing(request(3, "tools/list", {}), "tools/list")],
    ["tools/list", agreeing(JSON.stringify({ jsonrpc: "2.0", id: 4, method: "tools/list", params: { _meta: { "io.modelcontextprotocol/protocolVersion": "2026-07-28" } } }), "tools/list")],
    ["tools/list", post(JSON.stringify({ jsonrpc: "2.0", id: 5, method: "tools/list", params: { _meta: { ...meta, "io.modelcontextprotocol/protocolVersion": "2025-11-25" } } }), [["MCP-Protocol-Version", "2025-11-25"], ["Mcp-Method", "tools/list"]])],
    ["tools/list", agreeing(request(6, "prompts/list", {}), "prompts/list")],
  ];
  const answers = [];
  for (const [method, input] of requests) {
    const response = await dispatch(input);
    answers.push({ method, status: response.status, headers: response.headers, body: new TextDecoder().decode(response.body) });
  }
  console.log(JSON.stringify(answers));
  process.exit(0);
}
main().catch((error) => { console.error(error); process.exit(1); });
"#;

/// Authorization: every request of `auth_rows()` through the server with the `admit` of its scheme,
/// and every scope query through `documentServiceMcpScopes`.
const AUTH_DRIVER: &str = r#"
async function main() {
  const input = JSON.parse(INPUT);
  let reached = 0;
  const answering = { ...impl, async renameDocument() { return { ok: false, error: { errorCode: "not-found" } }; } };
  const counted = Object.fromEntries(Object.entries(answering).map(([name, handler]) => [name, (...given) => { reached += 1; return handler(...given); }]));
  const tokens = new Map([["read-token", "documents:read"], ["write-token", "documents:read documents:write"]]);
  const admits = {
    oauth: async (request) => {
      const needed = documentServiceMcpScopes(documentServiceMcpHeader(request, "mcp-method") ?? "", documentServiceMcpName(request));
      const token = documentServiceMcpBearerToken(request);
      if (token === undefined) return { denied: needed.length === 0 ? { kind: "noValidCredentials" } : { kind: "noValidCredentials", scope: needed.join(" ") } };
      if (token === "forbidden-token") return { denied: { kind: "forbidden" } };
      const held = tokens.get(token);
      if (held === undefined) return { denied: { kind: "noValidCredentials" } };
      if (needed.some((scope) => !held.split(" ").includes(scope))) return { denied: { kind: "insufficientScope", scope: needed.join(" ") } };
      return { ctx: {} };
    },
    keys: async (request) => {
      const key = documentServiceMcpHeader(request, "x-api-key");
      const secret = documentServiceMcpHeader(request, "x-api-secret");
      if (key === undefined || secret === undefined) return { denied: { kind: "noValidCredentials" } };
      return key === "admin-key" && secret === "admin-secret" ? { ctx: {} } : { denied: { kind: "forbidden" } };
    },
  };
  const settings = {
    oauth: { allowedOrigins: [input.origin], oauth: input.oauth },
    keys: { allowedOrigins: [input.origin] },
  };
  const answers = [];
  for (const row of input.rows) {
    const dispatch = createDocumentServiceMcpHttpDispatcher(counted, settings[row.scheme], admits[row.scheme]);
    const before = reached;
    const response = await dispatch({ method: row.method, path: row.path, headers: row.headers, body: new TextEncoder().encode(row.body) });
    answers.push({ status: response.status, headers: response.headers, body: new TextDecoder().decode(response.body), reached: reached - before });
  }
  const scopes = input.queries.map(([method, name]) => documentServiceMcpScopes(method, name ?? undefined));
  console.log(JSON.stringify({ answers, scopes }));
  process.exit(0);
}
main().catch((error) => { console.error(error); process.exit(1); });
"#;

/// Two services behind one endpoint, `route` calling each by its place.
const ENDPOINT_DRIVER: &str = r#"
const billingImpl = {
  async listInvoices(ctx, req) {
    return { ok: true, value: { invoices: ["inv-1", `limit=${req.limit}`] } };
  },
};

async function main() {
  const documents = createDocumentServiceMcpRoute(impl);
  const billing = createBillingServiceMcpRoute(billingImpl);
  const workspace = documentServiceMcpEndpoint(documentServiceMcpPart, billingServiceMcpPart);
  const serve = (text) => documentServiceMcpServe(text, "Workspace", undefined, workspace,
    async (part, method, params) => (part === 0 ? documents({}, method, params) : part === 1 ? billing({}, method, params) : undefined));
  const replies = [
    await serve(JSON.stringify({ jsonrpc: "2.0", id: "d", method: "server/discover", params: { _meta: meta } })),
    await serve(request(1, "tools/list", {})),
    await serve(call(2, "get-version", { document_id: "doc-1" })),
    await serve(`{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"list-invoices","arguments":{"limit":9007199254740993,"load":1.0},"_meta":${JSON.stringify(meta)}}}`),
    await serve(call(4, "delete-everything", {})),
  ];
  let sharedMessage = null;
  try {
    documentServiceMcpEndpoint(...[documentServiceMcpPart, archiveServiceMcpPart]);
  } catch (thrown) {
    sharedMessage = String(thrown.message);
  }
  console.log(JSON.stringify({ replies, sharedMessage }));
  process.exit(0);
}
main().catch((error) => { console.error(error); process.exit(1); });
"#;

/// An endpoint of two parts with distinct names, and one built from parts whose names `tsc` cannot
/// see.
const ENDPOINT_CALLERS: &str = r#"import {
  archiveServiceMcpPart,
  billingServiceMcpPart,
  documentServiceMcpEndpoint,
  documentServiceMcpPart,
  type DocumentServiceMcpPart,
} from "./bundle.js";

export const apart = documentServiceMcpEndpoint(documentServiceMcpPart, billingServiceMcpPart);
export const alone = documentServiceMcpEndpoint(archiveServiceMcpPart);

declare const unseen: DocumentServiceMcpPart;
export const widened = documentServiceMcpEndpoint(unseen, unseen);
"#;

/// An endpoint whose two parts both name `tool:get-version`.
const SHARED_NAME_CALLER: &str = r#"import {
  archiveServiceMcpPart,
  documentServiceMcpEndpoint,
  documentServiceMcpPart,
} from "./bundle.js";

export const shared = documentServiceMcpEndpoint(documentServiceMcpPart, archiveServiceMcpPart);
"#;

/// The client and the server written against what they publish.
const USAGE_CALLER: &str = r#"import {
  createDocumentServiceMcpClient,
  createDocumentServiceMcpDispatcher,
  createDocumentServiceMcpHttpDispatcher,
  createDocumentServiceMcpRoute,
  documentServiceMcpOperations,
  documentServiceMcpPart,
  documentServiceMcpServe,
  documentServiceMcpServeHttp,
  documentServiceMcpName,
  documentServiceMcpScopes,
  DocumentServiceMcpCall,
  type DocumentServiceImpl,
  type DocumentServiceMcpAdmitted,
  type DocumentServiceMcpDenied,
  type DocumentServiceMcpHttpRequest,
  type DocumentServiceMcpHttpSettings,
  type DocumentServiceMcpTransport,
} from "./bundle.js";

declare const impl: DocumentServiceImpl<number>;
declare const transport: DocumentServiceMcpTransport;

export const dispatcher: (ctx: number, text: string) => Promise<string | undefined> = createDocumentServiceMcpDispatcher(impl);
export const http = createDocumentServiceMcpHttpDispatcher(
  impl,
  { allowedOrigins: [] },
  async (request: DocumentServiceMcpHttpRequest, call: DocumentServiceMcpCall) =>
    call instanceof DocumentServiceMcpCall && request.path === "/mcp"
      ? { ctx: 1 }
      : ({ denied: { kind: "forbidden" } } satisfies DocumentServiceMcpAdmitted<number>),
);
export const settings: DocumentServiceMcpHttpSettings = {
  allowedOrigins: [],
  oauth: {
    authorizationServers: [],
    resource: "",
    resourceMetadataUrl: "",
    scopesSupported: [],
  },
};
export const denied: ReadonlyArray<DocumentServiceMcpDenied> = [
  { kind: "noValidCredentials" },
  { kind: "noValidCredentials", scope: "documents:read" },
  { kind: "insufficientScope", scope: "documents:write" },
  { kind: "forbidden" },
];
export const needed: ReadonlyArray<string> = documentServiceMcpScopes("tools/call", documentServiceMcpName({ method: "POST", path: "/", headers: [], body: new Uint8Array() }));
export const route = createDocumentServiceMcpRoute(impl);
export const serve = documentServiceMcpServe;
export const serveHttp = documentServiceMcpServeHttp;
export const operations = documentServiceMcpOperations.map((operation) => operation.tool);
export const names: readonly ["tool:get-tagged", "tool:get-version", "tool:lookup-document", "tool:rename-document", "tool:touch-document"] = documentServiceMcpPart.names;

export async function usage() {
  const client = createDocumentServiceMcpClient(transport);
  const version = await client.getVersion({ document_id: "doc-1" });
  if (version.ok) {
    const found: number = version.value.version;
    return found;
  }
  if ("isServiceFault" in version.error) return version.error.fault.kind;
  return version.error.errorCode;
}
"#;

/// One canned answer for each outcome the client reads, then the client against the emitted server.
const CLIENT_DRIVER: &str = r#"
const answerOf = (result) => JSON.stringify({ jsonrpc: "2.0", id: 1, result: { resultType: "complete", ...result } });
const text = (value) => ({ type: "text", text: JSON.stringify(value) });
const fault = { detail: "the handler came apart", kind: "handler-panic", operation: "get-version" };

async function main() {
  const seen = [];
  let canned = () => "";
  const client = createDocumentServiceMcpClient({
    async send(message) {
      seen.push({ protocolVersion: message.protocolVersion, method: message.method, name: message.name, request: JSON.parse(message.text) });
      return canned(message);
    },
  });
  const outcomes = {};
  const ask = async (label, answer, call) => {
    canned = answer;
    outcomes[label] = await call();
  };
  const version = () => client.getVersion({ document_id: "doc-1" });
  await ask("success", () => answerOf({ content: [text({ version: 7 })], isError: false, structuredContent: { version: 7 } }), version);
  await ask("faultInText", () => answerOf({ content: [text({ fault, isServiceFault: true })], isError: true }), version);
  await ask("declared", () => answerOf({ content: [text({ errorCode: "not-found" })], isError: true }), version);
  await ask("faultInError", () => JSON.stringify({ jsonrpc: "2.0", id: 1, error: { code: -32603, message: "get-version: the handler came apart", data: { fault, isServiceFault: true } } }), version);
  await ask("otherError", () => JSON.stringify({ jsonrpc: "2.0", id: 1, error: { code: -32601, message: "Method not found: tools/call" } }), version);
  await ask("seamFails", () => { throw new Error("the socket closed"); }, version);
  await ask("notJson", () => "<html>", version);
  await ask("refusedValue", () => answerOf({ content: [], isError: false, structuredContent: { version: "seven" } }), version);
  await ask("unit", () => answerOf({ content: [], isError: false }), () => client.renameDocument({ documentId: "doc-1", title: "Plan" }));
  await ask("header", () => answerOf({ content: [], isError: false, structuredContent: { etag: "\"v7\"", value: { version: 7 } } }), () => client.getTagged("doc-1", "\"v6\""));
  await ask("absentHeader", () => answerOf({ content: [], isError: false, structuredContent: { value: { version: 7 } } }), () => client.getTagged("doc-1", undefined));
  await ask("wrapped", () => answerOf({ content: [], isError: false, structuredContent: { version: 7 } }), () => client.lookup("doc-1"));
  await ask("refusedBeforeSending", () => { throw new Error("must not be reached"); }, () => client.getVersion({ document_id: "" }));
  await ask("oneWay", () => answerOf({ content: [], isError: false }), async () => (await client.touchDocument({ documentId: "doc-1", reader: "reviewer-7" })) ?? "void");
  let thrown = null;
  canned = () => answerOf({ content: [text({ fault, isServiceFault: true })], isError: true });
  try { await client.touchDocument({ documentId: "doc-1", reader: "reviewer-7" }); } catch (caught) { thrown = { fault: caught.fault }; }
  outcomes.oneWayThrows = thrown;

  const served = createDocumentServiceMcpDispatcher(impl);
  const round = createDocumentServiceMcpClient({
    send: async (message) => (await served({}, message.text)) ?? "",
  });
  outcomes.roundTrip = {
    success: await round.getVersion({ document_id: "doc-1" }),
    declared: await round.getVersion({ document_id: "missing" }),
    panic: await round.getVersion({ document_id: "boom" }),
    header: await round.getTagged("doc-1", "\"v6\""),
    declaredByHeader: await round.getTagged("doc-1", "\"v7\""),
    wrapped: await round.lookup("doc-1"),
    unit: await round.renameDocument({ documentId: "doc-1", title: "Plan" }),
    oneWay: (await round.touchDocument({ documentId: "doc-1", reader: "reviewer-7" })) ?? "void",
  };
  console.log(JSON.stringify({ outcomes, seen }));
  process.exit(0);
}
main().catch((error) => { console.error(error); process.exit(1); });
"#;

/// The one package the drivers import at runtime: `zod`, for the schemas the dispatcher judges by.
const REQUIRED_PACKAGES: &[&str] = &["zod"];

#[model_schema(enable = ["jsonschema"])]
#[derive(Debug, Deserialize, Serialize)]
pub struct GetVersionRequest {
    #[model_schema_prop(minLength = 1)]
    pub document_id: String,
}

#[model_schema(enable = ["jsonschema"])]
#[derive(Debug, Deserialize, Serialize)]
pub struct VersionResponse {
    pub version: u32,
}

#[model_schema(enable = ["jsonschema"])]
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case", tag = "errorCode")]
pub enum VersionError {
    NotFound,
}

#[model_schema(enable = ["jsonschema"])]
#[derive(Debug, Deserialize, Serialize)]
#[serde(transparent)]
pub struct DocumentId(String);

#[model_schema(enable = ["jsonschema"])]
#[derive(Debug, Deserialize, Serialize)]
pub struct InvoiceRequest {
    #[model_schema_prop(maximum = 9007199254740993)]
    pub limit: i64,
    #[model_schema_prop(maximum = 120)]
    pub load: f64,
}

#[model_schema(enable = ["jsonschema"])]
#[derive(Debug, Deserialize, Serialize)]
pub struct InvoiceList {
    pub invoices: Vec<String>,
}

#[model_schema(enable = ["jsonschema"])]
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case", tag = "errorCode")]
pub enum BillingError {
    Closed,
}

/// Reads and renames documents in the caller's workspace.
#[service_schema(transports = ["mcp"])]
pub trait DocumentService<Ctx> {
    /// Answers the current version of one document, unless it still carries the tag the caller
    /// knows.
    #[service_schema_op(http(
        method = "POST",
        path = "/get-tagged",
        header_in("if-none-match" = known_tag),
        header_out("etag")
    ))]
    async fn get_tagged(
        &self,
        ctx: &Ctx,
        id: DocumentId,
        known_tag: Option<String>,
    ) -> Result<(VersionResponse, String), VersionError>;

    /// Answers the current version of one document.
    #[service_schema_op(mcp(title = "Current Document Version", scopes = ["documents:read"]))]
    async fn get_version(
        &self,
        ctx: &Ctx,
        req: GetVersionRequest,
    ) -> Result<VersionResponse, VersionError>;

    /// Answers the current version of the document the identifier names.
    #[service_schema_op(mcp(name = "lookup-document"))]
    async fn lookup(&self, ctx: &Ctx, id: DocumentId) -> Result<VersionResponse, VersionError>;

    /// Renames one document.
    #[service_schema_op(mcp(scopes = ["documents:write"]))]
    async fn rename_document(
        &self,
        ctx: &Ctx,
        document_id: String,
        title: String,
    ) -> Result<(), VersionError>;

    /// Marks one document as read.
    #[service_schema_op(one_way, mcp(scopes = ["documents:read", "documents:touch"]))]
    async fn touch_document(&self, ctx: &Ctx, document_id: String, reader: String);
}

/// Bills the caller's organization.
#[service_schema(transports = ["mcp"])]
pub trait BillingService<Ctx> {
    /// Lists the invoices of the caller's organization.
    async fn list_invoices(
        &self,
        ctx: &Ctx,
        req: InvoiceRequest,
    ) -> Result<InvoiceList, BillingError>;
}

/// A second service that lists a tool `DocumentService` lists.
#[service_schema(transports = ["mcp"])]
pub trait ArchiveService<Ctx> {
    /// Answers the version of one archived document.
    async fn get_version(
        &self,
        ctx: &Ctx,
        req: GetVersionRequest,
    ) -> Result<VersionResponse, VersionError>;
}

/// Answers every operation of every service with the version `7`, or the error it declares.
struct Stand;

impl DocumentService<()> for Stand {
    async fn get_tagged(
        &self,
        _ctx: &(),
        _id: DocumentId,
        _known_tag: Option<String>,
    ) -> Result<(VersionResponse, String), VersionError> {
        ready(()).await;
        Ok((VersionResponse { version: 7 }, "\"v7\"".to_owned()))
    }

    async fn get_version(
        &self,
        _ctx: &(),
        _req: GetVersionRequest,
    ) -> Result<VersionResponse, VersionError> {
        ready(()).await;
        Ok(VersionResponse { version: 7 })
    }

    async fn lookup(&self, _ctx: &(), _id: DocumentId) -> Result<VersionResponse, VersionError> {
        ready(()).await;
        Ok(VersionResponse { version: 7 })
    }

    async fn rename_document(
        &self,
        _ctx: &(),
        _document_id: String,
        _title: String,
    ) -> Result<(), VersionError> {
        ready(()).await;
        Err(VersionError::NotFound)
    }

    async fn touch_document(&self, _ctx: &(), _document_id: String, _reader: String) {
        ready(()).await;
    }
}

impl BillingService<()> for Stand {
    async fn list_invoices(
        &self,
        _ctx: &(),
        _req: InvoiceRequest,
    ) -> Result<InvoiceList, BillingError> {
        ready(()).await;
        Err(BillingError::Closed)
    }
}

impl ArchiveService<()> for Stand {
    async fn get_version(
        &self,
        _ctx: &(),
        _req: GetVersionRequest,
    ) -> Result<VersionResponse, VersionError> {
        ready(()).await;
        Ok(VersionResponse { version: 7 })
    }
}

/// What `answering` answers on its first poll, which is every answer `Stand` gives.
fn answered<Answering>(answering: Answering) -> Answering::Output
where
    Answering: Future,
{
    let mut pinned = pin!(answering);
    let reply = match pinned
        .as_mut()
        .poll(&mut PollContext::from_waker(Waker::noop()))
    {
        Poll::Ready(output) => Some(output),
        Poll::Pending => None,
    };
    reply.unwrap()
}

/// Every type and service of this group as the one module the drivers below run.
fn bundle() -> String {
    [
        "import { z, type ZodType, type $brand } from \"zod\";".to_owned(),
        GetVersionRequest::ts_definition(),
        GetVersionRequest::zod_schema(),
        VersionResponse::ts_definition(),
        VersionResponse::zod_schema(),
        VersionError::ts_definition(),
        VersionError::zod_schema(),
        DocumentId::ts_definition(),
        DocumentId::zod_schema(),
        InvoiceRequest::ts_definition(),
        InvoiceRequest::zod_schema(),
        InvoiceList::ts_definition(),
        InvoiceList::zod_schema(),
        BillingError::ts_definition(),
        BillingError::zod_schema(),
        DocumentServiceSchema::ts_definition(),
        DocumentServiceSchema::ts_service(),
        DocumentServiceSchema::ts_mcp_service(),
        DocumentServiceSchema::ts_mcp_client(),
        BillingServiceSchema::ts_definition(),
        BillingServiceSchema::ts_service(),
        BillingServiceSchema::ts_mcp_service(),
        BillingServiceSchema::ts_mcp_client(),
        ArchiveServiceSchema::ts_definition(),
        ArchiveServiceSchema::ts_service(),
        ArchiveServiceSchema::ts_mcp_service(),
        ArchiveServiceSchema::ts_mcp_client(),
    ]
    .join("\n\n")
}

/// Runs `driver` after the bundle under node, or stands down naming `TIXSCHEMA_NODE_MODULES`.
fn run(named: &str, driver: &str) -> Option<Value> {
    let Some(modules) = node_modules(REQUIRED_PACKAGES) else {
        stand_down_modules(
            REQUIRED_PACKAGES,
            "the emitted TypeScript MCP server and client",
        );
        return None;
    };
    let source = format!("{}\n\n{driver}", bundle());
    let wrote = ran_with_modules(
        named,
        RUNTIME_VAR,
        "node",
        &format!("{named}.mts"),
        &source,
        &modules,
    )?;
    Some(serde_json::from_str(wrote.trim()).unwrap())
}

/// `serverInfo` as every answer of the service named `named` carries it.
fn info(named: &str) -> Value {
    json!({ "io.modelcontextprotocol/serverInfo": {
        "name": named,
        "version": env!("CARGO_PKG_VERSION"),
    } })
}

/// `value` as compact JSON with the keys of every object in order, as the design records its
/// bytes whatever order the build of `serde_json` keeps.
fn sorted_text(value: &Value) -> String {
    match value {
        Value::Object(members) => {
            let ordered: BTreeMap<&String, &Value> = members.iter().collect();
            let written: Vec<String> = ordered
                .into_iter()
                .map(|(key, held)| format!("{}:{}", Value::String(key.clone()), sorted_text(held)))
                .collect();
            format!("{{{}}}", written.join(","))
        }
        Value::Array(items) => {
            let written: Vec<String> = items.iter().map(sorted_text).collect();
            format!("[{}]", written.join(","))
        }
        Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_) => value.to_string(),
    }
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

/// Holds an answer to the definition its request names: the result definition of that request, or
/// the error definition of the code. The schema's request id is a string or an integer, so the
/// `null` an unreadable request is answered under is held without one.
fn held_as_answer(request_method: &str, answer: &Value) {
    let mut held = answer.clone();
    if held["id"].is_null()
        && let Some(members) = held.as_object_mut()
    {
        members.remove("id");
    }
    if let Some(code) = held["error"]["code"].as_i64() {
        held_to("JSONRPCErrorResponse", &held);
        let error = held["error"].clone();
        let specific = match code {
            -32600 => "InvalidRequestError",
            -32601 => "MethodNotFoundError",
            -32602 => "InvalidParamsError",
            -32603 => "InternalError",
            -32020 => "HeaderMismatchError",
            -32022 => "UnsupportedProtocolVersionError",
            _ => "",
        };
        assert!(
            !specific.is_empty(),
            "an error code the schema does not record: {code}"
        );
        held_to(
            specific,
            if matches!(code, -32020 | -32022) {
                &held
            } else {
                &error
            },
        );
        return;
    }
    let definition = match request_method {
        "server/discover" => "DiscoverResultResponse",
        "tools/list" => "ListToolsResultResponse",
        "tools/call" => "CallToolResultResponse",
        _ => "",
    };
    assert!(
        !definition.is_empty(),
        "a method this group does not answer: {request_method}"
    );
    held_to(definition, &held);
}

/// Holds every answer of the wire driver to the official schema.
fn held_every(answers: &Value) {
    for held in answers.as_array().unwrap() {
        if let Some(text) = held["reply"].as_str() {
            held_as_answer(
                held["method"].as_str().unwrap(),
                &serde_json::from_str(text).unwrap(),
            );
        }
    }
}

/// Every answer the wire driver collects, each held to the official schema.
fn wire_answers() -> Option<Value> {
    let driver = format!("{REQUESTS}\n{WIRE_DRIVER}");
    run("mcp-wire", &driver).inspect(held_every)
}

/// The text and the reading of the answer to the request numbered `id`.
fn answer_to(answers: &Value, id: u64) -> (String, Value) {
    answers
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|held| held["reply"].as_str())
        .filter_map(|text| {
            serde_json::from_str::<Value>(text)
                .ok()
                .map(|read| (text.to_owned(), read))
        })
        .find(|(_, read)| read["id"] == id)
        .unwrap()
}

#[test]
fn the_server_answers_the_bytes_the_design_records() {
    let Some(answers) = wire_answers() else {
        return;
    };
    let server = info("DocumentService");
    let tools: Vec<String> = DocumentServiceSchema::mcp_tools()
        .iter()
        .map(Value::to_string)
        .collect();
    let discover = sorted_text(&json!({
        "id": "discover-1",
        "jsonrpc": "2.0",
        "result": {
            "_meta": server,
            "cacheScope": "public",
            "capabilities": { "tools": {} },
            "instructions": "Reads and renames documents in the caller's workspace.",
            "resultType": "complete",
            "supportedVersions": ["2026-07-28"],
            "ttlMs": 300_000_i32,
        },
    }));
    assert_eq!(answers[0]["reply"], discover);
    let listed = format!(
        "{{\"id\":1,\"jsonrpc\":\"2.0\",\"result\":{{\"_meta\":{},\"cacheScope\":\"public\",\"resultType\":\"complete\",\"tools\":[{}],\"ttlMs\":300000}}}}",
        sorted_text(&server),
        tools.join(",")
    );
    assert_eq!(answers[1]["reply"], listed);
    let success = json!({
        "_meta": server,
        "content": [{ "text": "{\"version\":7}", "type": "text" }],
        "isError": false,
        "resultType": "complete",
        "structuredContent": { "version": 7_i32 },
    });
    let empty = json!({
        "_meta": server,
        "content": [],
        "isError": false,
        "resultType": "complete",
    });
    let declared = json!({
        "_meta": server,
        "content": [{ "text": "{\"errorCode\":\"not-found\"}", "type": "text" }],
        "isError": true,
        "resultType": "complete",
    });
    let etagged = json!({
        "_meta": server,
        "content": [{ "text": "{\"etag\":\"\\\"v7\\\"\",\"value\":{\"version\":7}}", "type": "text" }],
        "isError": false,
        "resultType": "complete",
        "structuredContent": { "etag": "\"v7\"", "value": { "version": 7_i32 } },
    });
    let unknown_tool = json!({
        "code": -32_602_i32,
        "data": { "fault": {
            "detail": "the service answers to no operation by that name",
            "kind": "unknown-operation",
            "operation": "delete-everything",
        }, "isServiceFault": true },
        "message": "Unknown tool: delete-everything",
    });
    let unsupported = json!({
        "code": -32_022_i32,
        "data": { "requested": "2025-11-25", "supported": ["2026-07-28"] },
        "message": "Unsupported protocol version",
    });
    let panicked = json!({
        "code": -32_603_i32,
        "data": { "fault": {
            "detail": "the handler came apart",
            "kind": "handler-panic",
            "operation": "get-version",
        }, "isServiceFault": true },
        "message": "get-version: the handler came apart",
    });
    let expected = [
        (2, json!({ "result": success })),
        (3, json!({ "result": empty })),
        (4, json!({ "result": declared })),
        (7, json!({ "error": unknown_tool })),
        (8, json!({ "error": unsupported })),
        (9, json!({ "error": panicked })),
        (10, json!({ "result": success })),
        (11, json!({ "result": empty })),
        (12, json!({ "result": etagged })),
        (14, json!({ "result": declared })),
        (15, json!({ "result": empty })),
    ];
    for (id, mut message) in expected {
        message["id"] = json!(id);
        message["jsonrpc"] = json!("2.0");
        assert_eq!(
            answer_to(&answers, id).0,
            sorted_text(&message),
            "the answer to request {id}"
        );
    }
}

#[test]
fn a_message_the_zod_reads_refuse_answers_the_failed_validation_fault() {
    let Some(answers) = wire_answers() else {
        return;
    };
    for (id, field, operation) in [
        (5, Some("document_id"), "get-version"),
        (6, Some("document_id"), "get-version"),
        (16, None, "lookup"),
        (17, Some("if-none-match"), "get-tagged"),
    ] {
        let (_, reply) = answer_to(&answers, id);
        let result = &reply["result"];
        assert_eq!(result["isError"], true, "request {id}: {reply:#}");
        assert!(result.get("structuredContent").is_none(), "{reply:#}");
        let text: Value =
            serde_json::from_str(result["content"][0]["text"].as_str().unwrap()).unwrap();
        assert_eq!(text["isServiceFault"], true, "request {id}: {text:#}");
        assert_eq!(text["fault"]["kind"], "failed-validation", "{text:#}");
        assert_eq!(text["fault"]["operation"], operation, "{text:#}");
        if let Some(named) = field {
            assert_eq!(text["fault"]["field"], named, "request {id}: {text:#}");
        }
    }
}

#[test]
fn a_notification_a_broken_text_and_an_unknown_method_are_answered_as_serve_orders() {
    let Some(answers) = wire_answers() else {
        return;
    };
    let replies = answers.as_array().unwrap();
    let last = |back: usize| &replies[replies.len() - back]["reply"];
    assert!(last(4).is_null(), "a notification is answered: {}", last(4));
    let unreadable: Value = serde_json::from_str(last(3).as_str().unwrap()).unwrap();
    assert_eq!(
        unreadable,
        json!({ "error": { "code": -32_600_i32, "message": "Invalid Request" }, "id": null, "jsonrpc": "2.0" })
    );
    let unknown: Value = serde_json::from_str(last(2).as_str().unwrap()).unwrap();
    assert_eq!(unknown["error"]["code"], -32_601_i32, "{unknown:#}");
    let unmarked: Value = serde_json::from_str(last(1).as_str().unwrap()).unwrap();
    assert_eq!(unmarked["error"]["code"], -32_602_i32, "{unmarked:#}");
}

#[test]
fn the_http_server_applies_the_rules_in_the_order_the_design_gives() {
    let Some(driven) = run("mcp-http", &format!("{REQUESTS}\n{HTTP_DRIVER}")) else {
        return;
    };
    let answers = driven.as_array().unwrap();
    let statuses: Vec<u64> = answers
        .iter()
        .map(|held| held["status"].as_u64().unwrap())
        .collect();
    assert_eq!(
        statuses,
        [
            405, 403, 400, 400, 202, 400, 400, 400, 400, 200, 200, 200, 200, 400, 400, 404
        ],
        "{answers:#?}"
    );
    for held in answers {
        let status = held["status"].as_u64().unwrap();
        let body = held["body"].as_str().unwrap();
        if matches!(status, 405 | 403 | 202) {
            let allowed = if status == 405 {
                json!([["Allow", "POST"]])
            } else {
                json!([])
            };
            assert_eq!(body, "", "{held:#}");
            assert_eq!(held["headers"], allowed, "{held:#}");
            continue;
        }
        assert_eq!(
            held["headers"],
            json!([["Content-Type", "application/json"]]),
            "{held:#}"
        );
        held_as_answer(
            held["method"].as_str().unwrap(),
            &serde_json::from_str(body).unwrap(),
        );
    }
    let code = |at: usize| {
        serde_json::from_str::<Value>(answers[at]["body"].as_str().unwrap()).unwrap()["error"]
            ["code"]
            .as_i64()
    };
    assert_eq!(
        [2, 3, 5, 6, 7, 8].map(code),
        [
            Some(-32600),
            Some(-32600),
            Some(-32020),
            Some(-32020),
            Some(-32020),
            Some(-32020)
        ]
    );
    assert_eq!(
        [12, 13, 14, 15].map(code),
        [None, Some(-32602), Some(-32022), Some(-32601)]
    );
    let unknown: Value = serde_json::from_str(answers[11]["body"].as_str().unwrap()).unwrap();
    assert_eq!(unknown["error"]["code"], -32_602_i32, "{unknown:#}");
    let answered: Value = serde_json::from_str(answers[10]["body"].as_str().unwrap()).unwrap();
    assert_eq!(
        answered["result"]["structuredContent"],
        json!({ "version": 7_i32 }),
        "{answered:#}"
    );
}

#[test]
fn two_services_share_one_endpoint_and_a_schema_keeps_its_digits() {
    let Some(held) = run("mcp-endpoint", &format!("{REQUESTS}\n{ENDPOINT_DRIVER}")) else {
        return;
    };
    let replies: Vec<&str> = held["replies"]
        .as_array()
        .unwrap()
        .iter()
        .map(|reply| reply.as_str().unwrap())
        .collect();
    for (reply, method) in replies.iter().zip([
        "server/discover",
        "tools/list",
        "tools/call",
        "tools/call",
        "tools/call",
    ]) {
        held_as_answer(method, &serde_json::from_str(reply).unwrap());
    }
    let workspace = info("Workspace");
    assert_eq!(
        replies[0],
        sorted_text(&json!({
            "id": "d",
            "jsonrpc": "2.0",
            "result": {
                "_meta": workspace,
                "cacheScope": "public",
                "capabilities": { "tools": {} },
                "resultType": "complete",
                "supportedVersions": ["2026-07-28"],
                "ttlMs": 300_000_i32,
            },
        }))
    );
    let tools: Vec<String> = DocumentServiceSchema::mcp_tools()
        .iter()
        .chain(&BillingServiceSchema::mcp_tools())
        .map(Value::to_string)
        .collect();
    assert_eq!(
        replies[1],
        format!(
            "{{\"id\":1,\"jsonrpc\":\"2.0\",\"result\":{{\"_meta\":{},\"cacheScope\":\"public\",\"resultType\":\"complete\",\"tools\":[{}],\"ttlMs\":300000}}}}",
            sorted_text(&workspace),
            tools.join(",")
        )
    );
    let digits = BillingServiceSchema::mcp_tools()[0].to_string();
    assert!(digits.contains("9007199254740993"), "{digits}");
    assert!(
        replies[1].contains(&digits),
        "the tool text changed on the way: {}",
        replies[1]
    );
    let called: Value = serde_json::from_str(replies[2]).unwrap();
    assert_eq!(called["result"]["_meta"], workspace, "{called:#}");
    let invoiced: Value = serde_json::from_str(replies[3]).unwrap();
    assert_eq!(
        invoiced["result"]["structuredContent"],
        json!({ "invoices": ["inv-1", "limit=9007199254740993"] }),
        "{invoiced:#}"
    );
    let unknown: Value = serde_json::from_str(replies[4]).unwrap();
    assert_eq!(unknown["error"]["code"], -32_602_i32, "{unknown:#}");
    assert!(
        held["sharedMessage"]
            .as_str()
            .is_some_and(|said| said.contains("tool:get-version")),
        "{held:#}"
    );
}

#[test]
fn the_emitted_text_type_checks_under_strict_and_a_shared_name_does_not() {
    let Some(modules) = node_modules(REQUIRED_PACKAGES) else {
        stand_down_modules(REQUIRED_PACKAGES, "the type-check of the emitted MCP text");
        return;
    };
    let text = bundle();
    let Some((succeeded, said)) = compiled_with_modules(
        "mcp-types",
        COMPILER_VAR,
        &[
            ("bundle.ts", &text),
            ("callers.ts", ENDPOINT_CALLERS),
            ("usage.ts", USAGE_CALLER),
        ],
        &modules,
    ) else {
        return;
    };
    assert!(succeeded, "the emitted text does not type-check:\n{said}");
    let Some((refused, complaint)) = compiled_with_modules(
        "mcp-shared",
        COMPILER_VAR,
        &[("bundle.ts", &text), ("shared.ts", SHARED_NAME_CALLER)],
        &modules,
    ) else {
        return;
    };
    assert!(
        !refused,
        "an endpoint of two services sharing a name type-checks:\n{complaint}"
    );
    assert!(
        complaint.contains("twoServicesShare") && complaint.contains("tool:get-version"),
        "tsc does not name the shared name:\n{complaint}"
    );
}

/// Asserts that `outcome` is a fault of `kind` on `operation`, naming `detail` where it is given.
fn assert_fault(outcome: &Value, kind: &str, operation: &str, detail: Option<&str>) {
    assert_eq!(outcome["ok"], false, "{outcome:#}");
    assert_eq!(outcome["error"]["isServiceFault"], true, "{outcome:#}");
    let fault = &outcome["error"]["fault"];
    assert_eq!(fault["kind"], kind, "{outcome:#}");
    assert_eq!(fault["operation"], operation, "{outcome:#}");
    if let Some(said) = detail {
        assert_eq!(fault["detail"], said, "{outcome:#}");
    }
}

#[test]
fn the_client_reads_each_outcome_and_sends_what_the_server_reads() {
    let Some(held) = run("mcp-client", &format!("{REQUESTS}\n{CLIENT_DRIVER}")) else {
        return;
    };
    let outcomes = &held["outcomes"];
    let version = json!({ "ok": true, "value": { "version": 7_i32 } });
    let not_found = json!({ "ok": false, "error": { "errorCode": "not-found" } });
    let tagged = json!({ "ok": true, "value": [{ "version": 7_i32 }, "\"v7\""] });
    assert_eq!(outcomes["success"], version);
    assert_eq!(
        outcomes["faultInText"],
        json!({ "ok": false, "error": { "isServiceFault": true, "fault": {
            "detail": "the handler came apart", "kind": "handler-panic", "operation": "get-version",
        } } })
    );
    assert_eq!(outcomes["declared"], not_found);
    assert_eq!(outcomes["faultInError"], outcomes["faultInText"]);
    assert_fault(
        &outcomes["otherError"],
        "transport-failure",
        "get-version",
        Some("-32601: Method not found: tools/call"),
    );
    assert_fault(
        &outcomes["seamFails"],
        "transport-failure",
        "get-version",
        Some("the socket closed"),
    );
    assert_fault(
        &outcomes["notJson"],
        "undeserializable-payload",
        "get-version",
        None,
    );
    assert_fault(
        &outcomes["refusedValue"],
        "undeserializable-payload",
        "get-version",
        None,
    );
    assert_fault(
        &outcomes["absentHeader"],
        "undeserializable-payload",
        "get-tagged",
        None,
    );
    assert_fault(
        &outcomes["refusedBeforeSending"],
        "failed-validation",
        "get-version",
        None,
    );
    assert_eq!(outcomes["unit"], json!({ "ok": true }));
    assert_eq!(outcomes["header"], tagged);
    assert_eq!(outcomes["wrapped"], version);
    assert_eq!(outcomes["oneWay"], "void");
    assert_eq!(outcomes["oneWayThrows"]["fault"]["kind"], "handler-panic");
    let round = &outcomes["roundTrip"];
    assert_eq!(round["success"], version);
    assert_eq!(round["declared"], not_found);
    assert_eq!(
        round["panic"]["error"]["fault"]["kind"], "handler-panic",
        "{round:#}"
    );
    assert_eq!(round["header"], tagged);
    assert_eq!(round["declaredByHeader"], not_found);
    assert_eq!(round["wrapped"], version);
    assert_eq!(round["unit"], json!({ "ok": true }));
    assert_eq!(round["oneWay"], "void");
}

#[test]
fn the_client_sends_the_request_the_server_reads() {
    let Some(held) = run("mcp-client", &format!("{REQUESTS}\n{CLIENT_DRIVER}")) else {
        return;
    };
    let seen = held["seen"].as_array().unwrap();
    let first = &seen[0];
    assert_eq!(first["protocolVersion"], "2026-07-28");
    assert_eq!(first["method"], "tools/call");
    assert_eq!(first["name"], "get-version");
    let params = &first["request"]["params"];
    assert_eq!(params["arguments"], json!({ "document_id": "doc-1" }));
    assert_eq!(params["name"], "get-version");
    assert_eq!(
        params["_meta"]["io.modelcontextprotocol/protocolVersion"],
        "2026-07-28"
    );
    assert_eq!(
        params["_meta"]["io.modelcontextprotocol/clientCapabilities"],
        json!({})
    );
    assert_eq!(
        params["_meta"]["io.modelcontextprotocol/clientInfo"],
        json!({ "name": "DocumentService", "version": env!("CARGO_PKG_VERSION") })
    );
    held_to("CallToolRequest", &first["request"]);
    let sent_of = |tool: &str| {
        let found = seen.iter().find(|message| message["name"] == tool);
        assert!(found.is_some(), "no call of {tool}: {seen:#?}");
        found.unwrap()
    };
    let tagged_call = sent_of("get-tagged");
    assert_eq!(
        tagged_call["request"]["params"]["arguments"],
        json!({ "value": "doc-1", "if-none-match": "\"v6\"" })
    );
    held_to("CallToolRequest", &tagged_call["request"]);
    assert_eq!(
        sent_of("lookup-document")["request"]["params"]["arguments"],
        json!({ "value": "doc-1" })
    );
}

#[test]
fn the_stand_in_answers_what_the_driver_implementations_do() {
    let ctx = ();
    let request = || GetVersionRequest {
        document_id: "doc-1".to_owned(),
    };
    assert_eq!(
        answered(DocumentService::get_version(&Stand, &ctx, request()))
            .unwrap()
            .version,
        7
    );
    assert_eq!(
        answered(ArchiveService::get_version(&Stand, &ctx, request()))
            .unwrap()
            .version,
        7
    );
    assert_eq!(
        answered(DocumentService::lookup(
            &Stand,
            &ctx,
            DocumentId("doc-1".to_owned())
        ))
        .unwrap()
        .version,
        7
    );
    assert_eq!(
        answered(DocumentService::get_tagged(
            &Stand,
            &ctx,
            DocumentId("doc-1".to_owned()),
            None
        ))
        .unwrap()
        .1,
        "\"v7\""
    );
    answered(DocumentService::rename_document(
        &Stand,
        &ctx,
        "doc-1".to_owned(),
        "Plan".to_owned(),
    ))
    .unwrap_err();
    answered(DocumentService::touch_document(
        &Stand,
        &ctx,
        "doc-1".to_owned(),
        "reviewer-7".to_owned(),
    ));
    answered(BillingService::list_invoices(
        &Stand,
        &ctx,
        InvoiceRequest {
            limit: 1,
            load: 1.0,
        },
    ))
    .unwrap_err();
}

#[test]
fn the_published_text_holds_no_marker_the_accessor_left_unreplaced() {
    for text in [
        DocumentServiceSchema::ts_mcp_service(),
        DocumentServiceSchema::ts_mcp_client(),
        BillingServiceSchema::ts_mcp_service(),
        BillingServiceSchema::ts_mcp_client(),
    ] {
        assert!(
            !text.contains("__TIXSCHEMA_MCP_"),
            "a marker is left in the text"
        );
    }
}

/// What the Rust dispatcher of `DocumentService` answers to `row`.
fn rust_answer(row: &AuthRow) -> (u16, Vec<(String, String)>, String) {
    rust_auth_answer!(document_mcp, DocumentServiceSchema, Stand, row, answered)
}

fn auth_input() -> Value {
    json!({
        "origin": ORIGIN,
        "oauth": {
            "authorizationServers": [AUTHORIZATION_SERVER],
            "resource": RESOURCE,
            "resourceMetadataUrl": METADATA_URL,
            "scopesSupported": [SCOPES_SUPPORTED],
        },
        "rows": auth_rows().iter().map(|row| json!({
            "body": row.body,
            "headers": row.headers.iter().map(|(name, value)| json!([name, value])).collect::<Vec<_>>(),
            "method": row.method,
            "path": row.path,
            "scheme": row.scheme.word(),
        })).collect::<Vec<_>>(),
        "queries": SCOPE_QUERIES.iter().map(|(method, name)| json!([method, name])).collect::<Vec<_>>(),
    })
}

fn auth_report() -> Option<Value> {
    let driver = format!(
        "{REQUESTS}\nconst INPUT = {};\n{AUTH_DRIVER}",
        Value::String(auth_input().to_string())
    );
    run("mcp-auth", &driver)
}

#[test]
fn the_http_server_answers_every_authorization_case_as_the_rust_dispatcher_does() {
    let Some(report) = auth_report() else {
        return;
    };
    let rows = auth_rows();
    let answers = report["answers"].as_array().unwrap();
    assert_eq!(answers.len(), rows.len());
    for (row, node) in rows.iter().zip(answers) {
        let (status, headers, body) = rust_answer(row);
        assert_eq!(
            node["status"].as_u64().unwrap(),
            u64::from(status),
            "{}: {node}",
            row.name
        );
        let node_headers: Vec<(String, String)> = node["headers"]
            .as_array()
            .unwrap()
            .iter()
            .map(|pair| {
                (
                    pair[0].as_str().unwrap().to_owned(),
                    pair[1].as_str().unwrap().to_owned(),
                )
            })
            .collect();
        assert_eq!(node_headers, headers, "{}", row.name);
        assert_eq!(node["body"].as_str().unwrap(), body, "{}", row.name);
        if status != 200 {
            assert_eq!(node["reached"], 0_i32, "{}: no handler is called", row.name);
        }
    }
    let statuses: Vec<u64> = answers
        .iter()
        .map(|held| held["status"].as_u64().unwrap())
        .collect();
    assert_eq!(
        statuses,
        [
            200, 200, 405, 401, 401, 401, 401, 401, 200, 200, 403, 403, 403, 403, 403, 200, 400,
            401, 405, 401, 401, 403, 403, 200, 200, 403
        ]
    );
}

#[test]
fn the_scopes_function_answers_what_the_rust_registry_answers() {
    let Some(report) = auth_report() else {
        return;
    };
    let node = report["scopes"].as_array().unwrap();
    assert_eq!(node.len(), SCOPE_QUERIES.len());
    for ((method, name), held) in SCOPE_QUERIES.iter().zip(node) {
        let rust = DocumentServiceSchema::mcp_scopes(method, *name);
        assert_eq!(*held, json!(rust), "{method} {name:?}");
    }
    assert_eq!(
        DocumentServiceSchema::mcp_scopes("tools/call", Some("touch-document")),
        ["documents:read", "documents:touch"]
    );
}

#[test]
fn the_rust_dispatcher_answers_alone_beside_a_second_service_and_over_http_alike() {
    let (alone, served, status) =
        rust_serves_alike!(document_mcp, DocumentServiceSchema, Stand, answered);
    assert_eq!(alone, served);
    assert!(alone.is_some());
    assert_eq!(status, 200);
}
