//! The TypeScript `mcp` server: the part a service puts on an endpoint, the endpoint with its
//! type-level shared-name check, `serve` and `serveHttp` for the services of an endpoint, and the
//! factories that drive `create{Service}Dispatcher` for one service.

use super::fault;
use super::mcp_scopes::{ReadUri, ScopeTable};
use super::message;
use super::service::enveloped_impl;
use crate::field_type::{EXPECTED_NUMERIC_KEY, EXPECTED_QUOTE};
use crate::rename_rule::RenameRule;
use crate::service_schema::mcp_tools::{answers_key, is_content_pair};
use crate::service_schema::parse::{
    HttpShape, McpCompletesKind, OperationDef, OperationInputs, OperationOutcome, PathSegment,
    ServiceDef, is_unit_type, wire_key, written,
};
use crate::service_schema::skills::written_frontmatter;
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

/// The text `ts_mcp_service()` replaces with the prompts' JSON texts, each quoted, joined by commas.
pub const PROMPTS_MARKER: &str = "/*__TIXSCHEMA_MCP_PROMPTS__*/";

/// The text `ts_mcp_service()` replaces with the skills' JSON texts, each quoted, joined by commas.
pub const SKILLS_MARKER: &str = "/*__TIXSCHEMA_MCP_SKILLS__*/";

/// The text `ts_mcp_service()` replaces with the files of the skills folder, each a quoted URI and
/// the quoted JSON text of its content, joined by commas.
pub const SKILL_FILES_MARKER: &str = "/*__TIXSCHEMA_MCP_SKILL_FILES__*/";

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

