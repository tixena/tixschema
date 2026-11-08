//! The TypeScript `mcp` server: the part a service puts on an endpoint, the endpoint with its
//! type-level shared-name check, `serve` and `serveHttp` for the services of an endpoint, and the
//! factories that drive `create{Service}Dispatcher` for one service.

use super::fault;
use super::message;
use super::service::enveloped_impl;
use crate::rename_rule::RenameRule;
use crate::service_schema::parse::{
    HttpShape, OperationDef, OperationInputs, OperationOutcome, ServiceDef, is_unit_type, written,
};
use crate::utils::ts_quoted;

/// The text `ts_mcp_service()` and `ts_mcp_client()` replace with the version of the crate that
/// declared the service, quoted as a string literal.
pub const VERSION_MARKER: &str = "/*__TIXSCHEMA_MCP_VERSION__*/";

/// The text `ts_mcp_service()` replaces with the service's `instructions`, quoted, or `undefined`.
pub const INSTRUCTIONS_MARKER: &str = "/*__TIXSCHEMA_MCP_INSTRUCTIONS__*/";

/// The text `ts_mcp_service()` replaces with the tools' JSON texts, each quoted, joined by commas.
pub const TOOLS_MARKER: &str = "/*__TIXSCHEMA_MCP_TOOLS__*/";

const TYPES: &str = "/** Everything a service puts on an endpoint: its names, and each list item as JSON text. */
export type __N__McpPart = {
  readonly names: ReadonlyArray<string>;
  readonly tools: ReadonlyArray<string>;
  readonly resources: ReadonlyArray<string>;
  readonly resourceTemplates: ReadonlyArray<string>;
  readonly prompts: ReadonlyArray<string>;
  readonly skills: ReadonlyArray<string>;
  readonly capabilities: ReadonlyArray<string>;
};

/** What a route answers for a request it owns: the `result` or the `error` of the JSON-RPC message. */
export type __N__McpAnswer = { result: unknown } | { error: unknown };

/** The names of a part `tsc` can read as literals; a name typed as `string` is checked when the endpoint is built. */
export type __N__McpNamesOf<P> = P extends { readonly names: ReadonlyArray<infer Name> }
  ? string extends Name
    ? never
    : Name
  : never;
export type __N__McpShared<Parts extends ReadonlyArray<unknown>, Seen = never> = Parts extends readonly [
  infer Head,
  ...infer Rest,
]
  ? (__N__McpNamesOf<Head> & Seen) | __N__McpShared<Rest, Seen | __N__McpNamesOf<Head>>
  : never;
/** `unknown` where no two parts share a name, and an object no part satisfies naming the shared names where two do. */
export type __N__McpNoSharedNames<Parts extends ReadonlyArray<unknown>> = [__N__McpShared<Parts>] extends [never]
  ? unknown
  : { readonly twoServicesShare: __N__McpShared<Parts> };";

const TEXT: &str = r#"/** A JSON text a value is already written as. */
class __N__McpRaw {
  readonly text: string;
  constructor(text: string) {
    this.text = text;
  }
}

/**
 * `value` as JSON text with the keys of every object in order, as serde_json writes them: a
 * `bigint` as its digits, a `Uint8Array` as the array of its bytes and a raw text as it stands.
 */
function __p__McpText(value: unknown): string {
  if (value instanceof __N__McpRaw) return value.text;
  if (typeof value === "bigint") return value.toString();
  if (value instanceof Uint8Array) return __p__McpText(Array.from(value));
  if (Array.isArray(value)) {
    return `[${value.map((held: unknown) => (held === undefined ? "null" : __p__McpText(held))).join(",")}]`;
  }
  if (typeof value === "object" && value !== null) {
    const prototype: unknown = Object.getPrototypeOf(value);
    if (prototype === Object.prototype || prototype === null) {
      const members: Array<string> = [];
      for (const key of Object.keys(value).sort()) {
        const held: unknown = (value as Record<string, unknown>)[key];
        if (held !== undefined && typeof held !== "function" && typeof held !== "symbol") {
          members.push(`${JSON.stringify(key)}:${__p__McpText(held)}`);
        }
      }
      return `{${members.join(",")}}`;
    }
  }
  return JSON.stringify(value) ?? "null";
}

