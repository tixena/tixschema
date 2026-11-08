//! The TypeScript `mcp` server: the part a service puts on an endpoint, the endpoint with its
//! type-level shared-name check, `serve` and `serveHttp` for the services of an endpoint, and the
//! factories that drive `create{Service}Dispatcher` for one service.

use super::fault;
use super::mcp_scopes::{ReadUri, ScopeTable};
use super::message;
use super::service::enveloped_impl;
use crate::rename_rule::RenameRule;
use crate::service_schema::mcp_tools::{answers_key, is_content_pair};
use crate::service_schema::parse::{
    HttpShape, OperationDef, OperationInputs, OperationOutcome, PathSegment, ServiceDef,
    is_unit_type, wire_key, written,
};
use crate::utils::ts_quoted;
use core::fmt::Write as _;
use syn::Type;

const DEFAULT_TTL_MS: u64 = 0;
const DEFAULT_CACHE_SCOPE: &str = "private";

/// The text `ts_mcp_service()` and `ts_mcp_client()` replace with the version of the crate that
/// declared the service, quoted as a string literal.
pub const VERSION_MARKER: &str = "/*__TIXSCHEMA_MCP_VERSION__*/";

/// The text `ts_mcp_service()` replaces with the service's `instructions`, quoted, or `undefined`.
pub const INSTRUCTIONS_MARKER: &str = "/*__TIXSCHEMA_MCP_INSTRUCTIONS__*/";

/// The text `ts_mcp_service()` replaces with the tools' JSON texts, each quoted, joined by commas.
pub const TOOLS_MARKER: &str = "/*__TIXSCHEMA_MCP_TOOLS__*/";

/// The text `ts_mcp_service()` replaces with the resources' JSON texts, each quoted, joined by commas.
pub const RESOURCES_MARKER: &str = "/*__TIXSCHEMA_MCP_RESOURCES__*/";

/// The text `ts_mcp_service()` replaces with the resource templates' JSON texts, each quoted,
/// joined by commas.
pub const TEMPLATES_MARKER: &str = "/*__TIXSCHEMA_MCP_TEMPLATES__*/";

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

const WRITER: &str = r#"/** Where the transport writes a message beyond the answer: one line on stdio, one event on HTTP. A write that rejects means the client has gone. */
export type __N__McpWriter = (message: string) => Promise<void>;

/** The request in flight, handed to `admit` and to the function that builds `Ctx`, so that a handler reaches it. */
export class __N__McpCall {
  private readonly token: string | number | bigint | undefined;
  private readonly writer: __N__McpWriter;
  private gone = false;

  constructor(token?: string | number | bigint, writer?: __N__McpWriter) {
    this.token = token;
    this.writer = writer ?? (() => Promise.resolve());
  }

  /** Writes `notifications/progress` where the request carries a `progressToken`, and does nothing otherwise. A write that fails cancels the call. */
  async progress(done: number, total?: number, message?: string): Promise<void> {
    if (this.token === undefined || this.gone) return;
    const params: Record<string, unknown> = { progress: done, progressToken: this.token };
    if (total !== undefined) params.total = total;
    if (message !== undefined) params.message = message;
    try {
      await this.writer(__p__McpText({ jsonrpc: "2.0", method: "notifications/progress", params }));
    } catch {
      this.gone = true;
    }
  }

  /** Whether the client cancelled this request, or went away. */
  get cancelled(): boolean {
    return this.gone;
  }

  /** Marks the request cancelled. The transport calls it. */
  cancel(): void {
    this.gone = true;
  }
}

/** The requests a stdio host has in flight, so that `notifications/cancelled` reaches the right one. */
export class __N__McpInFlight {
  private readonly calls = new Map<string, __N__McpCall>();

  /** Marks the request `id` names as cancelled, and does nothing where none is in flight. */
  cancel(id: unknown): void {
    this.calls.get(JSON.stringify(id) ?? "")?.cancel();
  }

  /** Lists `call` under `id`; the function it answers takes it off the list. */
  enter(id: unknown, call: __N__McpCall): () => void {
    const key = JSON.stringify(id) ?? "";
    this.calls.set(key, call);
    return () => {
      if (this.calls.get(key) === call) this.calls.delete(key);
    };
  }
}

/** What `serve` writes to, and the requests it has in flight. */
export type __N__McpHost = { readonly writer: __N__McpWriter; readonly inFlight: __N__McpInFlight };