/** What the application's source of changes hands to a `subscriptions/listen` stream. */
export type __N__McpChange =
  /** The list of resources changed. */
  | { readonly kind: \"resourcesListChanged\" }
  /** One resource changed. */
  | { readonly kind: \"resourceUpdated\"; readonly uri: string }
  /** One task moved: where it stands now, as `McpTaskState` writes it. */
  | { readonly kind: \"task\"; readonly taskId: string; readonly state: Readonly<Record<string, unknown>> }
  /** Keeps an event stream open: the comment line `:` on HTTP, nothing on stdio. */
  | { readonly kind: \"keepAlive\" };

/** The application's source of changes: each call answers the next change, and `undefined` ends the stream. */
export type __N__McpChanges = () => Promise<__N__McpChange | undefined>;

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
  private readonly comment: () => Promise<void>;
  private gone = false;
  private release: () => void = () => {};
  /** Resolves when the client cancels this request or goes away. */
  readonly stopped: Promise<void>;

  constructor(token?: string | number | bigint, writer?: __N__McpWriter, comment?: () => Promise<void>) {
    this.token = token;
    this.writer = writer ?? (() => Promise.resolve());
    this.comment = comment ?? (() => Promise.resolve());
    this.stopped = new Promise<void>((resolve) => {
      this.release = resolve;
    });
  }

  /** Writes `notifications/progress` where the request carries a `progressToken`, and does nothing otherwise. A write that fails cancels the call. */
  async progress(done: number, total?: number, message?: string): Promise<void> {
    if (this.token === undefined || this.gone) return;
    const params: Record<string, unknown> = { progress: done, progressToken: this.token };
    if (total !== undefined) params.total = total;
    if (message !== undefined) params.message = message;
    await this.write(__p__McpText({ jsonrpc: "2.0", method: "notifications/progress", params }));
  }

  /** Writes one message beyond the answer. A write that fails cancels the call, and answers `false`. */
  async write(message: string): Promise<boolean> {
    if (this.gone) return false;
    try {
      await this.writer(message);
      return true;
    } catch {
      this.cancel();
      return false;
    }
  }

  /** Writes the comment line `:` that keeps an event stream open. A write that fails cancels the call, and answers `false`. */
  async keepAlive(): Promise<boolean> {
    if (this.gone) return false;
    try {
      await this.comment();
      return true;
    } catch {
      this.cancel();
      return false;
    }
  }

  /** Whether the client cancelled this request, or went away. */
  get cancelled(): boolean {
    return this.gone;
  }

  /** Marks the request cancelled. The transport calls it. */
  cancel(): void {
    this.gone = true;
    this.release();
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
export type __N__McpHost = {
  readonly writer: __N__McpWriter;
  readonly inFlight: __N__McpInFlight;
  /** Where `subscriptions/listen` takes its changes from; absent where the host offers none. */
  readonly changes?: __N__McpChanges;
};

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

/** A reply whose public answer is private where the server's name came from the caller. */
function __p__McpPrivately(body: Record<string, unknown>, named: boolean): Record<string, unknown> {
  const result = __p__McpObject(body.result);
  return named && result?.cacheScope === "public" ? { ...body, result: { ...result, cacheScope: "private" } } : body;
}

function __p__McpRefused(
  id: unknown,
  status: number,
  code: number,
  message: string,
  data?: unknown,
): __N__McpHandled {
  return __p__McpReplied(status, { id: id ?? undefined, error: { code, data, message } });
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
function __p__McpCapabilities(endpoint: __N__McpEndpoint, listens: boolean): Record<string, unknown> {
  const merged: Record<string, unknown> = {};
  for (const part of endpoint.parts) {
    for (const text of part.capabilities) {
      const held = __p__McpObject(__p__McpRead(text));
      if (held !== undefined) __p__McpJoined(merged, held);
    }
  }
  const resources = __p__McpObject(merged.resources);
  if (listens && resources !== undefined) {
    resources.listChanged = true;
    resources.subscribe = true;
  }
  return merged;
}

/** Whether any part of an endpoint offers the tasks extension. */
function __p__McpTasked(endpoint: __N__McpEndpoint): boolean {
  return endpoint.parts.some((part) =>
    part.capabilities.some(
      (text) => __p__McpObject(__p__McpObject(__p__McpRead(text))?.extensions)?.["io.modelcontextprotocol/tasks"] !== undefined,
    ),
  );
}

/** What a routed request asks for: the tool, the URI, the prompt, or the URI of the template or the name of the prompt a completion is for. */
function __p__McpNamed(params: Record<string, unknown> | undefined, method: string): string | undefined {
  const text = (value: unknown): string | undefined => (typeof value === "string" ? value : undefined);
  if (method === "tools/call" || method === "prompts/get") return text(params?.name);
  if (method === "resources/read") return text(params?.uri);
  if (method === "skills/get") return text(params?.name) ?? text(params?.uri);
  if (method === "tasks/get" || method === "tasks/cancel" || method === "tasks/update") return text(params?.taskId);
  if (text(__p__McpObject(params?.argument)?.name) === undefined) return undefined;
  const reference = __p__McpObject(params?.ref);
  return reference?.type === "ref/prompt" ? text(reference.name) : text(reference?.uri);
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
  named: boolean,
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
  return __p__McpReplied(200, __p__McpPrivately({ id, result }, named));
}

/** A task state as a task method and a notification write it: under the full task id, with a completed task's result as the `CallToolResult` the starting tool would have answered. */
function __p__McpTaskView(taskId: string, state: Readonly<Record<string, unknown>>): Record<string, unknown> {
  const view: Record<string, unknown> = { ...state, taskId };
  if ("result" in view) {
    view.result = {
      content: [{ text: __p__McpText(view.result), type: "text" }],
      isError: false,
      resultType: "complete",
      structuredContent: view.result,
    };
  }
  return view;
}

const __p__McpSubscription = "io.modelcontextprotocol/subscriptionId";

/** The `subscriptions/listen` of one call: the acknowledgment, then what `changes` yields that the client asked for, until it yields `undefined` or the call is cancelled. */
async function __p__McpListen(
  id: unknown,
  params: Record<string, unknown> | undefined,
  info: Record<string, unknown>,
  endpoint: __N__McpEndpoint,
  call: __N__McpCall,
  changes: __N__McpChanges,
): Promise<__N__McpHandled> {
  const invalid = (detail: string): __N__McpHandled => __p__McpRefused(id, 200, -32602, `Invalid params: ${detail}`);
  const filter = __p__McpObject(params?.notifications);
  if (filter === undefined) return invalid("subscriptions/listen needs `notifications`");
  const uris = filter.resourceSubscriptions;
  if (filter.resourcesListChanged !== undefined && typeof filter.resourcesListChanged !== "boolean") {
    return invalid("`resourcesListChanged` is not a boolean");
  }
  if (uris !== undefined && !(Array.isArray(uris) && uris.every((uri: unknown) => typeof uri === "string"))) {
    return invalid("`resourceSubscriptions` is not an array of strings");
  }
  const tasks = filter.taskIds;
  if (tasks !== undefined && !(Array.isArray(tasks) && tasks.every((task: unknown) => typeof task === "string"))) {
    return invalid("`taskIds` is not an array of strings");
  }
  const agreed: Record<string, unknown> = {};
  if (endpoint.parts.some((part) => part.capabilities.some((text) => __p__McpObject(__p__McpRead(text))?.resources !== undefined))) {
    if (filter.resourcesListChanged === true) agreed.resourcesListChanged = true;
    if (Array.isArray(uris) && uris.length > 0) agreed.resourceSubscriptions = uris;
  }
  if (__p__McpTasked(endpoint) && Array.isArray(tasks) && tasks.length > 0) agreed.taskIds = tasks;
  const notified = (method: string, given: Record<string, unknown>): string =>
    __p__McpText({ jsonrpc: "2.0", method, params: { ...given, _meta: { [__p__McpSubscription]: id } } });
  const lists = agreed.resourcesListChanged === true;
  const watched = Array.isArray(agreed.resourceSubscriptions) ? (agreed.resourceSubscriptions as ReadonlyArray<string>) : [];
  const followed = Array.isArray(agreed.taskIds) ? (agreed.taskIds as ReadonlyArray<string>) : [];
  const ended = __p__McpReplied(200, { id, result: { _meta: { ...info, [__p__McpSubscription]: id }, resultType: "complete" } });
  if (!(await call.write(notified("notifications/subscriptions/acknowledged", { notifications: agreed })))) return ended;
  const stopped = call.stopped.then(() => "stopped" as const);
  for (;;) {
    const change = await Promise.race([changes(), stopped]);
    if (call.cancelled || change === "stopped") return ended;
    if (change === undefined) return ended;
    let written = true;
    if (change.kind === "resourcesListChanged" && lists) {
      written = await call.write(notified("notifications/resources/list_changed", {}));
    } else if (change.kind === "resourceUpdated" && watched.includes(change.uri)) {
      written = await call.write(notified("notifications/resources/updated", { uri: change.uri }));
    } else if (change.kind === "task" && followed.includes(change.taskId)) {
      written = await call.write(notified("notifications/tasks", __p__McpTaskView(change.taskId, change.state)));
    } else if (change.kind === "keepAlive") {
      written = await call.keepAlive();
    }
    if (!written) return ended;
  }
}

async function __p__McpHandle(
  message: Record<string, unknown>,
  serverName: string,
  named: boolean,
  instructions: string | undefined,
  endpoint: __N__McpEndpoint,
  route: (part: number, method: string, params: unknown) => Promise<__N__McpAnswer | undefined>,
  listening?: (id: unknown, params: Record<string, unknown> | undefined, info: Record<string, unknown>) => Promise<__N__McpHandled>,
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
  const replied = (status: number, body: Record<string, unknown>): __N__McpHandled =>
    __p__McpReplied(status, __p__McpPrivately(body, named));
  if (method === "subscriptions/listen" && listening !== undefined) return listening(id, params, info);
  if (method.startsWith("tasks/") && !__p__McpTasked(endpoint)) {
    return __p__McpRefused(id, 404, -32601, `Method not found: ${method}`);
  }
  switch (method) {
    case "server/discover":
      return replied(200, {
        id,
        result: {
          _meta: info,
          cacheScope: "public",
          capabilities: __p__McpCapabilities(endpoint, listening !== undefined),
          instructions,
          resultType: "complete",
          supportedVersions: [__p__McpVersion],
          ttlMs: 300000,
        },
      });
    case "tools/list":
      return replied(200, {
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
      return replied(200, {
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
    case "prompts/list":
      return replied(200, {
        id,
        result: {
          _meta: info,
          cacheScope: "public",
          prompts: endpoint.parts.flatMap((part) => part.prompts.map((prompt) => new __N__McpRaw(prompt))),
          resultType: "complete",
          ttlMs: 300000,
        },
      });
    case "resources/list":
      return __p__McpListResources(id, params, info, named, endpoint, route);
    case "skills/list": {
      const skills = endpoint.parts.flatMap((part) => part.skills);
      const dynamic = skills.some((text) => __p__McpObject(__p__McpRead(text))?.resources === "dynamic");
      return replied(200, {
        id,
        result: {
          _meta: info,
          cacheScope: dynamic ? "private" : "public",
          resultType: "complete",
          skills: skills.map((text) => new __N__McpRaw(text)),
          ttlMs: dynamic ? 0 : 300000,
        },
      });
    }
    case "tasks/get":
    case "tasks/cancel":
    case "tasks/update":
    case "tools/call":
    case "resources/read":
    case "prompts/get":
    case "skills/get":
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
        return replied(200, {
          id,
          result: result === undefined ? answer.result : { ...result, _meta: info },
        });
      }
      const unknown =
        method === "tools/call"
          ? "Unknown tool"
          : method === "resources/read"
            ? "Resource not found"
            : method === "prompts/get"
              ? "Unknown prompt"
              : method === "skills/get"
                ? "Skill not found"
                : method === "completion/complete"
                  ? "Unknown completion"
                  : "Unknown task";
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
 * it names in `host.inFlight`, and the answer of a cancelled request is dropped. A host that hands in
 * `host.changes` also answers `subscriptions/listen`.
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
    const changes = host.changes;
    const listening =
      changes === undefined
        ? undefined
        : (id: unknown, params: Record<string, unknown> | undefined, info: Record<string, unknown>) => {
            entered.leave = host.inFlight.enter(message.id, call);
            return __p__McpListen(id, params, info, endpoint, call, changes);
          };
    const handled = await __p__McpHandle(
      message,
      serverName,
      false,
      instructions,
      endpoint,
      async (part, method, params) => {
        if (entered.ctx === undefined) {
          entered.leave = host.inFlight.enter(message.id, call);
          entered.ctx = { built: ctx(call) };
        }
        return route(entered.ctx.built, part, method, params);
      },
      listening,
    );
    return call.cancelled ? undefined : handled.text;
  } finally {
    entered.leave?.();
  }
}"#;

const INTERVIEW: &str = r#"/** Whether the client declared the elicitation `mode`: an empty `elicitation` is a form. */
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
  const written = __p__McpObject(error);
  const held = interview.tag === "" ? __p__McpObject(written?.[interview.wire]) : written?.[interview.tag] === interview.wire ? written : undefined;
  const arm = interview.content === "" ? held : __p__McpObject(held?.[interview.content]);
  if (arm === undefined) return undefined;
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
  if (__p__McpDismissed(call, interview.wire) || !__p__McpElicits(call, "url")) return undefined;
  return __p__McpInputRequired(interview.wire, { message, mode: "url", url: arm.url });
}"#;

const BLOCK: &str = "/** One block of `content`, which the dispatcher has already written as the specification writes it. */
function __p__McpBlock(block: unknown): Record<string, unknown> {
  return __p__McpObject(block) ?? {};
}";

const CONTENT: &str = r#"/**
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

/** The Zod issue codes that mean the arguments do not read as the message; any other is a validator's refusal. */
const __p__McpUnreadable: ReadonlySet<string> = new Set([
  "invalid_type",
  "invalid_union",
  "invalid_value",
  "invalid_key",
  "invalid_element",
  "unrecognized_keys",
]);

type __p__McpIssue = {
  readonly code: string;
  readonly message: string;
  readonly path: ReadonlyArray<PropertyKey>;
  readonly keys?: ReadonlyArray<string>;
  readonly errors?: ReadonlyArray<ReadonlyArray<__p__McpIssue>>;
  readonly params?: { readonly undeserializable?: unknown; readonly numberKey?: unknown };
};

/** Whether `issue` keeps the arguments from reading. serde reads them out of a value, where a `Result` of other than one key is a value of the wrong kind. */
function __p__McpUnreadableIssue(issue: __p__McpIssue): boolean {
  return (
    __p__McpUnreadable.has(issue.code) ||
    issue.params?.undeserializable === true ||
    issue.params?.numberKey === true
  );
}

/** The words serde refuses a key of a map keyed by a number in where it reads it as text that is not the document, out of a value, which reads the text after the number as no number; none for any other issue. */
function __p__McpNoNumber(issue: __p__McpIssue): string | undefined {
  if (issue.params?.numberKey !== true || issue.params.undeserializable !== true) return undefined;
  return issue.message === "__EXPECTED_QUOTE__" ? "__EXPECTED_NUMERIC_KEY__" : issue.message;
}

/** The path of an issue: below an `invalid_union`, the deepest path any of its alternatives reached; below `unrecognized_keys`, the first key. */
function __p__McpSteps(issue: __p__McpIssue): ReadonlyArray<PropertyKey> {
  if (issue.code === "unrecognized_keys" && issue.keys !== undefined) return [...issue.path, issue.keys[0]];
  if (issue.code !== "invalid_union" || issue.errors === undefined) return issue.path;
  let deepest: ReadonlyArray<PropertyKey> = issue.path;
  for (const alternative of issue.errors) {
    for (const inner of alternative) {
      const steps = [...issue.path, ...__p__McpSteps(inner)];
      if (steps.length > deepest.length) deepest = steps;
    }
  }
  return deepest;
}

/** Each issue at its dotted path, one per key of an `unrecognized_keys` issue. */
function __p__McpListed(
  issues: ReadonlyArray<__p__McpIssue & { readonly message: string }>,
): Array<{ detail: string; field: string | undefined }> {
  return issues.flatMap((issue) => {
    if (issue.code === "unrecognized_keys" && issue.keys !== undefined) {
      return issue.keys.map((key) => ({ detail: `Unrecognized key: "${key}"`, field: [...issue.path, key].join(".") }));
    }
    const steps = __p__McpSteps(issue);
    return [{ detail: issue.message, field: steps.length === 0 ? undefined : steps.join(".") }];
  });
}

/** The fault of a message that does not read as `schema`: `failed-validation` naming `answers` where a form answer does not read, `undeserializable-payload` naming no field where the first value that does not read is a key that is no number, and otherwise `failed-validation` naming the path of the first value that does not read and listing every value that does not read where there are several, an unknown key at its own path; none where it reads or only the validator refuses it, whose bounds wait for a read that succeeds. */
function __p__McpUnread(
  schema: ZodType,
  payload: unknown,
  operation: string,
  answers?: string,
): __N__Fault | undefined {
  const read = schema.safeParse(payload);
  if (read.success) return undefined;
  const issues = read.error.issues;
  if (answers !== undefined && issues.some((issue) => issue.path[0] === answers)) {
    const detail = issues
      .filter((issue) => issue.path[0] === answers)
      .map((issue) => (issue.path.length === 1 ? issue.message : `'${issue.path.join(".")}': ${issue.message}`))
      .join("; ");
    return __FAULT__("failed-validation", operation, detail, answers);
  }
  const unreadable = issues.filter(__p__McpUnreadableIssue);
  const unread = unreadable[0];
  if (unread === undefined) return undefined;
  const noNumber = __p__McpNoNumber(unread);
  if (noNumber !== undefined) return __FAULT__("undeserializable-payload", operation, noNumber);
  const steps = __p__McpSteps(unread);
  const detail = unreadable
    .flatMap((issue) =>
      issue.code === "unrecognized_keys" && issue.keys !== undefined
        ? issue.keys.map((key) => `'${[...issue.path, key].join(".")}': Unrecognized key: "${key}"`)
        : [issue.path.length === 0 ? issue.message : `'${issue.path.join(".")}': ${issue.message}`],
    )
    .join("; ");
  const fault = __FAULT__("failed-validation", operation, detail, steps.length === 0 ? undefined : steps.join("."));
  const listed = __p__McpListed(unreadable);
  return listed.length < 2 ? fault : { ...fault, issues: listed };
}

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
 * The route of `__N__`: answers a `tools/call` of one of its tools, a `resources/read`, a
 * `prompts/get`, its page of `resources/list` and a `completion/complete` through `create__N__Dispatcher`, so the Zod reads
 * judge every message, and answers `undefined` for any other request.
 */
export function create__N__McpRoute<Ctx>(
  impl: __DRIVEN__,
  onFault: (fault: __N__Fault) => void = (fault) => console.error(fault),
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
    const refused =
      held === undefined && given !== null
        ? __FAULT__("undeserializable-payload", operation.tool, "the arguments are not a JSON object")
        : operation.wrapped && held !== undefined && held.value === undefined
          ? __FAULT__("failed-validation", operation.tool, "'value': missing field `value`", "value")
          : undefined;
    const unread = refused ?? __p__McpUnread(operation.schema, payload, operation.tool, operation.interview?.answers);
    const attempt =
      unread === undefined
        ? await attempted(ctx, operation.operation, payload, headers)
        : { dispatched: __p__Framed(unread) };
    if ("thrown" in attempt) {
      const detail = attempt.thrown instanceof Error ? attempt.thrown.message : String(attempt.thrown);
      if (operation.oneWay) {
        onFault(__FAULT__("handler-panic", operation.tool, detail));
        return { result: __p__McpEmpty };
      }
      return {
        error: {
          code: -32603,
          data: { fault: __FAULT__("handler-panic", operation.tool, detail), isServiceFault: true },
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
__TASK_START____BLOCKS__      let structured: unknown;
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
__ASKING__    const faulted = error as { fault: object };
    const named = fault ? { ...faulted, fault: { ...faulted.fault, operation: operation.tool } } : error;
    const body =
      !fault && operation.errorHeadersOut.length > 0
        ? { value: error, ...beside(operation.errorHeadersOut) }
        : named;
    return {
      result: { content: [{ text: __p__McpText(body), type: "text" }], isError: true, resultType: "complete" },
    };
  };
  const answering = async (
    ctx: Ctx,
    operation: string,
    schema: ZodType,
    payload: unknown,
    refusal: string,
    success: (value: unknown) => Record<string, unknown>,
  ): Promise<__N__McpAnswer> => {
    const unread = __p__McpUnread(schema, payload, operation);
    if (unread !== undefined) return __p__McpDeclined({ fault: unread, isServiceFault: true }, refusal);
    const attempt = await attempted(ctx, operation, payload, []);
    if ("thrown" in attempt) return __p__McpPanicked(operation, attempt.thrown);
    const answered = attempt.dispatched?.answered as { ok: boolean; value?: unknown; error?: unknown } | undefined;
    if (answered === undefined || !answered.ok) return __p__McpDeclined(answered?.error, refusal);
    return { result: success(answered.value) };
  };
  const reading = async (ctx: Ctx, call: Record<string, unknown>): Promise<__N__McpAnswer | undefined> => {
    const uri = call.uri;
    if (typeof uri !== "string") return undefined;
    const folder = __p__McpSkillFiles.get(uri);
    if (folder !== undefined) {
      return {
        result: { cacheScope: "public", contents: [new __N__McpRaw(folder)], resultType: "complete", ttlMs: 300000 },
      };
    }
    for (const operation of __p__McpOperations) {
      if (operation.kind !== "skill") continue;
      const root = `skill://${operation.name}/`;
      if (!uri.startsWith(root) || uri.length === root.length) continue;
      const path = uri.slice(root.length);
      return answering(ctx, operation.operation, operation.schema, path, "Skill file not found", (value) => {
        const given: Record<string, unknown> = { uri };
        if (operation.blob) {
          const pair = value as ReadonlyArray<unknown>;
          given.mimeType = pair[1];
          given.blob = __p__McpBase64(pair[0]);
        } else {
          given.mimeType = "text/markdown";
          given.text = path === "SKILL.md" ? `${operation.frontmatter}${value as string}` : value;
        }
        return { cacheScope: "private", contents: [given], resultType: "complete", ttlMs: 0 };
      });
    }
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
      return answering(ctx, operation.operation, operation.schema, payload, "Resource not found", (value) => {
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
  const getting = (call: Record<string, unknown>): __N__McpAnswer | undefined => {
    const name = typeof call.name === "string" ? call.name : undefined;
    const uri = typeof call.uri === "string" ? call.uri : undefined;
    for (const text of __p__McpPart.skills) {
      const entry = __p__McpObject(__p__McpRead(text));
      if (entry === undefined) continue;
      const held = __p__McpObject(entry.frontmatter);
      if (!((name !== undefined && held?.name === name) || (uri !== undefined && entry.uri === uri))) continue;
      const dynamic = entry.resources === "dynamic";
      return {
        result: {
          cacheScope: dynamic ? "private" : "public",
          resultType: "complete",
          skill: new __N__McpRaw(text),
          ttlMs: dynamic ? 0 : 300000,
        },
      };
    }
    return undefined;
  };
  const listing = async (ctx: Ctx, call: Record<string, unknown>): Promise<__N__McpAnswer | undefined> => {
    const operation = __p__McpOperations.find((candidate) => candidate.kind === "listing");
    if (operation === undefined || operation.kind !== "listing") return undefined;
    const payload = operation.cursor ? (typeof call.cursor === "string" ? call.cursor : null) : {};
    return answering(ctx, operation.operation, operation.schema, payload, "Resources could not be listed", (value) => ({
      ...(__p__McpObject(value) ?? {}),
      cacheScope: operation.cacheScope,
      resultType: "complete",
      ttlMs: operation.ttlMs,
    }));
  };
__PROMPTING____TASKING__  const completing = async (ctx: Ctx, call: Record<string, unknown>): Promise<__N__McpAnswer | undefined> => {
    const reference = __p__McpObject(call.ref);
    const argument = __p__McpObject(call.argument);
    if (reference === undefined) return undefined;
    const kind = reference.type === "ref/resource" ? "resource" : reference.type === "ref/prompt" ? "prompt" : undefined;
    const uri = kind === "prompt" ? reference.name : reference.uri;
    const parameter = argument?.name;
    if (kind === undefined || typeof uri !== "string" || typeof parameter !== "string") return undefined;
    const typed = typeof argument?.value === "string" ? argument.value : "";
    const context = __p__McpObject(call.context);
    const chosen = context !== undefined && "arguments" in context ? context.arguments : {};
    for (const operation of __p__McpOperations) {
      if (operation.kind !== "completion" || operation.reference !== kind || operation.uri !== uri || operation.parameter !== parameter) continue;
      return answering(
        ctx,
        operation.operation,
        operation.schema,
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
__PROMPT_CASE____TASK_CASE__      case "skills/get":
        return getting(call);
      case "completion/complete":
        return completing(ctx, call);
      default:
        return undefined;
    }
  };
}"#;

const PROMPTING: &str = r#"  const prompting = async (ctx: Ctx, call: Record<string, unknown>): Promise<__N__McpAnswer | undefined> => {
    const operation = __p__McpOperations.find((candidate) => candidate.kind === "prompt" && candidate.prompt === call.name);
    if (operation === undefined || operation.kind !== "prompt") return undefined;
    const given = __p__McpObject(call.arguments) ?? {};
    const payload = operation.source === "empty" ? {} : operation.source === "single" ? (given[operation.key] ?? null) : given;
    return answering(ctx, operation.operation, operation.schema, payload, `${operation.prompt}: the service refused the prompt`, (value) => ({
      description: operation.doc,
      messages: operation.text
        ? [{ content: { text: value, type: "text" }, role: "user" }]
        : (Array.isArray(value) ? (value as ReadonlyArray<Record<string, unknown>>) : []).map((message) => ({
            content: __p__McpBlock(message.content),
            role: message.role,
          })),
      resultType: "complete",
    }));
  };
"#;

const TASK_START: &str = r#"      if (operation.task !== undefined) {
        const handle: Record<string, unknown> = { ...(__p__McpObject(answered.value) ?? {}), status: "working" };
        const declares = __p__McpObject(__p__McpObject(__p__McpObject(call._meta)?.[__p__McpMetaCapabilities])?.extensions)?.[
          "io.modelcontextprotocol/tasks"
        ];
        if (__p__McpObject(declares) !== undefined) {
          return { result: { ...handle, resultType: "task", taskId: `${operation.tool}:${String(handle.taskId)}` } };
        }
        return {
          result: {
            content: [{ text: __p__McpText(handle), type: "text" }],
            isError: false,
            resultType: "complete",
            structuredContent: handle,
          },
        };
      }
"#;

const TASKING: &str = r#"  const tasking = async (ctx: Ctx, method: string, call: Record<string, unknown>): Promise<__N__McpAnswer | undefined> => {
    const taskId = typeof call.taskId === "string" ? call.taskId : undefined;
    const colon = taskId === undefined ? -1 : taskId.indexOf(":");
    if (taskId === undefined || colon < 0) return undefined;
    const tool = taskId.slice(0, colon);
    const held = taskId.slice(colon + 1);
    const start = __p__McpOperations.find((candidate) => candidate.kind === "tool" && candidate.tool === tool);
    if (start === undefined || start.kind !== "tool" || start.task === undefined) return undefined;
    const named = (name: string | undefined) => {
      const found = __p__McpOperations.find((candidate) => candidate.kind === "tool" && candidate.tool === name);
      return found !== undefined && found.kind === "tool" ? found : undefined;
    };
    if (method === "tasks/get") {
      const target = named(start.task.get);
      if (target === undefined) return undefined;
      return answering(ctx, target.operation, target.schema, held, `${tool}: the service refused the task`, (value) => {
        return { ...__p__McpTaskView(taskId, __p__McpObject(value) ?? {}), resultType: "complete" };
      });
    }
    if (method === "tasks/cancel") {
      const target = named(start.task.cancel);
      if (target === undefined) return undefined;
      return answering(ctx, target.operation, target.schema, held, `${tool}: the service refused to cancel the task`, () => ({
        resultType: "complete",
      }));
    }
    const target = named(start.task.update);
    if (target === undefined) return undefined;
    const responses = __p__McpObject(call.inputResponses);
    if (responses === undefined) {
      return { error: { code: -32602, message: "Invalid params: tasks/update needs `inputResponses`" } };
    }
    const [idKey, keyKey, actionKey, contentKey] = start.task.keys;
    for (const key of Object.keys(responses).sort()) {
      const response = __p__McpObject(responses[key]);
      const answer = await answering(
        ctx,
        target.operation,
        target.schema,
        {
          [idKey ?? ""]: held,
          [keyKey ?? ""]: key,
          [actionKey ?? ""]: typeof response?.action === "string" ? response.action : "",
          [contentKey ?? ""]: response?.content === undefined ? null : __p__McpText(response.content),
        },
        `${tool}: the service refused the answer`,
        () => ({ resultType: "complete" }),
      );
      if ("error" in answer) return answer;
    }
    return { result: { resultType: "complete" } };
  };
"#;

const TASK_CASE: &str = r#"      case "tasks/get":
      case "tasks/cancel":
      case "tasks/update":
        return tasking(ctx, method, call);
"#;

const PROMPT_CASE: &str = r#"      case "prompts/get":
        return prompting(ctx, call);
"#;

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

/**
 * Where an HTTP server writes an event-stream answer. `begin` is called once, before the first
 * event, with the status and headers of the answer; `write` then takes the text of each event, or
 * of the comment line that keeps the stream open. A call that rejects means the client has gone.
 * The host ends its response when the server returns `undefined`.
 */
export type __N__McpHttpStream = {
  begin(status: number, headers: ReadonlyArray<readonly [string, string]>): Promise<void>;
  write(event: string): Promise<void>;
};

/** What `admit` answers: the context the handlers run with and, where the caller decides it, the name the server answers as with every public answer of the request made private, or why the request is refused. */
export type __N__McpAdmitted<Ctx> = { readonly ctx: Ctx; readonly serverName?: string | undefined } | { readonly denied: __N__McpDenied };";

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

const __p__McpEventHeaders: ReadonlyArray<readonly [string, string]> = [
  ["Content-Type", "text/event-stream"],
  ["X-Accel-Buffering", "no"],
];

/** One event of an event stream, carrying one JSON-RPC message. */
function __p__McpFrame(message: string): string {
  return `event: message\ndata: ${message}\n\n`;
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
 * `progressToken` answers an event stream written to `stream` as it happens: `begin`, each
 * notification, then the answer, and this answers `undefined`. So does a `subscriptions/listen`,
 * where `changes` is handed in: it ends when `changes` answers `undefined`. Any other request
 * answers the response, and `stream` is never begun.
 */
export async function __p__McpServeHttp<Ctx>(
  request: __N__McpHttpRequest,
  settings: __N__McpHttpSettings,
  stream: __N__McpHttpStream,
  serverName: string,
  instructions: string | undefined,
  endpoint: __N__McpEndpoint,
  admit: (request: __N__McpHttpRequest, call: __N__McpCall) => Promise<__N__McpAdmitted<Ctx>>,
  route: (ctx: Ctx, part: number, method: string, params: unknown) => Promise<__N__McpAnswer | undefined>,
  changes?: __N__McpChanges,
): Promise<__N__McpHttpResponse | undefined> {
  if (settings.oauth !== undefined) {
    const path = request.path.split(/[?#]/)[0];
    if (request.method === "GET" && path === __p__McpUrlPath(settings.oauth.resourceMetadataUrl)) {
      return __p__McpMetadata(settings.oauth);
    }
  }
  if (request.method !== "POST") return { ...__p__McpHttpReplied(405), headers: [["Allow", "POST"]] };
  const origin = __p__McpHeader(request, "origin");
  if (origin !== undefined && !settings.allowedOrigins.includes(origin)) return __p__McpHttpReplied(403);
  const held: Array<string> = [];
  let live = false;
  const give = (text: string): Promise<void> => {
    if (live) return stream.write(text);
    held.push(text);
    return Promise.resolve();
  };
  const call = new __N__McpCall(
    __p__McpBodyToken(request),
    (event) => give(__p__McpFrame(event)),
    () => give(":\n\n"),
  );
  const admitted = await admit(request, call);
  if ("denied" in admitted) return __p__McpDenial(admitted.denied, settings, request);
  const { ctx } = admitted;
  const callerNamed = admitted.serverName !== undefined;
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
  const streams = __p__McpToken(message) !== undefined || method === "subscriptions/listen";
  let beginning: Promise<void> | undefined;
  const begin = (): Promise<void> => {
    if (!streams) return Promise.resolve();
    beginning ??= (async () => {
      try {
        await stream.begin(200, __p__McpEventHeaders);
        live = true;
        for (const text of held.splice(0)) await stream.write(text);
      } catch {
        call.cancel();
      }
    })();
    return beginning;
  };
  const asked = { routed: false };
  const listening =
    changes === undefined
      ? undefined
      : async (id: unknown, given: Record<string, unknown> | undefined, info: Record<string, unknown>) => {
          asked.routed = true;
          await begin();
          return __p__McpListen(id, given, info, endpoint, call, changes);
        };
  const handled = await __p__McpHandle(
    message,
    admitted.serverName ?? serverName,
    callerNamed,
    instructions,
    endpoint,
    async (part, called, given) => {
      asked.routed = true;
      await begin();
      return call.cancelled && streams ? undefined : route(ctx, part, called, given);
    },
    listening,
  );
  if (!asked.routed) return __p__McpHttpReplied(handled);
  const answered = call.cancelled ? undefined : handled.text;
  if (!streams) return __p__McpHttpReplied(answered === undefined ? 202 : handled);
  if (answered !== undefined) {
    try {
      await stream.write(__p__McpFrame(answered));
    } catch {
      call.cancel();
    }
  }
  return undefined;
}"#;

const FACTORIES: &str = "/**
 * Turns a `__N__` implementation into the function that answers one JSON-RPC message of the
 * service alone: usable as it is over stdio, one line in and one line out. `ctx` builds the
 * handler's context from the request in flight, `writer` takes what the request writes beyond its
 * answer, and `inFlight` lists the requests that `notifications/cancelled` can reach. `changes`
 * is the application's source of changes: where it is handed in, `subscriptions/listen` is answered.
 */
export function create__N__McpDispatcher<Ctx>(
  impl: __DRIVEN__,
  onFault?: (fault: __N__Fault) => void,
): (
  text: string,
  ctx: (call: __N__McpCall) => Ctx,
  writer: __N__McpWriter,
  inFlight: __N__McpInFlight,
  changes?: __N__McpChanges,
) => Promise<string | undefined> {
  const route = create__N__McpRoute(impl, onFault);
  const endpoint = __p__McpEndpoint(__p__McpPart);
  return (text, ctx, writer, inFlight, changes) =>
    __p__McpServe(
      text,
      __p__McpServerName,
      __p__McpInstructions,
      endpoint,
      ctx,
      changes === undefined ? { writer, inFlight } : { writer, inFlight, changes },
      (built, part, method, params) =>
        part === 0 ? route(built, method, params) : Promise.resolve(undefined),
    );
}

/** `create__N__McpDispatcher` over Streamable HTTP. The function it answers writes an event-stream answer to the `stream` it is handed and answers `undefined`. */
export function create__N__McpHttpDispatcher<Ctx>(
  impl: __DRIVEN__,
  settings: __N__McpHttpSettings,
  admit: (request: __N__McpHttpRequest, call: __N__McpCall) => Promise<__N__McpAdmitted<Ctx>>,
  changes?: __N__McpChanges,
  onFault?: (fault: __N__Fault) => void,
): (request: __N__McpHttpRequest, stream: __N__McpHttpStream) => Promise<__N__McpHttpResponse | undefined> {
  const route = create__N__McpRoute(impl, onFault);
  const endpoint = __p__McpEndpoint(__p__McpPart);
  return (request, stream) =>
    __p__McpServeHttp(
      request,
      settings,
      stream,
      __p__McpServerName,
      __p__McpInstructions,
      endpoint,
      admit,
      (ctx, part, method, params) => (part === 0 ? route(ctx, method, params) : Promise.resolve(undefined)),
      changes,
    );
}";

/// The type an `mcp(content)` operation answers a list of, as the service module declares it in
/// Rust: one block of a tool's `content`, in the form the specification writes it.
pub const CONTENT_TYPE: &str = r#"/** One block of a tool's `content`: what an `mcp(content)` operation answers a list of. */
export type McpContent =
  | { readonly type: "text"; readonly text: string }
  | { readonly type: "image"; readonly data: Uint8Array; readonly mimeType: string }
  | { readonly type: "audio"; readonly data: Uint8Array; readonly mimeType: string }
  | {
      readonly type: "resource_link";
      readonly uri: string;
      readonly name: string;
      readonly title?: string | undefined;
      readonly description?: string | undefined;
      readonly mimeType?: string | undefined;
    }
  | {
      readonly type: "resource";
      readonly uri: string;
      readonly mimeType?: string | undefined;
      readonly text: string;
    }
  | {
      readonly type: "resource";
      readonly uri: string;
      readonly mimeType?: string | undefined;
      readonly blob: Uint8Array;
    };

/** Who a prompt's message is from. */
export type McpRole = "user" | "assistant";

/** One message of a prompt: who it is from, and the block it holds. */
export type McpPromptMessage = { readonly role: McpRole; readonly content: McpContent };

/** `value` in base64. */
function McpBytes$Base64(value: Uint8Array): string {
  let binary = "";
  for (let at = 0; at < value.length; at += 4096) binary += String.fromCharCode(...value.subarray(at, at + 4096));
  return btoa(binary);
}

/** `block` as the specification writes it, which is how every transport of the service carries it. */
export function McpContent$Wire(block: McpContent): Record<string, unknown> {
  switch (block.type) {
    case "text":
      return { text: block.text, type: "text" };
    case "image":
    case "audio":
      return { data: McpBytes$Base64(block.data), mimeType: block.mimeType, type: block.type };
    case "resource_link":
      return {
        ...(block.description === undefined ? {} : { description: block.description }),
        ...(block.mimeType === undefined ? {} : { mimeType: block.mimeType }),
        name: block.name,
        ...(block.title === undefined ? {} : { title: block.title }),
        type: "resource_link",
        uri: block.uri,
      };
    case "resource":
      return {
        resource: {
          ...("blob" in block ? { blob: McpBytes$Base64(block.blob) } : {}),
          ...(block.mimeType === undefined ? {} : { mimeType: block.mimeType }),
          ...("text" in block ? { text: block.text } : {}),
          uri: block.uri,
        },
        type: "resource",
      };
  }
}

/** `message` as the specification writes it. */
export function McpPromptMessage$Wire(message: McpPromptMessage): Record<string, unknown> {
  return { content: McpContent$Wire(message.content), role: message.role };
}

const McpBytes$Schema = z.string().transform((text, ctx): Uint8Array => {
  try {
    return Uint8Array.from(atob(text), (letter) => letter.charCodeAt(0));
  } catch {
    ctx.addIssue({ code: "custom", message: "not base64" });
    return z.NEVER;
  }
});

const McpOptionalText$Schema = z.string().optional();

/** One block, read as the specification writes it. */
export const McpContent$Schema: ZodType<McpContent> = z.union([
  z.object({ type: z.literal("text"), text: z.string() }),
  z.object({ type: z.literal("image"), data: McpBytes$Schema, mimeType: z.string() }),
  z.object({ type: z.literal("audio"), data: McpBytes$Schema, mimeType: z.string() }),
  z.object({
    type: z.literal("resource_link"),
    uri: z.string(),
    name: z.string(),
    title: McpOptionalText$Schema,
    description: McpOptionalText$Schema,
    mimeType: McpOptionalText$Schema,
  }),
  z
    .object({
      type: z.literal("resource"),
      resource: z.union([
        z.object({ uri: z.string(), mimeType: McpOptionalText$Schema, text: z.string() }),
        z.object({ uri: z.string(), mimeType: McpOptionalText$Schema, blob: McpBytes$Schema }),
      ]),
    })
    .transform((block): McpContent => ({ type: "resource", ...block.resource })),
]);

/** One message, read as the specification writes it. */
export const McpPromptMessage$Schema: ZodType<McpPromptMessage> = z.object({
  role: z.enum(["user", "assistant"]),
  content: McpContent$Schema,
});"#;

/// The types an `mcp(task(...))` operation answers, as the service module declares them in Rust:
/// the handle of a task, and where a task stands.
pub const TASK_TYPE: &str = r#"/** The handle of a task: the id the application keeps it under, and when it started. */
export type McpTask = {
  readonly taskId: string;
  readonly createdAt: string;
  readonly lastUpdatedAt: string;
  readonly ttlMs: number | null;
  readonly pollIntervalMs?: number | undefined;
};

/** Why a task failed, as the specification writes an error. */
export type McpTaskError = { readonly code: number; readonly message: string; readonly data?: unknown };

/** Where a task stands: what `tasks/get` and the operation named in `task(get = ...)` answer. */
export type McpTaskState<T> =
  | (McpTask & { readonly status: "working" })
  | (McpTask & { readonly status: "input_required"; readonly inputRequests: Readonly<Record<string, unknown>> })
  | (McpTask & { readonly status: "completed"; readonly result: T })
  | (McpTask & { readonly status: "failed"; readonly error: McpTaskError })
  | (McpTask & { readonly status: "cancelled" });

const McpTask$Members = {
  taskId: z.string(),
  createdAt: z.string(),
  lastUpdatedAt: z.string(),
  ttlMs: z.number().nullable(),
  pollIntervalMs: z.number().optional(),
};

/** The schema a client reads the handle of a task by. */
export const McpTask$Schema = z.object(McpTask$Members);

/** The schema a client reads where a task stands by, given the schema of its result. */
export function McpTaskState$SchemaFactory<Result extends ZodType>(result: Result) {
  return z.discriminatedUnion("status", [
    z.object({ ...McpTask$Members, status: z.literal("working") }),
    z.object({
      ...McpTask$Members,
      status: z.literal("input_required"),
      inputRequests: z.record(z.string(), z.unknown()),
    }),
    z.object({ ...McpTask$Members, status: z.literal("completed"), result }),
    z.object({
      ...McpTask$Members,
      status: z.literal("failed"),
      error: z.object({ code: z.number(), message: z.string(), data: z.unknown().optional() }),
    }),
    z.object({ ...McpTask$Members, status: z.literal("cancelled") }),
  ]);
}"#;

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
    let mut published = vec![constants(&named, &prefix), fill(TYPES)];
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

/// The name an operation takes on an endpoint, `kind:name`: a tool, a resource, a template or a
/// skill.
fn endpoint_name(operation: &OperationDef) -> Option<String> {
    if operation.is_mcp_tool() {
        return Some(format!("tool:{}", operation.mcp_tool_name()));
    }
    if operation.is_mcp_prompt() {
        return Some(format!("prompt:{}", operation.mcp_tool_name()));
    }
    if let Some((skill, _)) = &operation.mcp.skill {
        return Some(format!("skill:{skill}"));
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
    let folder_names = service
        .skills
        .iter()
        .flat_map(|folder| folder.skills.iter())
        .map(|skill| format!("skill:{}", skill.name));
    let literal = service
        .operations
        .iter()
        .filter_map(endpoint_name)
        .chain(folder_names)
        .map(|name| ts_quoted(&name))
        .collect::<Vec<_>>()
        .join(", ");
    let skilled = service.skills.is_some()
        || service
            .operations
            .iter()
            .any(|operation| operation.mcp.skill.is_some());
    let served = skilled
        || service
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
    if service.operations.iter().any(OperationDef::is_mcp_prompt) {
        capabilities.push(ts_quoted("{\"prompts\":{}}"));
    }
    if skilled {
        capabilities.push(ts_quoted(
            "{\"extensions\":{\"io.modelcontextprotocol/skills\":{}}}",
        ));
    }
    if service
        .operations
        .iter()
        .any(|operation| operation.mcp.task.is_some())
    {
        capabilities.push(ts_quoted(
            "{\"extensions\":{\"io.modelcontextprotocol/tasks\":{}}}",
        ));
    }
    format!(
        "/** Everything `{named}` puts on an endpoint: its names, and each list item as JSON text, \
         so a schema keeps the digits Rust wrote. */\n\
         export const {prefix}McpPart: {named}McpPart & {{ readonly names: readonly [{literal}] }} = {{\n  \
         names: [{literal}],\n  \
         tools: [{TOOLS_MARKER}],\n  \
         resources: [{RESOURCES_MARKER}],\n  \
         resourceTemplates: [{TEMPLATES_MARKER}],\n  \
         prompts: [{PROMPTS_MARKER}],\n  \
         skills: [{SKILLS_MARKER}],\n  \
         capabilities: [{capabilities}],\n\
         }};\n\n\
         /** The content `resources/read` answers for each file of the skills folder, as JSON text, by \
         its URI. */\n\
         const {prefix}McpSkillFiles: ReadonlyMap<string, string> = new Map<string, string>([{SKILL_FILES_MARKER}]);",
        capabilities = capabilities.join(", "),
    )
}

/// `{Service}McpResource` and `{Service}McpResourcePage`, which a listing answers on every transport and
/// `ts_mcp_content()` declares. Interfaces, named after their service, so two listing services of one bundle
/// declare them without a clash.
pub fn page_types(service: &ServiceDef) -> String {
    if !service
        .operations
        .iter()
        .any(|operation| operation.mcp.list_resources)
    {
        return String::new();
    }
    "/** One resource of a listing, as `resources/list` writes it. */\n\
     export interface @N@McpResource {\n  \
     uri: string;\n  \
     name: string;\n  \
     title?: string;\n  \
     description?: string;\n  \
     mimeType?: string;\n  \
     size?: number;\n\
     }\n\n\
     /** One page of `resources/list`, with the cursor of the next where there is one. */\n\
     export interface @N@McpResourcePage {\n  \
     resources: Array<@N@McpResource>;\n  \
     nextCursor?: string;\n\
     }\n\n\
     /** One resource of a listing, read as `resources/list` writes it. */\n\
     export const @N@McpResource$Schema: ZodType<@N@McpResource> = z.object({\n  \
     uri: z.string(),\n  \
     name: z.string(),\n  \
     title: z.string().optional(),\n  \
     description: z.string().optional(),\n  \
     mimeType: z.string().optional(),\n  \
     size: z.number().optional(),\n\
     });\n\n\
     /** One page of `resources/list`, read as it is written. */\n\
     export const @N@McpResourcePage$Schema: ZodType<@N@McpResourcePage> = z.object({\n  \
     resources: z.array(@N@McpResource$Schema),\n  \
     nextCursor: z.string().optional(),\n\
     });"
    .replace("@N@", &service.ident.to_string())
}

fn operations(service: &ServiceDef, named: &str, prefix: &str) -> String {
    let mut rows: Vec<String> = service
        .operations
        .iter()
        .filter(|operation| operation.is_mcp_tool())
        .map(|operation| operation_row(service, operation))
        .collect();
    rows.extend(resource_rows(service));
    rows.extend(
        service
            .operations
            .iter()
            .filter(|operation| operation.is_mcp_prompt())
            .map(prompt_row),
    );
    rows.extend(service.operations.iter().filter_map(skill_row));
    format!(
        "{interview_types}\n\n\
         /** What the route reads of each operation: the tool, resource, listing or completion it \
         answers, the operation it calls, and how its arguments, success and error are carried. */\n\
         export type {named}McpOperation =\n  \
         | {{\n      \
         readonly kind: \"tool\";\n      \
         readonly tool: string;\n      \
         readonly operation: string;\n      \
         readonly schema: ZodType;\n      \
         readonly wrapped: boolean;\n      \
         readonly oneWay: boolean;\n      \
         readonly unit: boolean;\n      \
         readonly headersIn: ReadonlyArray<string>;\n      \
         readonly headersOut: ReadonlyArray<string>;\n      \
         readonly errorHeadersOut: ReadonlyArray<string>;\n      \
         readonly content: \"pair\" | \"list\" | undefined;\n      \
         readonly interview: {named}McpInterview | undefined;\n      \
         readonly task: {named}McpTask | undefined;\n    \
         }}\n  \
         | {{\n      \
         readonly kind: \"resource\";\n      \
         readonly uri: string;\n      \
         readonly operation: string;\n      \
         readonly schema: ZodType;\n      \
         readonly segments: ReadonlyArray<readonly [string, boolean]>;\n      \
         readonly source: \"empty\" | \"none\" | \"single\" | \"members\";\n      \
         readonly members: ReadonlyArray<readonly [number, string]>;\n      \
         readonly blob: boolean;\n      \
         readonly mimeType: string | undefined;\n      \
         readonly ttlMs: number;\n      \
         readonly cacheScope: string;\n    \
         }}\n  \
         | {{\n      \
         readonly kind: \"skill\";\n      \
         readonly name: string;\n      \
         readonly operation: string;\n      \
         readonly schema: ZodType;\n      \
         readonly blob: boolean;\n      \
         readonly frontmatter: string;\n    \
         }}\n  \
         | {{\n      \
         readonly kind: \"listing\";\n      \
         readonly operation: string;\n      \
         readonly schema: ZodType;\n      \
         readonly cursor: boolean;\n      \
         readonly ttlMs: number;\n      \
         readonly cacheScope: string;\n    \
         }}\n  \
         | {{\n      \
         readonly kind: \"completion\";\n      \
         readonly operation: string;\n      \
         readonly schema: ZodType;\n      \
         readonly reference: \"resource\" | \"prompt\";\n      \
         readonly uri: string;\n      \
         readonly parameter: string;\n      \
         readonly typed: string;\n      \
         readonly chosen: string;\n    \
         }}\n  \
         | {{\n      \
         readonly kind: \"prompt\";\n      \
         readonly prompt: string;\n      \
         readonly operation: string;\n      \
         readonly schema: ZodType;\n      \
         readonly source: \"empty\" | \"members\" | \"single\";\n      \
         readonly key: string;\n      \
         readonly text: boolean;\n      \
         readonly doc: string;\n    \
         }};\n\n\
         export const {prefix}McpOperations: ReadonlyArray<{named}McpOperation> = [\n{rows}\n];",
        interview_types = interview_types(named),
        rows = rows.join("\n"),
    )
}

fn interview_types(named: &str) -> String {
    format!(
        "/** What a tool that asks the user is told: the arm of its declared error that asks, the \
         wire key of the argument that carries a form's answers and the JSON text of the form, \
         and the message to ask with where the arm holds none. */\n\
         export type {named}McpInterview = {{\n  \
         readonly arm: string;\n  \
         readonly tag: string;\n  \
         readonly wire: string;\n  \
         readonly content: string;\n  \
         readonly answers: string | undefined;\n  \
         readonly form: string | undefined;\n  \
         readonly doc: string;\n\
         }};\n\n\
         /** What a tool that starts a task is told: the tools that answer `tasks/get`, \
         `tasks/cancel` and `tasks/update`, and the wire keys of the update's message. */\n\
         export type {named}McpTask = {{\n  \
         readonly get: string;\n  \
         readonly cancel: string | undefined;\n  \
         readonly update: string | undefined;\n  \
         readonly keys: ReadonlyArray<string>;\n\
         }};"
    )
}

/// The rows of the resources, a fixed URI ahead of a template, then the listing and the completions.
fn resource_rows(service: &ServiceDef) -> Vec<String> {
    let resourced: Vec<&OperationDef> = service
        .operations
        .iter()
        .filter(|operation| !operation.is_mcp_tool() && !operation.is_mcp_prompt())
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
        "  {{ kind: \"resource\", uri: {uri}, operation: {wire}, schema: {schema}, segments: [{segments}], \
         source: \"{source}\", members: [{members}], blob: {blob}, mimeType: {mime}, \
         ttlMs: {ttl}, cacheScope: {scope} }},",
        uri = ts_quoted(&resource.uri),
        wire = ts_quoted(&operation.wire_name),
        schema = message::schema(operation),
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

fn skill_row(operation: &OperationDef) -> Option<String> {
    let (name, _) = operation.mcp.skill.as_ref()?;
    let blob = matches!(
        &operation.outcome,
        OperationOutcome::Reply { success, error: _error } if matches!(success.as_ref(), Type::Tuple(_))
    );
    Some(format!(
        "  {{ kind: \"skill\", name: {name}, operation: {wire}, schema: {schema}, blob: {blob}, \
         frontmatter: {frontmatter} }},",
        name = ts_quoted(name),
        wire = ts_quoted(&operation.wire_name),
        schema = message::schema(operation),
        frontmatter = ts_quoted(&written_frontmatter(
            name,
            operation.doc.as_deref().unwrap_or_default()
        )),
    ))
}

fn listing_row(operation: &OperationDef) -> String {
    format!(
        "  {{ kind: \"listing\", operation: {wire}, schema: {schema}, cursor: {cursor}, ttlMs: {ttl}, \
         cacheScope: {scope} }},",
        wire = ts_quoted(&operation.wire_name),
        schema = message::schema(operation),
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
        .find(|candidate| written(&candidate.ident) == completes.operation)?;
    let (reference, uri, parameter) = match completes.kind {
        McpCompletesKind::Resource => (
            "resource",
            target.mcp.resource.as_ref()?.uri.clone(),
            completes.parameter.clone(),
        ),
        McpCompletesKind::Prompt => {
            let argument = target
                .mcp
                .prompt
                .as_ref()?
                .arguments
                .iter()
                .find(|(argument, _)| written(argument) == completes.parameter)?;
            (
                "prompt",
                target.mcp_tool_name().to_owned(),
                wire_key(&argument.0),
            )
        }
    };
    let OperationInputs::Generated(fields) = &operation.inputs else {
        return None;
    };
    Some(format!(
        "  {{ kind: \"completion\", operation: {wire}, schema: {schema}, reference: \"{reference}\", uri: {uri}, \
         parameter: {parameter}, typed: {typed}, chosen: {chosen} }},",
        wire = ts_quoted(&operation.wire_name),
        schema = message::schema(operation),
        uri = ts_quoted(&uri),
        parameter = ts_quoted(&parameter),
        typed = ts_quoted(&wire_key(&fields.first()?.0)),
        chosen = ts_quoted(&wire_key(&fields.get(1)?.0)),
    ))
}

/// The row of a prompt: how its arguments are read, and whether its success is one text.
fn prompt_row(operation: &OperationDef) -> String {
    let (source, key) = match &operation.inputs {
        OperationInputs::Empty => ("empty", String::new()),
        OperationInputs::Generated(_) => ("members", String::new()),
        OperationInputs::Named(_) => (
            "single",
            operation
                .mcp
                .prompt
                .iter()
                .filter_map(|prompt| prompt.arguments.first())
                .map(|(argument, _)| wire_key(argument))
                .next()
                .unwrap_or_default(),
        ),
    };
    let text = matches!(
        &operation.outcome,
        OperationOutcome::Reply { success, error: _error }
            if matches!(success.as_ref(), Type::Path(path) if path.path.is_ident("String"))
    );
    format!(
        "  {{ kind: \"prompt\", prompt: {prompt}, operation: {wire}, schema: {schema}, source: \"{source}\", \
         key: {key}, text: {text}, doc: {doc} }},",
        prompt = ts_quoted(operation.mcp_tool_name()),
        wire = ts_quoted(&operation.wire_name),
        schema = message::schema(operation),
        key = ts_quoted(&key),
        doc = ts_quoted(operation.doc.as_deref().unwrap_or_default()),
    )
}

fn operation_row(service: &ServiceDef, operation: &OperationDef) -> String {
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
        "  {{ kind: \"tool\", tool: {tool}, operation: {wire}, schema: {schema}, wrapped: {wrapped}, oneWay: {one_way}, unit: {unit}, \
         headersIn: [{headers_in}], headersOut: [{headers_out}], errorHeadersOut: \
         [{error_headers_out}], content: {content}, interview: {interview}, task: {task} }},",
        content = content_kind(operation),
        task = task_literal(service, operation),
        interview = interview_literal(operation),
        tool = ts_quoted(operation.mcp_tool_name()),
        schema = message::schema(operation),
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

/// Stands in the emitted text for the key serde tags the arm an operation asks with, and for the
/// name it writes the arm under and the key it holds the arm's members under: `part` is `TAG`, `WIRE`
/// or `CONTENT`.
pub fn arm_marker(operation: &OperationDef, part: &str) -> String {
    format!(
        "/*__TIXSCHEMA_MCP_ARM_{part}:{}__*/",
        written(&operation.ident)
    )
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

/// The tools a task tool names, by tool name, and the wire keys of the update's message.
fn task_literal(service: &ServiceDef, operation: &OperationDef) -> String {
    let Some(task) = operation.mcp.task.as_ref() else {
        return "undefined".to_owned();
    };
    let tool_of = |name: &str| {
        service
            .operations
            .iter()
            .find(|candidate| written(&candidate.ident) == name)
    };
    let quoted_tool = |name: &Option<String>| {
        name.as_deref().and_then(tool_of).map_or_else(
            || "undefined".to_owned(),
            |found| ts_quoted(found.mcp_tool_name()),
        )
    };
    let keys = task
        .update
        .as_deref()
        .and_then(tool_of)
        .map(|target| match &target.inputs {
            OperationInputs::Generated(fields) => fields
                .iter()
                .map(|(field, _)| ts_quoted(&wire_key(field)))
                .collect::<Vec<_>>()
                .join(", "),
            OperationInputs::Empty | OperationInputs::Named(_) => String::new(),
        })
        .unwrap_or_default();
    format!(
        "{{ get: {}, cancel: {}, update: {}, keys: [{keys}] }}",
        quoted_tool(&task.get),
        quoted_tool(&task.cancel),
        quoted_tool(&task.update),
    )
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
        "{{ arm: {}, tag: {}, wire: {}, content: {}, answers: {answers}, form: {form}, doc: {} }}",
        ts_quoted(&interview.arm.to_string()),
        arm_marker(operation, "TAG"),
        arm_marker(operation, "WIRE"),
        arm_marker(operation, "CONTENT"),
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

/// `__p__McpBlock` where a content tool or the prompt route calls it, and `__p__McpBlocks` where a
/// content tool does.
fn content_text(service: &ServiceDef, prefix: &str) -> String {
    let content = service
        .operations
        .iter()
        .any(|operation| operation.mcp.content.is_some());
    let prompting = service.operations.iter().any(OperationDef::is_mcp_prompt);
    [
        (content || prompting).then_some(BLOCK),
        content.then_some(CONTENT),
    ]
    .into_iter()
    .flatten()
    .map(|template| filled(template, "", prefix))
    .collect::<Vec<_>>()
    .join("\n\n")
}

/// The scopes `prompts/get` needs, by the name of the prompt.
fn prompt_scopes(table: &ScopeTable) -> String {
    if table.prompts.is_empty() {
        return String::new();
    }
    let mut body = String::from("  if (method === \"prompts/get\") {\n    switch (name) {\n");
    for (prompt, needed) in &table.prompts {
        let scopes = needed
            .iter()
            .map(|scope| ts_quoted(scope))
            .collect::<Vec<_>>()
            .join(", ");
        let _ = writeln!(
            body,
            "      case {}:\n        return [{scopes}];",
            ts_quoted(prompt)
        );
    }
    body.push_str("      default:\n        return [];\n    }\n  }\n");
    body
}

/// The scopes the task methods need, by the tool that started the task.
fn task_scopes(table: &ScopeTable) -> String {
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
    for (method, rows) in &table.tasks {
        let _ = writeln!(
            body,
            "  if (method === {} && name !== undefined) {{\n    switch (name.split(\":\")[0]) {{",
            ts_quoted(method)
        );
        for (tool, needed) in rows {
            let _ = writeln!(
                body,
                "      case {}:\n        return {};",
                ts_quoted(tool),
                list(needed)
            );
        }
        body.push_str("      default:\n        return [];\n    }\n  }\n");
    }
    body
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
    body.push_str(&prompt_scopes(&table));
    body.push_str(&task_scopes(&table));
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
         function {prefix}McpFault(kind: {named}FaultKind, operation: string, detail: string, field?: string): \
         {named}Fault {{\n\
         {minted}\n\
         }}",
        minted = fault::minted(named, "    detail,\n    field,\n    kind,\n    operation,"),
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
    let prompting = service.operations.iter().any(OperationDef::is_mcp_prompt);
    let tasks = service
        .operations
        .iter()
        .any(|operation| operation.mcp.task.is_some());
    ROUTE
        .replace("__TASK_START__", if tasks { TASK_START } else { "" })
        .replace("__TASKING__", if tasks { TASKING } else { "" })
        .replace("__TASK_CASE__", if tasks { TASK_CASE } else { "" })
        .replace("__PROMPTING__", if prompting { PROMPTING } else { "" })
        .replace("__PROMPT_CASE__", if prompting { PROMPT_CASE } else { "" })
        .replace("__DECLINED__", declined)
        .replace("__ANSWERS__", if answers { ANSWERS } else { "" })
        .replace("__BLOCKS__", if blocks { BLOCKS } else { "" })
        .replace("__ASKING__", if asks { ASKING } else { "" })
        .replace("__DRIVEN__", &driven)
        .replace("__FAULT__", &fault)
        .replace("__EXPECTED_NUMERIC_KEY__", EXPECTED_NUMERIC_KEY)
        .replace("__EXPECTED_QUOTE__", &EXPECTED_QUOTE.replace('"', "\\\""))
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