function __p__McpObject(value: unknown): Record<string, unknown> | undefined {
  return typeof value === "object" && value !== null && !Array.isArray(value)
    ? (value as Record<string, unknown>)
    : undefined;
}"#;

const HANDLING: &str = r#"const __p__McpVersion = "2026-07-28";
const __p__McpMetaProtocol = "io.modelcontextprotocol/protocolVersion";
const __p__McpMetaCapabilities = "io.modelcontextprotocol/clientCapabilities";

/** One JSON-RPC message answered: the HTTP status it travels under over HTTP, and its text. */
type __N__McpHandled = { readonly status: number; readonly text: string };

function __p__McpReplied(status: number, body: Record<string, unknown>): __N__McpHandled {
  return { status, text: __p__McpText({ jsonrpc: "2.0", ...body }) };
}

function __p__McpRefused(
  id: unknown,
  status: number,
  code: number,
  message: string,
  data?: unknown,
): __N__McpHandled {
  return __p__McpReplied(status, { id, error: { code, data, message } });
}

function __p__McpParse(
  text: string,
): { ok: true; message: Record<string, unknown> } | { ok: false; refused: __N__McpHandled } {
  let read: unknown;
  try {
    read = __p__McpRead(text);
  } catch {
    read = undefined;
  }
  const message = __p__McpObject(read);
  if (message === undefined) {
    return { ok: false, refused: __p__McpRefused(null, 400, -32600, "Invalid Request") };
  }
  return { ok: true, message };
}

function __p__McpJoined(into: Record<string, unknown>, from: Record<string, unknown>): void {
  for (const [key, held] of Object.entries(from)) {
    const present = __p__McpObject(into[key]);
    const added = __p__McpObject(held);
    if (present !== undefined && added !== undefined) {
      __p__McpJoined(present, added);
    } else if (!(key in into)) {
      into[key] = held;
    }
  }
}

/** The union of the capabilities of the parts of an endpoint. */
function __p__McpCapabilities(endpoint: __N__McpEndpoint): Record<string, unknown> {
  const merged: Record<string, unknown> = {};
  for (const part of endpoint.parts) {
    for (const text of part.capabilities) {
      const held = __p__McpObject(__p__McpRead(text));
      if (held !== undefined) __p__McpJoined(merged, held);
    }
  }
  return merged;
}

async function __p__McpHandle(
  message: Record<string, unknown>,
  serverName: string,
  instructions: string | undefined,
  endpoint: __N__McpEndpoint,
  route: (part: number, method: string, params: unknown) => Promise<__N__McpAnswer | undefined>,
): Promise<__N__McpHandled> {
  const id = message.id;
  const method = message.method;
  if (typeof method !== "string") return __p__McpRefused(id, 400, -32600, "Invalid Request");
  const params = __p__McpObject(message.params);
  const meta = __p__McpObject(params?._meta) ?? {};
  for (const key of [__p__McpMetaProtocol, __p__McpMetaCapabilities]) {
    if (meta[key] === undefined) {
      return __p__McpRefused(id, 400, -32602, `Missing required _meta key: ${key}`);
    }
  }
  const requested = meta[__p__McpMetaProtocol];
  if (requested !== __p__McpVersion) {
    return __p__McpRefused(id, 400, -32022, "Unsupported protocol version", {
      requested,
      supported: [__p__McpVersion],
    });
  }
  const info = {
    "io.modelcontextprotocol/serverInfo": { name: serverName, version: __p__McpServerVersion },
  };
  switch (method) {
    case "server/discover":
      return __p__McpReplied(200, {
        id,
        result: {
          _meta: info,
          cacheScope: "public",
          capabilities: __p__McpCapabilities(endpoint),
          instructions,
          resultType: "complete",
          supportedVersions: [__p__McpVersion],
          ttlMs: 300000,
        },
      });
    case "tools/list":
      return __p__McpReplied(200, {
        id,
        result: {
          _meta: info,
          cacheScope: "public",
          resultType: "complete",
          tools: endpoint.parts.flatMap((part) => part.tools.map((tool) => new __N__McpRaw(tool))),
          ttlMs: 300000,
        },
      });
    case "tools/call": {
      for (let part = 0; part < endpoint.parts.length; part += 1) {
        const answer = await route(part, method, params);
        if (answer === undefined) continue;
        if ("error" in answer) return __p__McpReplied(200, { id, error: answer.error });
        const result = __p__McpObject(answer.result);
        return __p__McpReplied(200, {
          id,
          result: result === undefined ? answer.result : { ...result, _meta: info },
        });
      }
      const name = String(params?.name);
      return __p__McpRefused(id, 200, -32602, `Unknown tool: ${name}`, {
        fault: {
          detail: "the service answers to no operation by that name",
          kind: "unknown-operation",
          operation: name,
        },
        isServiceFault: true,
      });
    }
    default:
      return __p__McpRefused(id, 404, -32601, `Method not found: ${method}`);
  }
}"#;