function __p__McpToken(message: Record<string, unknown>): string | number | bigint | undefined {
  const token = __p__McpObject(__p__McpObject(message.params)?._meta)?.progressToken;
  return typeof token === "string" || typeof token === "number" || typeof token === "bigint" ? token : undefined;
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

/** What a routed request asks for: the tool, the URI, or the URI of the template a completion is for. */
function __p__McpNamed(params: Record<string, unknown> | undefined, method: string): string | undefined {
  const text = (value: unknown): string | undefined => (typeof value === "string" ? value : undefined);
  if (method === "tools/call") return text(params?.name);
  if (method === "resources/read") return text(params?.uri);
  return text(__p__McpObject(params?.argument)?.name) === undefined
    ? undefined
    : text(__p__McpObject(params?.ref)?.uri);
}

type __N__McpListingStart =
  | { readonly part: number; readonly own: string | undefined; readonly first: boolean }
  | undefined;

/** Where a `resources/list` starts: the service, its own cursor, and whether this is the first page. */
function __p__McpListingStart(cursor: unknown, services: number): __N__McpListingStart {
  if (cursor === undefined || cursor === null) return { part: 0, own: undefined, first: true };
  if (typeof cursor !== "string") return undefined;
  const colon = cursor.indexOf(":");
  if (colon < 0) return undefined;
  const place = cursor.slice(0, colon);
  if (!/^\+?[0-9]+$/.test(place)) return undefined;
  const part = Number(place);
  if (part >= services) return undefined;
  const own = cursor.slice(colon + 1);
  return { part, own: own === "" ? undefined : own, first: false };
}

/** One page of `resources/list`: the resources with a fixed URI on the first page, then the page the first service that lists answered. */
async function __p__McpListResources(
  id: unknown,
  params: Record<string, unknown> | undefined,
  info: Record<string, unknown>,
  endpoint: __N__McpEndpoint,
  route: (part: number, method: string, params: unknown) => Promise<__N__McpAnswer | undefined>,
): Promise<__N__McpHandled> {
  const start = __p__McpListingStart(params?.cursor, endpoint.parts.length);
  if (start === undefined) return __p__McpRefused(id, 200, -32602, "Invalid cursor");
  let page: { readonly part: number; readonly answer: __N__McpAnswer } | undefined;
  for (let part = start.part; part < endpoint.parts.length; part += 1) {
    const asked = part === start.part && start.own !== undefined ? { cursor: start.own } : {};
    const answer = await route(part, "resources/list", asked);
    if (answer !== undefined) {
      page = { part, answer };
      break;
    }
  }
  const resources: Array<unknown> = start.first
    ? endpoint.parts.flatMap((part) => part.resources.map((resource) => new __N__McpRaw(resource)))
    : [];
  const result: Record<string, unknown> = { _meta: info, cacheScope: "private", resultType: "complete", ttlMs: 0 };
  if (page !== undefined) {
    if ("error" in page.answer) return __p__McpReplied(200, { id, error: page.answer.error });
    const listed = __p__McpObject(page.answer.result) ?? {};
    if (Array.isArray(listed.resources)) resources.push(...(listed.resources as ReadonlyArray<unknown>));
    for (const key of ["ttlMs", "cacheScope"]) {
      if (listed[key] !== undefined) result[key] = listed[key];
    }
    const own = typeof listed.nextCursor === "string" && listed.nextCursor !== "" ? listed.nextCursor : undefined;
    if (own !== undefined) {
      result.nextCursor = `${page.part}:${own}`;
    } else if (page.part + 1 < endpoint.parts.length) {
      result.nextCursor = `${page.part + 1}:`;
    }
  }
  result.resources = resources;
  return __p__McpReplied(200, { id, result });
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
    case "resources/templates/list":
      return __p__McpReplied(200, {
        id,
        result: {
          _meta: info,
          cacheScope: "public",
          resourceTemplates: endpoint.parts.flatMap((part) =>
            part.resourceTemplates.map((template) => new __N__McpRaw(template)),
          ),
          resultType: "complete",
          ttlMs: 300000,
        },
      });
    case "resources/list":
      return __p__McpListResources(id, params, info, endpoint, route);
    case "tools/call":
    case "resources/read":
    case "completion/complete": {
      const name = __p__McpNamed(params, method);
      if (name === undefined) {
        return __p__McpRefused(id, 200, -32602, `Invalid params: ${method} names nothing to answer`);
      }
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
      const unknown = method === "tools/call" ? "Unknown tool" : method === "resources/read" ? "Resource not found" : "Unknown completion";
      return __p__McpRefused(id, 200, -32602, `${unknown}: ${name}`, {
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
 * service at that place in the endpoint and receives the `Ctx` that `ctx` built from the request
 * in flight, and a notification answers `undefined`. `notifications/cancelled` marks the request
 * it names in `host.inFlight`, and the answer of a cancelled request is dropped.
 */
export async function __p__McpServe<Ctx>(
  text: string,
  serverName: string,
  instructions: string | undefined,
  endpoint: __N__McpEndpoint,
  ctx: (call: __N__McpCall) => Ctx,
  host: __N__McpHost,
  route: (ctx: Ctx, part: number, method: string, params: unknown) => Promise<__N__McpAnswer | undefined>,
): Promise<string | undefined> {
  const parsed = __p__McpParse(text);
  if (!parsed.ok) return parsed.refused.text;
  const message = parsed.message;
  if (!("id" in message)) {
    if (message.method === "notifications/cancelled") {
      host.inFlight.cancel(__p__McpObject(message.params)?.requestId);
    }
    return undefined;
  }
  const call = new __N__McpCall(__p__McpToken(message), host.writer);
  const entered: { leave?: () => void; ctx?: { readonly built: Ctx } } = {};
  try {
    const handled = await __p__McpHandle(message, serverName, instructions, endpoint, async (part, method, params) => {
      if (entered.ctx === undefined) {
        entered.leave = host.inFlight.enter(message.id, call);
        entered.ctx = { built: ctx(call) };
      }
      return route(entered.ctx.built, part, method, params);
    });
    return call.cancelled ? undefined : handled.text;
  } finally {
    entered.leave?.();
  }
}"#;

const INTERVIEW: &str = r#"function __p__McpPlain(text: string): string {
  return text.replace(/[-_ ]/g, "").toLowerCase();
}

/** Whether the client declared the elicitation `mode`: an empty `elicitation` is a form. */
function __p__McpElicits(call: Record<string, unknown>, mode: "form" | "url"): boolean {
  const held = __p__McpObject(__p__McpObject(__p__McpObject(call._meta)?.[__p__McpMetaCapabilities])?.elicitation);
  if (held === undefined) return false;
  if (mode === "url") return __p__McpObject(held.url) !== undefined;
  return __p__McpObject(held.form) !== undefined || held.url === undefined;
}

/** Whether the user declined or dismissed the request keyed `key`. */
function __p__McpDismissed(call: Record<string, unknown>, key: string): boolean {
  const action = __p__McpObject(__p__McpObject(call.inputResponses)?.[key])?.action;
  return action === "decline" || action === "cancel";
}

/** The result that asks the client for the elicitation `request`, keyed `key`. */
function __p__McpInputRequired(key: string, request: Record<string, unknown>): Record<string, unknown> {
  return { inputRequests: { [key]: { method: "elicitation/create", params: request } }, resultType: "input_required" };
}

/** The result that asks the user where `error` is the arm that asks and the client can be asked, `undefined` otherwise. */
function __p__McpAsking(
  interview: __N__McpInterview,
  error: unknown,
  call: Record<string, unknown>,
  declined: boolean,
): Record<string, unknown> | undefined {
  const arm = __p__McpObject(error);
  const code = arm?.errorCode;
  if (arm === undefined || typeof code !== "string" || __p__McpPlain(code) !== __p__McpPlain(interview.arm)) {
    return undefined;
  }
  const message = typeof arm.message === "string" ? arm.message : undefined;
  if (interview.answers !== undefined) {
    if (declined || !__p__McpElicits(call, "form")) return undefined;
    return __p__McpInputRequired(interview.answers, {
      message: message ?? interview.doc,
      mode: "form",
      requestedSchema: new __N__McpRaw(interview.form ?? "{}"),
    });
  }
  if (typeof arm.url !== "string" || message === undefined) return undefined;
  if (__p__McpDismissed(call, code) || !__p__McpElicits(call, "url")) return undefined;
  return __p__McpInputRequired(code, { message, mode: "url", url: arm.url });
}"#;

const CONTENT: &str = r#"/** One block of `content`, as the result writes it. */
function __p__McpBlock(block: unknown): Record<string, unknown> {
  const held = __p__McpObject(block) ?? {};
  if (held.type === "image" || held.type === "audio") {
    return { data: __p__McpBase64(held.data), mimeType: held.mimeType, type: held.type };
  }
  if (held.type === "resource") {
    const resource: Record<string, unknown> = { mimeType: held.mimeType, uri: held.uri };
    if (typeof held.text === "string") resource.text = held.text;
    else resource.blob = __p__McpBase64(held.blob);
    return { resource, type: "resource" };
  }
  return held;
}

/**
 * The blocks a content tool answers: the one image or audio block of a pair, or the blocks of a
 * list. A pair of any other content type has none, and answers as an operation with no flag does.
 */
function __p__McpBlocks(kind: "pair" | "list" | undefined, value: unknown): ReadonlyArray<Record<string, unknown>> | undefined {
  if (kind === "list" && Array.isArray(value)) return value.map(__p__McpBlock);
  if (kind !== "pair" || !Array.isArray(value) || typeof value[1] !== "string") return undefined;
  const type: string = value[1];
  const block = type.startsWith("image/") ? "image" : type.startsWith("audio/") ? "audio" : undefined;
  return block === undefined ? undefined : [{ data: __p__McpBase64(value[0]), mimeType: type, type: block }];
}"#;

const ROUTE: &str = r#"const __p__McpEmpty = { content: [], isError: false, resultType: "complete" };

/** The `-32603` answer of a handler that came apart. */
function __p__McpPanicked(operation: string, thrown: unknown): __N__McpAnswer {
  const detail = thrown instanceof Error ? thrown.message : String(thrown);
  return {
    error: {
      code: -32603,
      data: { fault: __FAULT__("handler-panic", operation, detail), isServiceFault: true },
      message: `${operation}: ${detail}`,
    },
  };
}

/** `value` in base64: a `Uint8Array`, or the array of bytes a dispatcher writes it as. */
function __p__McpBase64(value: unknown): string {
  const bytes = value instanceof Uint8Array ? value : Uint8Array.from(Array.isArray(value) ? (value as ReadonlyArray<number>) : []);
  let binary = "";
  for (let at = 0; at < bytes.length; at += 4096) binary += String.fromCharCode(...bytes.subarray(at, at + 4096));
  return btoa(binary);
}

/** What each placeholder of `segments` captures in `uri`: one non-empty run of text up to the next `/`. */
function __p__McpCaptured(
  segments: ReadonlyArray<readonly [string, boolean]>,
  uri: string,
): ReadonlyArray<string> | undefined {
  let rest = uri;
  const captured: Array<string> = [];
  for (const [text, placeholder] of segments) {
    if (placeholder) {
      const slash = rest.indexOf("/");
      const end = slash < 0 ? rest.length : slash;
      if (end === 0) return undefined;
      captured.push(rest.slice(0, end));
      rest = rest.slice(end);
    } else {
      if (!rest.startsWith(text)) return undefined;
      rest = rest.slice(text.length);
    }
  }
  return rest === "" ? captured : undefined;
}

/** A failed resource request: a fault is `Invalid params`, and a declared error is `refusal`, both a `-32602`. */
function __p__McpDeclined(error: unknown, refusal: string): __N__McpAnswer {
  const fault = typeof error === "object" && error !== null && "isServiceFault" in error;
  return { error: { code: -32602, data: error, message: fault ? "Invalid params" : refusal } };
}

/**
 * The route of `__N__`: answers a `tools/call` of one of its tools, a `resources/read`, its page of
 * `resources/list` and a `completion/complete` through `create__N__Dispatcher`, so the Zod reads
 * judge every message, and answers `undefined` for any other request.
 */
export function create__N__McpRoute<Ctx>(
  impl: __DRIVEN__,
): (ctx: Ctx, method: string, params: unknown) => Promise<__N__McpAnswer | undefined> {
  const dispatch = create__N__Dispatcher(impl);
  const attempted = (ctx: Ctx, operation: string, payload: unknown, headers: Array<readonly [string, string]>) =>
    dispatch(ctx, operation, payload, headers).then(
      (dispatched) => ({ dispatched }),
      (thrown: unknown) => ({ thrown }),
    );
  const calling = async (ctx: Ctx, call: Record<string, unknown>): Promise<__N__McpAnswer | undefined> => {
    const operation = __p__McpOperations.find((candidate) => candidate.kind === "tool" && candidate.tool === call.name);
    if (operation === undefined || operation.kind !== "tool") return undefined;
    const given: unknown = call.arguments === undefined ? {} : call.arguments;
    const held = __p__McpObject(given);
    const headers: Array<readonly [string, string]> = [];
    let payload: unknown = given;__DECLINED__
    if (held !== undefined) {
      const rest: Record<string, unknown> = { ...held };
      for (const name of operation.headersIn) {
        delete rest[name];
        const carried = held[name];
        if (carried !== undefined) headers.push([name, __p__McpText(carried)]);
      }
__ANSWERS__      payload = operation.wrapped ? held.value : rest;
    }
    const attempt = await attempted(ctx, operation.operation, payload, headers);
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
__BLOCKS__      let structured: unknown;
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
__ASKING__    const body =
      !fault && operation.errorHeadersOut.length > 0
        ? { value: error, ...beside(operation.errorHeadersOut) }
        : error;
    return {
      result: { content: [{ text: __p__McpText(body), type: "text" }], isError: true, resultType: "complete" },
    };
  };
  const answering = async (
    ctx: Ctx,
    operation: string,
    payload: unknown,
    refusal: string,
    success: (value: unknown) => Record<string, unknown>,
  ): Promise<__N__McpAnswer> => {
    const attempt = await attempted(ctx, operation, payload, []);
    if ("thrown" in attempt) return __p__McpPanicked(operation, attempt.thrown);
    const answered = attempt.dispatched?.answered as { ok: boolean; value?: unknown; error?: unknown } | undefined;
    if (answered === undefined || !answered.ok) return __p__McpDeclined(answered?.error, refusal);
    return { result: success(answered.value) };
  };
  const reading = async (ctx: Ctx, call: Record<string, unknown>): Promise<__N__McpAnswer | undefined> => {
    const uri = call.uri;
    if (typeof uri !== "string") return undefined;
    for (const operation of __p__McpOperations) {
      if (operation.kind !== "resource") continue;
      const captured = __p__McpCaptured(operation.segments, uri);
      if (captured === undefined) continue;
      const payload =
        operation.source === "empty"
          ? {}
          : operation.source === "none"
            ? null
            : operation.source === "single"
              ? captured[0]
              : Object.fromEntries(operation.members.map(([at, key]) => [key, captured[at]]));
      return answering(ctx, operation.operation, payload, "Resource not found", (value) => {
        const given: Record<string, unknown> = { uri };
        const pair = operation.blob ? (value as ReadonlyArray<unknown>) : [];
        if (operation.blob) {
          given.mimeType = pair[1];
          given.blob = __p__McpBase64(pair[0]);
        } else {
          given.mimeType = operation.mimeType;
          given.text = value;
        }
        return { cacheScope: operation.cacheScope, contents: [given], resultType: "complete", ttlMs: operation.ttlMs };
      });
    }
    return undefined;
  };
  const listing = async (ctx: Ctx, call: Record<string, unknown>): Promise<__N__McpAnswer | undefined> => {
    const operation = __p__McpOperations.find((candidate) => candidate.kind === "listing");
    if (operation === undefined || operation.kind !== "listing") return undefined;
    const payload = operation.cursor ? (typeof call.cursor === "string" ? call.cursor : null) : {};
    return answering(ctx, operation.operation, payload, "Resources could not be listed", (value) => ({
      ...(__p__McpObject(value) ?? {}),
      cacheScope: operation.cacheScope,
      resultType: "complete",
      ttlMs: operation.ttlMs,
    }));
  };
  const completing = async (ctx: Ctx, call: Record<string, unknown>): Promise<__N__McpAnswer | undefined> => {
    const reference = __p__McpObject(call.ref);
    const argument = __p__McpObject(call.argument);
    if (reference === undefined || reference.type !== "ref/resource") return undefined;
    const uri = reference.uri;
    const parameter = argument?.name;
    if (typeof uri !== "string" || typeof parameter !== "string") return undefined;
    const typed = typeof argument?.value === "string" ? argument.value : "";
    const context = __p__McpObject(call.context);
    const chosen = context !== undefined && "arguments" in context ? context.arguments : {};
    for (const operation of __p__McpOperations) {
      if (operation.kind !== "completion" || operation.uri !== uri || operation.parameter !== parameter) continue;
      return answering(
        ctx,
        operation.operation,
        { [operation.typed]: typed, [operation.chosen]: chosen },
        "Completion refused",
        (value) => {
          const suggested = Array.isArray(value) ? (value as ReadonlyArray<unknown>) : [];
          const completion: Record<string, unknown> = {
            hasMore: suggested.length > 100,
            values: suggested.slice(0, 100),
          };
          if (suggested.length > 100) completion.total = suggested.length;
          return { completion, resultType: "complete" };
        },
      );
    }
    return undefined;
  };
  return async (ctx, method, params) => {
    const call = __p__McpObject(params);
    if (call === undefined) return undefined;
    switch (method) {
      case "tools/call":
        return calling(ctx, call);
      case "resources/read":
        return reading(ctx, call);
      case "resources/list":
        return listing(ctx, call);
      case "completion/complete":
        return completing(ctx, call);
      default:
        return undefined;
    }
  };
}"#;

const ANSWERS: &str = r#"      const answers = operation.interview?.answers;
      if (answers !== undefined) {
        const response = __p__McpObject(__p__McpObject(call.inputResponses)?.[answers]);
        if (response !== undefined) {
          if (response.action === "accept") {
            rest[answers] = response.content ?? {};
          } else {
            delete rest[answers];
            declined = true;
          }
        }
      }
"#;

const BLOCKS: &str = r#"      const blocks = __p__McpBlocks(operation.content, answered.value);
      if (blocks !== undefined) {
        return { result: { content: blocks, isError: false, resultType: "complete" } };
      }
"#;

const ASKING: &str = "    if (!fault && operation.interview !== undefined) {
      const asked = __p__McpAsking(operation.interview, error, call, declined);
      if (asked !== undefined) return { result: asked };
    }
";

const HTTP_TYPES: &str = "/** What the HTTP endpoint checks of a request before it answers. */
export type __N__McpHttpSettings = {
  /** The `Origin` values a browser may send; a request carrying another is refused. */
  readonly allowedOrigins: ReadonlyArray<string>;
  /** Absent where the server authorizes by other means: no metadata document and no challenge. */
  readonly oauth?: __N__McpOAuth;
};

/** The protected-resource metadata of a server that authorizes by OAuth. */
export type __N__McpOAuth = {
  readonly authorizationServers: ReadonlyArray<string>;
  readonly resource: string;
  readonly resourceMetadataUrl: string;
  readonly scopesSupported: ReadonlyArray<string>;
};

/** Why `admit` refused a request. */
export type __N__McpDenied =
  /** `401`: no credentials, or credentials that do not hold. `scope` names what the called operation needs. */
  | { readonly kind: \"noValidCredentials\"; readonly scope?: string | undefined }
  /** `403` with a challenge: the credentials hold, but not the scopes `scope` names. */
  | { readonly kind: \"insufficientScope\"; readonly scope: string }
  /** `403` with no challenge. */
  | { readonly kind: \"forbidden\" };

/** An HTTP request in plain terms. `path` is the request's path, with or without its query. */
export type __N__McpHttpRequest = {
  readonly method: string;
  readonly path: string;
  readonly headers: ReadonlyArray<readonly [string, string]>;
  readonly body: Uint8Array;
};

/** An HTTP response in plain terms. */
export type __N__McpHttpResponse = {
  readonly status: number;
  readonly headers: ReadonlyArray<readonly [string, string]>;
  readonly body: Uint8Array;
};

/** What `admit` answers: the context the handlers run with, or why the request is refused. */
export type __N__McpAdmitted<Ctx> = { readonly ctx: Ctx } | { readonly denied: __N__McpDenied };";

const SERVE_HTTP: &str = r#"const __p__McpNamedBy = new Map<string, string>([
  ["tools/call", "name"],
  ["resources/read", "uri"],
  ["prompts/get", "name"],
]);

/** The value of the header called `name`, whatever its case. */
export function __p__McpHeader(request: __N__McpHttpRequest, name: string): string | undefined {
  const wanted = name.toLowerCase();
  return request.headers.find(([candidate]) => candidate.toLowerCase() === wanted)?.[1];
}

/** The token of an `Authorization: Bearer` header, whatever the case of the scheme. */
export function __p__McpBearerToken(request: __N__McpHttpRequest): string | undefined {
  const value = __p__McpHeader(request, "authorization");
  if (value === undefined) return undefined;
  const at = value.indexOf(" ");
  if (at < 0) return undefined;
  const token = value.slice(at + 1).trim();
  return value.slice(0, at).toLowerCase() === "bearer" && token !== "" ? token : undefined;
}

/** The `Mcp-Name` header, decoded from `=?base64?...?=` where it is written so. */
export function __p__McpName(request: __N__McpHttpRequest): string | undefined {
  const value = __p__McpHeader(request, "mcp-name");
  return value === undefined ? undefined : __p__McpHeaderText(value);
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

/** The `200` event-stream answer: each event written, then the answer. */
function __p__McpEventStream(events: ReadonlyArray<string>, answered: string | undefined): __N__McpHttpResponse {
  const written = answered === undefined ? events : [...events, answered];
  return {
    status: 200,
    headers: [
      ["Content-Type", "text/event-stream"],
      ["X-Accel-Buffering", "no"],
    ],
    body: new TextEncoder().encode(written.map((event) => `event: message\ndata: ${event}\n\n`).join("")),
  };
}

function __p__McpHttpReplied(handled: __N__McpHandled | number): __N__McpHttpResponse {
  if (typeof handled === "number") return { status: handled, headers: [], body: new Uint8Array() };
  return {
    status: handled.status,
    headers: [["Content-Type", "application/json"]],
    body: new TextEncoder().encode(handled.text),
  };
}

/** The progress token of an HTTP request, read before its body is judged. */
function __p__McpBodyToken(request: __N__McpHttpRequest): string | number | bigint | undefined {
  try {
    const message = __p__McpObject(__p__McpRead(new TextDecoder().decode(request.body)));
    return message === undefined ? undefined : __p__McpToken(message);
  } catch {
    return undefined;
  }
}

function __p__McpUrlPath(url: string): string {
  const separator = url.indexOf("://");
  const after = separator < 0 ? url : url.slice(separator + 3);
  const at = after.indexOf("/");
  const fromPath = at < 0 ? "/" : after.slice(at);
  return fromPath.split(/[?#]/)[0] ?? fromPath;
}

function __p__McpDenial(
  denied: __N__McpDenied,
  settings: __N__McpHttpSettings,
  request: __N__McpHttpRequest,
): __N__McpHttpResponse {
  const oauth = settings.oauth;
  let status = 403;
  let challenge: string | undefined;
  if (denied.kind === "noValidCredentials") {
    status = 401;
    if (oauth !== undefined) {
      const parts = [`resource_metadata="${oauth.resourceMetadataUrl}"`];
      if (denied.scope !== undefined) parts.push(`scope="${denied.scope}"`);
      if (__p__McpBearerToken(request) !== undefined) parts.push('error="invalid_token"');
      challenge = `Bearer ${parts.join(", ")}`;
    }
  } else if (denied.kind === "insufficientScope" && oauth !== undefined) {
    challenge = `Bearer error="insufficient_scope", scope="${denied.scope}", resource_metadata="${oauth.resourceMetadataUrl}"`;
  }
  const response = __p__McpHttpReplied(status);
  return challenge === undefined ? response : { ...response, headers: [["WWW-Authenticate", challenge]] };
}

function __p__McpMetadata(oauth: __N__McpOAuth): __N__McpHttpResponse {
  return {
    status: 200,
    headers: [["Content-Type", "application/json"]],
    body: new TextEncoder().encode(
      __p__McpText({
        authorization_servers: oauth.authorizationServers,
        bearer_methods_supported: ["header"],
        resource: oauth.resource,
        scopes_supported: oauth.scopesSupported.length === 0 ? undefined : oauth.scopesSupported,
      }),
    ),
  };
}

/**
 * The Streamable HTTP rules around `__p__McpServe`: the metadata document, the method, the `Origin`,
 * `admit`, the body, the headers that must agree with it, and the status each refusal travels
 * under. `route` also receives the `Ctx` that `admit` answered. A routed request that carries a
 * `progressToken` answers an event stream: each notification, then the answer.
 */
export async function __p__McpServeHttp<Ctx>(
  request: __N__McpHttpRequest,
  settings: __N__McpHttpSettings,
  serverName: string,
  instructions: string | undefined,
  endpoint: __N__McpEndpoint,
  admit: (request: __N__McpHttpRequest, call: __N__McpCall) => Promise<__N__McpAdmitted<Ctx>>,
  route: (ctx: Ctx, part: number, method: string, params: unknown) => Promise<__N__McpAnswer | undefined>,
): Promise<__N__McpHttpResponse> {
  if (settings.oauth !== undefined) {
    const path = request.path.split(/[?#]/)[0];
    if (request.method === "GET" && path === __p__McpUrlPath(settings.oauth.resourceMetadataUrl)) {
      return __p__McpMetadata(settings.oauth);
    }
  }
  if (request.method !== "POST") return { ...__p__McpHttpReplied(405), headers: [["Allow", "POST"]] };
  const origin = __p__McpHeader(request, "origin");
  if (origin !== undefined && !settings.allowedOrigins.includes(origin)) return __p__McpHttpReplied(403);
  const events: Array<string> = [];
  const call = new __N__McpCall(__p__McpBodyToken(request), (event) => {
    events.push(event);
    return Promise.resolve();
  });
  const admitted = await admit(request, call);
  if ("denied" in admitted) return __p__McpDenial(admitted.denied, settings, request);
  const { ctx } = admitted;
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
      ["MCP-Protocol-Version", __p__McpObject(params?._meta)?.[__p__McpMetaProtocol]],
      ["Mcp-Method", method],
    ];
    const named = __p__McpNamedBy.get(method);
    if (named !== undefined) expected.push(["Mcp-Name", params?.[named]]);
    for (const [header, wanted] of expected) {
      const carried = __p__McpHeader(request, header);
      const read = carried === undefined ? undefined : header === "Mcp-Name" ? __p__McpHeaderText(carried) : carried;
      if (read === undefined || read !== wanted) {
        return __p__McpHttpReplied(__p__McpRefused(message.id, 400, -32020, `Header mismatch: ${header}`));
      }
    }
  }
  const asked = { routed: false };
  const handled = await __p__McpHandle(message, serverName, instructions, endpoint, (part, called, given) => {
    asked.routed = true;
    return route(ctx, part, called, given);
  });
  if (!asked.routed) return __p__McpHttpReplied(handled);
  const answered = call.cancelled ? undefined : handled.text;
  if (__p__McpToken(message) !== undefined) return __p__McpEventStream(events, answered);
  return __p__McpHttpReplied(answered === undefined ? 202 : handled);
}"#;

const FACTORIES: &str = "/**
 * Turns a `__N__` implementation into the function that answers one JSON-RPC message of the
 * service alone: usable as it is over stdio, one line in and one line out. `ctx` builds the
 * handler's context from the request in flight, `writer` takes what the request writes beyond its
 * answer, and `inFlight` lists the requests that `notifications/cancelled` can reach.
 */
export function create__N__McpDispatcher<Ctx>(
  impl: __DRIVEN__,
): (
  text: string,
  ctx: (call: __N__McpCall) => Ctx,
  writer: __N__McpWriter,
  inFlight: __N__McpInFlight,
) => Promise<string | undefined> {
  const route = create__N__McpRoute(impl);
  const endpoint = __p__McpEndpoint(__p__McpPart);
  return (text, ctx, writer, inFlight) =>
    __p__McpServe(
      text,
      __p__McpServerName,
      __p__McpInstructions,
      endpoint,
      ctx,
      { writer, inFlight },
      (built, part, method, params) =>
        part === 0 ? route(built, method, params) : Promise.resolve(undefined),
    );
}

/** `create__N__McpDispatcher` over Streamable HTTP. */
export function create__N__McpHttpDispatcher<Ctx>(
  impl: __DRIVEN__,
  settings: __N__McpHttpSettings,
  admit: (request: __N__McpHttpRequest, call: __N__McpCall) => Promise<__N__McpAdmitted<Ctx>>,
): (request: __N__McpHttpRequest) => Promise<__N__McpHttpResponse> {
  const route = create__N__McpRoute(impl);
  const endpoint = __p__McpEndpoint(__p__McpPart);
  return (request) =>
    __p__McpServeHttp(
      request,
      settings,
      __p__McpServerName,
      __p__McpInstructions,
      endpoint,
      admit,
      (ctx, part, method, params) => (part === 0 ? route(ctx, method, params) : Promise.resolve(undefined)),
    );
}";

/// The type an `mcp(content)` operation answers a list of, as the service module declares it in
/// Rust: one block of a tool's `content`, in the form the specification writes it.
pub const CONTENT_TYPE: &str =
    "/** One block of a tool's `content`: what an `mcp(content)` operation answers a list of. */
export type McpContent =
  | { readonly type: \"text\"; readonly text: string }
  | { readonly type: \"image\"; readonly data: Uint8Array; readonly mimeType: string }
  | { readonly type: \"audio\"; readonly data: Uint8Array; readonly mimeType: string }
  | {
      readonly type: \"resource_link\";
      readonly uri: string;
      readonly name: string;
      readonly title?: string | undefined;
      readonly description?: string | undefined;
      readonly mimeType?: string | undefined;
    }
  | {
      readonly type: \"resource\";
      readonly uri: string;
      readonly mimeType?: string | undefined;
      readonly text: string;
    }
  | {
      readonly type: \"resource\";
      readonly uri: string;
      readonly mimeType?: string | undefined;
      readonly blob: Uint8Array;
    };";

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
    let lists = service
        .operations
        .iter()
        .any(|operation| operation.mcp.list_resources);
    let mut published = vec![constants(&named, &prefix), fill(TYPES)];
    if lists {
        published.push(page_types());
    }
    published.extend([
        part(service, &named, &prefix),
        operations(service, &named, &prefix),
        message::read_fn(&format!("{prefix}McpRead")),
        fill(TEXT),
        fault_fn(&named, &prefix),
        fill(WRITER),
        fill(HANDLING),
        fill(ENDPOINT),
        fill(SERVE),
        interview_text(service, &named, &prefix),
        content_text(service, &prefix),
        route(service, &named, &prefix),
        fill(HTTP_TYPES),
        scopes(service, &prefix),
        fill(SERVE_HTTP),
        factories(service, &named, &prefix),
    ]);
    published.retain(|text| !text.is_empty());
    published
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

/// The name an operation takes on an endpoint, `kind:name`: a tool, a resource or a template.
fn endpoint_name(operation: &OperationDef) -> Option<String> {
    if operation.is_mcp_tool() {
        return Some(format!("tool:{}", operation.mcp_tool_name()));
    }
    let resource = operation.mcp.resource.as_ref()?;
    let kind = if resource.is_template() {
        "resource template"
    } else {
        "resource"
    };
    Some(format!("{kind}:{}", resource.uri))
}

fn part(service: &ServiceDef, named: &str, prefix: &str) -> String {
    let literal = service
        .operations
        .iter()
        .filter_map(endpoint_name)
        .map(|name| ts_quoted(&name))
        .collect::<Vec<_>>()
        .join(", ");
    let served = service
        .operations
        .iter()
        .any(|operation| operation.mcp.resource.is_some() || operation.mcp.list_resources);
    let completes = service
        .operations
        .iter()
        .any(|operation| operation.mcp.completes.is_some());
    let mut capabilities = vec![ts_quoted("{\"tools\":{}}")];
    if served {
        capabilities.push(ts_quoted("{\"resources\":{}}"));
    }
    if completes {
        capabilities.push(ts_quoted("{\"completions\":{}}"));
    }
    format!(
        "/** Everything `{named}` puts on an endpoint: its names, and each list item as JSON text, \
         so a schema keeps the digits Rust wrote. */\n\
         export const {prefix}McpPart: {named}McpPart & {{ readonly names: readonly [{literal}] }} = {{\n  \
         names: [{literal}],\n  \
         tools: [{TOOLS_MARKER}],\n  \
         resources: [{RESOURCES_MARKER}],\n  \
         resourceTemplates: [{TEMPLATES_MARKER}],\n  \
         prompts: [],\n  \
         skills: [],\n  \
         capabilities: [{capabilities}],\n\
         }};",
        capabilities = capabilities.join(", "),
    )
}

/// `McpResource` and `McpResourcePage`, which a listing answers and the Rust service module declares.
/// Interfaces, so two services of one bundle that both list declare them without a clash.
fn page_types() -> String {
    "/** One resource of a listing, as `resources/list` writes it. */\n\
     export interface McpResource {\n  \
     uri: string;\n  \
     name: string;\n  \
     title?: string;\n  \
     description?: string;\n  \
     mimeType?: string;\n  \
     size?: number;\n\
     }\n\n\
     /** One page of `resources/list`, with the cursor of the next where there is one. */\n\
     export interface McpResourcePage {\n  \
     resources: Array<McpResource>;\n  \
     nextCursor?: string;\n\
     }"
    .to_owned()
}

fn operations(service: &ServiceDef, named: &str, prefix: &str) -> String {
    let mut rows: Vec<String> = service
        .operations
        .iter()
        .filter(|operation| operation.is_mcp_tool())
        .map(operation_row)
        .collect();
    rows.extend(resource_rows(service));
    format!(
        "/** What a tool that asks the user is told: the arm of its declared error that asks, the \
         wire key of the argument that carries a form's answers and the JSON text of the form, \
         and the message to ask with where the arm holds none. */\n\
         export type {named}McpInterview = {{\n  \
         readonly arm: string;\n  \
         readonly answers: string | undefined;\n  \
         readonly form: string | undefined;\n  \
         readonly doc: string;\n\
         }};\n\n\
         /** What the route reads of each operation: the tool, resource, listing or completion it \
         answers, the operation it calls, and how its arguments, success and error are carried. */\n\
         export type {named}McpOperation =\n  \
         | {{\n      \
         readonly kind: \"tool\";\n      \
         readonly tool: string;\n      \
         readonly operation: string;\n      \
         readonly wrapped: boolean;\n      \
         readonly oneWay: boolean;\n      \
         readonly unit: boolean;\n      \
         readonly headersIn: ReadonlyArray<string>;\n      \
         readonly headersOut: ReadonlyArray<string>;\n      \
         readonly errorHeadersOut: ReadonlyArray<string>;\n      \
         readonly content: \"pair\" | \"list\" | undefined;\n      \
         readonly interview: {named}McpInterview | undefined;\n    \
         }}\n  \
         | {{\n      \
         readonly kind: \"resource\";\n      \
         readonly uri: string;\n      \
         readonly operation: string;\n      \
         readonly segments: ReadonlyArray<readonly [string, boolean]>;\n      \
         readonly source: \"empty\" | \"none\" | \"single\" | \"members\";\n      \
         readonly members: ReadonlyArray<readonly [number, string]>;\n      \
         readonly blob: boolean;\n      \
         readonly mimeType: string | undefined;\n      \
         readonly ttlMs: number;\n      \
         readonly cacheScope: string;\n    \
         }}\n  \
         | {{\n      \
         readonly kind: \"listing\";\n      \
         readonly operation: string;\n      \
         readonly cursor: boolean;\n      \
         readonly ttlMs: number;\n      \
         readonly cacheScope: string;\n    \
         }}\n  \
         | {{\n      \
         readonly kind: \"completion\";\n      \
         readonly operation: string;\n      \
         readonly uri: string;\n      \
         readonly parameter: string;\n      \
         readonly typed: string;\n      \
         readonly chosen: string;\n    \
         }};\n\n\
         export const {prefix}McpOperations: ReadonlyArray<{named}McpOperation> = [\n{rows}\n];",
        rows = rows.join("\n"),
    )
}

/// The rows of the resources, a fixed URI ahead of a template, then the listing and the completions.
fn resource_rows(service: &ServiceDef) -> Vec<String> {
    let resourced: Vec<&OperationDef> = service
        .operations
        .iter()
        .filter(|operation| !operation.is_mcp_tool())
        .collect();
    let reads = |template: bool| {
        resourced
            .iter()
            .filter(move |operation| {
                operation
                    .mcp
                    .resource
                    .as_ref()
                    .is_some_and(|resource| resource.is_template() == template)
            })
            .map(|operation| resource_row(operation))
    };
    let listing = resourced
        .iter()
        .find(|operation| operation.mcp.list_resources)
        .map(|operation| listing_row(operation));
    let completions = resourced
        .iter()
        .filter_map(|operation| completion_row(service, operation));
    reads(false)
        .chain(reads(true))
        .chain(listing)
        .chain(completions)
        .collect()
}

fn resource_row(operation: &OperationDef) -> String {
    let Some(resource) = operation.mcp.resource.as_ref() else {
        return String::new();
    };
    let segments = if resource.is_template() {
        resource
            .segments
            .iter()
            .map(|segment| match segment {
                PathSegment::Literal(text) => format!("[{}, false]", ts_quoted(text)),
                PathSegment::Placeholder(name) => format!("[{}, true]", ts_quoted(name)),
            })
            .collect::<Vec<_>>()
            .join(", ")
    } else {
        format!("[{}, false]", ts_quoted(&resource.uri))
    };
    let names = resource.placeholder_names();
    let (source, members) = match &operation.inputs {
        OperationInputs::Empty => ("empty", String::new()),
        OperationInputs::Named(_) if names.is_empty() => ("none", String::new()),
        OperationInputs::Named(_) => ("single", String::new()),
        OperationInputs::Generated(fields) => (
            "members",
            names
                .iter()
                .enumerate()
                .filter_map(|(at, name)| {
                    let (field, _) = fields.iter().find(|(field, _)| written(field) == *name)?;
                    Some(format!("[{at}, {}]", ts_quoted(&wire_key(field))))
                })
                .collect::<Vec<_>>()
                .join(", "),
        ),
    };
    let blob = matches!(
        &operation.outcome,
        OperationOutcome::Reply { success, error: _error } if matches!(success.as_ref(), Type::Tuple(_))
    );
    format!(
        "  {{ kind: \"resource\", uri: {uri}, operation: {wire}, segments: [{segments}], \
         source: \"{source}\", members: [{members}], blob: {blob}, mimeType: {mime}, \
         ttlMs: {ttl}, cacheScope: {scope} }},",
        uri = ts_quoted(&resource.uri),
        wire = ts_quoted(&operation.wire_name),
        mime = operation
            .mcp
            .mime_type
            .as_deref()
            .map_or_else(|| "undefined".to_owned(), ts_quoted),
        ttl = operation.mcp.ttl_ms.unwrap_or(DEFAULT_TTL_MS),
        scope = ts_quoted(
            operation
                .mcp
                .cache_scope
                .as_deref()
                .unwrap_or(DEFAULT_CACHE_SCOPE)
        ),
    )
}

fn listing_row(operation: &OperationDef) -> String {
    format!(
        "  {{ kind: \"listing\", operation: {wire}, cursor: {cursor}, ttlMs: {ttl}, \
         cacheScope: {scope} }},",
        wire = ts_quoted(&operation.wire_name),
        cursor = matches!(operation.inputs, OperationInputs::Named(_)),
        ttl = operation.mcp.ttl_ms.unwrap_or(DEFAULT_TTL_MS),
        scope = ts_quoted(
            operation
                .mcp
                .cache_scope
                .as_deref()
                .unwrap_or(DEFAULT_CACHE_SCOPE)
        ),
    )
}

fn completion_row(service: &ServiceDef, operation: &OperationDef) -> Option<String> {
    let completes = operation.mcp.completes.as_ref()?;
    let target = service
        .operations
        .iter()
        .find(|candidate| written(&candidate.ident) == completes.resource)?;
    let uri = &target.mcp.resource.as_ref()?.uri;
    let OperationInputs::Generated(fields) = &operation.inputs else {
        return None;
    };
    Some(format!(
        "  {{ kind: \"completion\", operation: {wire}, uri: {uri}, parameter: {parameter}, \
         typed: {typed}, chosen: {chosen} }},",
        wire = ts_quoted(&operation.wire_name),
        uri = ts_quoted(uri),
        parameter = ts_quoted(&completes.parameter),
        typed = ts_quoted(&wire_key(&fields.first()?.0)),
        chosen = ts_quoted(&wire_key(&fields.get(1)?.0)),
    ))
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
        "  {{ kind: \"tool\", tool: {tool}, operation: {wire}, wrapped: {wrapped}, oneWay: {one_way}, unit: {unit}, \
         headersIn: [{headers_in}], headersOut: [{headers_out}], errorHeadersOut: \
         [{error_headers_out}], content: {content}, interview: {interview} }},",
        content = content_kind(operation),
        interview = interview_literal(operation),
        tool = ts_quoted(operation.mcp_tool_name()),
        wire = ts_quoted(&operation.wire_name),
        headers_in = quoted(&headers_in),
        headers_out = quoted(&shape.header_out),
        error_headers_out = quoted(&shape.error_header_out),
    )
}

/// Stands in the emitted text for the JSON text of the form `mcp_form_{operation}()` answers.
pub fn form_marker(operation: &OperationDef) -> String {
    format!("/*__TIXSCHEMA_MCP_FORM:{}__*/", written(&operation.ident))
}

/// What the route writes of an `mcp(content)` operation.
fn content_kind(operation: &OperationDef) -> &'static str {
    if is_content_pair(operation) {
        "\"pair\""
    } else if operation.mcp.content.is_some() {
        "\"list\""
    } else {
        "undefined"
    }
}

fn interview_literal(operation: &OperationDef) -> String {
    let Some(interview) = operation.mcp.interview.as_ref() else {
        return "undefined".to_owned();
    };
    let answers =
        answers_key(operation).map_or_else(|| "undefined".to_owned(), |key| ts_quoted(&key));
    let form = if interview.answers.is_some() {
        form_marker(operation)
    } else {
        "undefined".to_owned()
    };
    format!(
        "{{ arm: {}, answers: {answers}, form: {form}, doc: {} }}",
        ts_quoted(&interview.arm.to_string()),
        ts_quoted(operation.doc.as_deref().unwrap_or_default()),
    )
}

fn interview_text(service: &ServiceDef, named: &str, prefix: &str) -> String {
    if service
        .operations
        .iter()
        .any(|operation| operation.mcp.interview.is_some())
    {
        filled(INTERVIEW, named, prefix)
    } else {
        String::new()
    }
}

fn content_text(service: &ServiceDef, prefix: &str) -> String {
    if service
        .operations
        .iter()
        .any(|operation| operation.mcp.content.is_some())
    {
        filled(CONTENT, "", prefix)
    } else {
        String::new()
    }
}

fn scopes(service: &ServiceDef, prefix: &str) -> String {
    let table = ScopeTable::of(service);
    let list = |scopes: &[String]| {
        format!(
            "[{}]",
            scopes
                .iter()
                .map(|scope| ts_quoted(scope))
                .collect::<Vec<_>>()
                .join(", ")
        )
    };
    let mut body = String::new();
    if table.has_templates() {
        body.push_str(
            "  const matches = (uri: string, template: ReadonlyArray<readonly [string, boolean]>): boolean => {\n    \
             let rest = uri;\n    \
             for (const [text, placeholder] of template) {\n      \
             if (placeholder) {\n        \
             const slash = rest.indexOf(\"/\");\n        \
             const end = slash < 0 ? rest.length : slash;\n        \
             if (end === 0) return false;\n        \
             rest = rest.slice(end);\n      \
             } else if (rest.startsWith(text)) {\n        \
             rest = rest.slice(text.length);\n      \
             } else {\n        \
             return false;\n      \
             }\n    \
             }\n    \
             return rest.length === 0;\n  \
             };\n",
        );
    }
    if !table.tools.is_empty() {
        body.push_str("  if (method === \"tools/call\") {\n    switch (name) {\n");
        for (tool, needed) in &table.tools {
            let _ = writeln!(
                body,
                "      case {}:\n        return {};",
                ts_quoted(tool),
                list(needed)
            );
        }
        body.push_str("      default:\n        return [];\n    }\n  }\n");
    }
    if !table.reads.is_empty() {
        body.push_str("  if (method === \"resources/read\" && name !== undefined) {\n");
        for (uri, needed) in &table.reads {
            match uri {
                ReadUri::Fixed(fixed) => {
                    let _ = writeln!(
                        body,
                        "    if (name === {}) return {};",
                        ts_quoted(fixed),
                        list(needed)
                    );
                }
                ReadUri::Template(parts) => {
                    let pairs: Vec<String> = parts
                        .iter()
                        .map(|(part, placeholder)| format!("[{}, {placeholder}]", ts_quoted(part)))
                        .collect();
                    let _ = writeln!(
                        body,
                        "    if (matches(name, [{}] as const)) return {};",
                        pairs.join(", "),
                        list(needed)
                    );
                }
            }
        }
        body.push_str("    return [];\n  }\n");
    }
    if let Some(needed) = &table.listing {
        let _ = writeln!(
            body,
            "  if (method === \"resources/list\") return {};",
            list(needed)
        );
    }
    if !table.completions.is_empty() {
        body.push_str("  if (method === \"completion/complete\" && name !== undefined) {\n");
        for (uri, needed) in &table.completions {
            let _ = writeln!(
                body,
                "    if (name === {}) return {};",
                ts_quoted(uri),
                list(needed)
            );
        }
        body.push_str("  }\n");
    }
    format!(
        "/** The scopes a caller needs for the request `method` and `name` name, none for any other. */\n\
         export function {prefix}McpScopes(method: string, name: string | undefined): ReadonlyArray<string> {{\n\
         {body}  return [];\n\
         }}"
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
    let asks = service
        .operations
        .iter()
        .any(|operation| operation.mcp.interview.is_some());
    let answers = service.operations.iter().any(|operation| {
        operation
            .mcp
            .interview
            .as_ref()
            .is_some_and(|interview| interview.answers.is_some())
    });
    let blocks = service
        .operations
        .iter()
        .any(|operation| operation.mcp.content.is_some());
    let declined = if asks {
        "\n    let declined = false;"
    } else {
        ""
    };
    ROUTE
        .replace("__DECLINED__", declined)
        .replace("__ANSWERS__", if answers { ANSWERS } else { "" })
        .replace("__BLOCKS__", if blocks { BLOCKS } else { "" })
        .replace("__ASKING__", if asks { ASKING } else { "" })
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