const ENDPOINT: &str = r"export type __N__McpEndpoint = { readonly parts: ReadonlyArray<__N__McpPart> };

/** Fails to type-check when two parts share a name, and throws when called with names it cannot see. */
export function __p__McpEndpoint<const Parts extends ReadonlyArray<__N__McpPart>>(
  ...parts: Parts & __N__McpNoSharedNames<Parts>
): __N__McpEndpoint {
  const held: ReadonlyArray<__N__McpPart> = parts;
  const seen = new Set<string>();
  for (const part of held) {
    for (const name of part.names) {
      if (seen.has(name)) throw new Error(`two services share the name \`${name}\``);
    }
    for (const name of part.names) seen.add(name);
  }
  return { parts: held };
}";

const SERVE: &str = r#"/**
 * One JSON-RPC message in, one out, for the services of `endpoint`. `route` answers for the
 * service at that place in the endpoint, and a notification answers `undefined`.
 */
export async function __p__McpServe(
  text: string,
  serverName: string,
  instructions: string | undefined,
  endpoint: __N__McpEndpoint,
  route: (part: number, method: string, params: unknown) => Promise<__N__McpAnswer | undefined>,
): Promise<string | undefined> {
  const parsed = __p__McpParse(text);
  if (!parsed.ok) return parsed.refused.text;
  if (!("id" in parsed.message)) return undefined;
  return (await __p__McpHandle(parsed.message, serverName, instructions, endpoint, route)).text;
}"#;

const ROUTE: &str = r#"const __p__McpEmpty = { content: [], isError: false, resultType: "complete" };

/**
 * The route of `__N__`: answers a `tools/call` of one of its tools through `create__N__Dispatcher`,
 * so the Zod reads judge every message, and answers `undefined` for any other request.
 */
export function create__N__McpRoute<Ctx>(
  impl: __DRIVEN__,
): (ctx: Ctx, method: string, params: unknown) => Promise<__N__McpAnswer | undefined> {
  const dispatch = create__N__Dispatcher(impl);
  return async (ctx, method, params) => {
    const call = __p__McpObject(params);
    if (method !== "tools/call" || call === undefined) return undefined;
    const operation = __p__McpOperations.find((candidate) => candidate.tool === call.name);
    if (operation === undefined) return undefined;
    const given: unknown = call.arguments === undefined ? {} : call.arguments;
    const held = __p__McpObject(given);
    const headers: Array<readonly [string, string]> = [];
    let payload: unknown = given;
    if (held !== undefined) {
      const rest: Record<string, unknown> = { ...held };
      for (const name of operation.headersIn) {
        delete rest[name];
        const carried = held[name];
        if (carried !== undefined) headers.push([name, __p__McpText(carried)]);
      }
      payload = operation.wrapped ? held.value : rest;
    }
    const attempt = await dispatch(ctx, operation.operation, payload, headers).then(
      (dispatched) => ({ dispatched }),
      (thrown: unknown) => ({ thrown }),
    );
    if ("thrown" in attempt) {
      if (operation.oneWay) return { result: __p__McpEmpty };
      const detail = attempt.thrown instanceof Error ? attempt.thrown.message : String(attempt.thrown);
      return {
        error: {
          code: -32603,
          data: { fault: __FAULT__("handler-panic", operation.operation, detail), isServiceFault: true },
          message: `${operation.tool}: ${detail}`,
        },
      };
    }
    const { dispatched } = attempt;
    if (dispatched === undefined) return { result: __p__McpEmpty };
    const answered = dispatched.answered as { ok: boolean; value?: unknown; error?: unknown };
    const beside = (names: ReadonlyArray<string>): Record<string, unknown> => {
      const written: Record<string, unknown> = {};
      for (const name of names) {
        const text = dispatched.headers.find(([candidate]) => candidate === name)?.[1];
        if (text !== undefined) written[name] = new __N__McpRaw(text);
      }
      return written;
    };
    if (answered.ok) {
      let structured: unknown;
      if (operation.headersOut.length > 0) {
        structured = { value: answered.value ?? null, ...beside(operation.headersOut) };
      } else if (!operation.unit) {
        structured = answered.value ?? null;
      }
      return {
        result: {
          content: structured === undefined ? [] : [{ text: __p__McpText(structured), type: "text" }],
          isError: false,
          resultType: "complete",
          structuredContent: structured,
        },
      };
    }
    const error = answered.error;
    const fault = typeof error === "object" && error !== null && "isServiceFault" in error;
    const body =
      !fault && operation.errorHeadersOut.length > 0
        ? { value: error, ...beside(operation.errorHeadersOut) }
        : error;
    return {
      result: { content: [{ text: __p__McpText(body), type: "text" }], isError: true, resultType: "complete" },
    };
  };
}"#;

const HTTP_TYPES: &str = "/** What the HTTP endpoint checks of a request before it answers. */
export type __N__McpHttpSettings = {
  /** The `Origin` values a browser may send; a request carrying another is refused. */
  readonly allowedOrigins: ReadonlyArray<string>;
};

/** An HTTP request in plain terms. */
export type __N__McpHttpRequest = {
  readonly method: string;
  readonly headers: ReadonlyArray<readonly [string, string]>;
  readonly body: Uint8Array;
};

/** An HTTP response in plain terms. */
export type __N__McpHttpResponse = {
  readonly status: number;
  readonly headers: ReadonlyArray<readonly [string, string]>;
  readonly body: Uint8Array;
};";

const SERVE_HTTP: &str = r#"const __p__McpNamedBy = new Map<string, string>([
  ["tools/call", "name"],
  ["resources/read", "uri"],
  ["prompts/get", "name"],
]);

function __p__McpHeader(request: __N__McpHttpRequest, name: string): string | undefined {
  return request.headers.find(([candidate]) => candidate.toLowerCase() === name)?.[1];
}

/** A header value that may be written `=?base64?...?=`, read as the text it stands for. */
function __p__McpHeaderText(value: string): string | undefined {
  const encoded = /^=\?base64\?(.*)\?=$/.exec(value);
  if (encoded === null) return value;
  try {
    const binary = atob(encoded[1] ?? "");
    return new TextDecoder("utf-8", { fatal: true }).decode(Uint8Array.from(binary, (letter) => letter.charCodeAt(0)));
  } catch {
    return undefined;
  }
}

function __p__McpHttpReplied(handled: __N__McpHandled | number): __N__McpHttpResponse {
  if (typeof handled === "number") return { status: handled, headers: [], body: new Uint8Array() };
  return {
    status: handled.status,
    headers: [["Content-Type", "application/json"]],
    body: new TextEncoder().encode(handled.text),
  };
}

/**
 * The Streamable HTTP rules around `__p__McpServe`: the method, the `Origin`, the body, the headers
 * that must agree with it, and the status each refusal travels under.
 */
export async function __p__McpServeHttp(
  request: __N__McpHttpRequest,
  settings: __N__McpHttpSettings,
  serverName: string,
  instructions: string | undefined,
  endpoint: __N__McpEndpoint,
  route: (part: number, method: string, params: unknown) => Promise<__N__McpAnswer | undefined>,
): Promise<__N__McpHttpResponse> {
  if (request.method !== "POST") return __p__McpHttpReplied(405);
  const origin = __p__McpHeader(request, "origin");
  if (origin !== undefined && !settings.allowedOrigins.includes(origin)) return __p__McpHttpReplied(403);
  let text: string;
  try {
    text = new TextDecoder("utf-8", { fatal: true }).decode(request.body);
  } catch {
    return __p__McpHttpReplied(__p__McpRefused(null, 400, -32600, "Invalid Request"));
  }
  const parsed = __p__McpParse(text);
  if (!parsed.ok) return __p__McpHttpReplied(parsed.refused);
  const message = parsed.message;
  if (!("id" in message)) return __p__McpHttpReplied(202);
  const method = message.method;
  if (typeof method === "string") {
    const params = __p__McpObject(message.params);
    const expected: Array<readonly [string, unknown]> = [
      ["mcp-protocol-version", __p__McpObject(params?._meta)?.[__p__McpMetaProtocol]],
      ["mcp-method", method],
    ];
    const named = __p__McpNamedBy.get(method);
    if (named !== undefined) expected.push(["mcp-name", params?.[named]]);
    for (const [header, wanted] of expected) {
      const carried = __p__McpHeader(request, header);
      const read = carried === undefined ? undefined : header === "mcp-name" ? __p__McpHeaderText(carried) : carried;
      if (read === undefined || read !== wanted) {
        return __p__McpHttpReplied(__p__McpRefused(message.id, 400, -32020, `Header mismatch: ${header}`));
      }
    }
  }
  return __p__McpHttpReplied(await __p__McpHandle(message, serverName, instructions, endpoint, route));
}"#;

const FACTORIES: &str = "/**
 * Turns a `__N__` implementation into the function that answers one JSON-RPC message of the
 * service alone: usable as it is over stdio, one line in and one line out.
 */
export function create__N__McpDispatcher<Ctx>(
  impl: __DRIVEN__,
): (ctx: Ctx, text: string) => Promise<string | undefined> {
  const route = create__N__McpRoute(impl);
  const endpoint = __p__McpEndpoint(__p__McpPart);
  return (ctx, text) =>
    __p__McpServe(text, __p__McpServerName, __p__McpInstructions, endpoint, (part, method, params) =>
      part === 0 ? route(ctx, method, params) : Promise.resolve(undefined),
    );
}

/** `create__N__McpDispatcher` over Streamable HTTP. */
export function create__N__McpHttpDispatcher<Ctx>(
  impl: __DRIVEN__,
  settings: __N__McpHttpSettings,
): (ctx: Ctx, request: __N__McpHttpRequest) => Promise<__N__McpHttpResponse> {
  const route = create__N__McpRoute(impl);
  const endpoint = __p__McpEndpoint(__p__McpPart);
  return (ctx, request) =>
    __p__McpServeHttp(
      request,
      settings,
      __p__McpServerName,
      __p__McpInstructions,
      endpoint,
      (part, method, params) => (part === 0 ? route(ctx, method, params) : Promise.resolve(undefined)),
    );
}";

/// Stands in the emitted text for what `wrapped_{operation}()` answers.
pub fn wrapped_marker(operation: &OperationDef) -> String {
    format!(
        "/*__TIXSCHEMA_MCP_WRAPPED:{}__*/",
        written(&operation.ident)
    )
}

/// `template` with the service's name and its camelCase prefix written in.
pub fn filled(template: &str, named: &str, prefix: &str) -> String {
    template.replace("__N__", named).replace("__p__", prefix)
}

pub fn emit(declared: &ServiceDef) -> Vec<String> {
    let service = &declared.enveloped();
    let named = service.ident.to_string();
    let prefix = RenameRule::CamelCase.apply_to_variant(&named);
    let fill = |template: &str| filled(template, &named, &prefix);
    vec![
        constants(&named, &prefix),
        fill(TYPES),
        part(service, &named, &prefix),
        operations(service, &named, &prefix),
        message::read_fn(&format!("{prefix}McpRead")),
        fill(TEXT),
        fault_fn(&named, &prefix),
        fill(HANDLING),
        fill(ENDPOINT),
        fill(SERVE),
        route(service, &named, &prefix),
        fill(HTTP_TYPES),
        fill(SERVE_HTTP),
        factories(service, &named, &prefix),
    ]
}

fn constants(named: &str, prefix: &str) -> String {
    format!(
        "/** The name `server/discover` answers: the trait's own name. */\n\
         export const {prefix}McpServerName: string = {quoted};\n\n\
         /** The version of the crate that declared `{named}`, answered as `serverInfo.version`. */\n\
         export const {prefix}McpServerVersion: string = {VERSION_MARKER};\n\n\
         /** The trait's rustdoc, answered as `instructions` by `server/discover`. */\n\
         export const {prefix}McpInstructions: string | undefined = {INSTRUCTIONS_MARKER};",
        quoted = ts_quoted(named),
    )
}

fn part(service: &ServiceDef, named: &str, prefix: &str) -> String {
    let tool_names: Vec<String> = service
        .operations
        .iter()
        .map(|operation| ts_quoted(&format!("tool:{}", operation.mcp_tool_name())))
        .collect();
    let literal = tool_names.join(", ");
    format!(
        "/** Everything `{named}` puts on an endpoint: its names, and each list item as JSON text, \
         so a schema keeps the digits Rust wrote. */\n\
         export const {prefix}McpPart: {named}McpPart & {{ readonly names: readonly [{literal}] }} = {{\n  \
         names: [{literal}],\n  \
         tools: [{TOOLS_MARKER}],\n  \
         resources: [],\n  \
         resourceTemplates: [],\n  \
         prompts: [],\n  \
         skills: [],\n  \
         capabilities: [{capabilities}],\n\
         }};",
        capabilities = ts_quoted("{\"tools\":{}}"),
    )
}

fn operations(service: &ServiceDef, named: &str, prefix: &str) -> String {
    let rows: Vec<String> = service.operations.iter().map(operation_row).collect();
    format!(
        "/** What the route reads of each tool: the operation it calls, and how its arguments, \
         success and error are carried. */\n\
         export type {named}McpOperation = {{\n  \
         readonly tool: string;\n  \
         readonly operation: string;\n  \
         readonly wrapped: boolean;\n  \
         readonly oneWay: boolean;\n  \
         readonly unit: boolean;\n  \
         readonly headersIn: ReadonlyArray<string>;\n  \
         readonly headersOut: ReadonlyArray<string>;\n  \
         readonly errorHeadersOut: ReadonlyArray<string>;\n\
         }};\n\n\
         export const {prefix}McpOperations: ReadonlyArray<{named}McpOperation> = [\n{rows}\n];",
        rows = rows.join("\n"),
    )
}

fn operation_row(operation: &OperationDef) -> String {
    let shape = HttpShape::of(operation);
    let wrapped = match &operation.inputs {
        OperationInputs::Named(_) => wrapped_marker(operation),
        OperationInputs::Empty | OperationInputs::Generated(_) => "false".to_owned(),
    };
    let (one_way, unit) = match &operation.outcome {
        OperationOutcome::OneWay => (true, false),
        OperationOutcome::Reply {
            success,
            error: _error,
        } => (false, is_unit_type(&message::success_body(&shape, success))),
    };
    let quoted = |names: &[String]| {
        names
            .iter()
            .map(|name| ts_quoted(name))
            .collect::<Vec<_>>()
            .join(", ")
    };
    let headers_in: Vec<String> = shape
        .header_in
        .iter()
        .map(|bound| bound.name.clone())
        .collect();
    format!(
        "  {{ tool: {tool}, operation: {wire}, wrapped: {wrapped}, oneWay: {one_way}, unit: {unit}, \
         headersIn: [{headers_in}], headersOut: [{headers_out}], errorHeadersOut: \
         [{error_headers_out}] }},",
        tool = ts_quoted(operation.mcp_tool_name()),
        wire = ts_quoted(&operation.wire_name),
        headers_in = quoted(&headers_in),
        headers_out = quoted(&shape.header_out),
        error_headers_out = quoted(&shape.error_header_out),
    )
}

fn fault_fn(named: &str, prefix: &str) -> String {
    format!(
        "/** A `{named}Fault` of the kind given. */\n\
         function {prefix}McpFault(kind: {named}FaultKind, operation: string, detail: string): \
         {named}Fault {{\n\
         {minted}\n\
         }}",
        minted = fault::minted(
            named,
            "    detail,\n    field: undefined,\n    kind,\n    operation,"
        ),
    )
}

fn route(service: &ServiceDef, named: &str, prefix: &str) -> String {
    let driven = enveloped_impl(service);
    let fault = format!("{prefix}McpFault");
    ROUTE
        .replace("__DRIVEN__", &driven)
        .replace("__FAULT__", &fault)
        .replace("__N__", named)
        .replace("__p__", prefix)
}

fn factories(service: &ServiceDef, named: &str, prefix: &str) -> String {
    let driven = enveloped_impl(service);
    FACTORIES
        .replace("__DRIVEN__", &driven)
        .replace("__N__", named)
        .replace("__p__", prefix)
}
