//! The emitted bundle put through a real TypeScript compiler.

#[cfg(feature = "zod")]
use super::keyed::KeyedProbeServiceSchema;
use super::the_bundle_one_registration_line_produces::{
    audit_seam, author_schemas, bundle, probe_seam,
};
use super::{
    ApplyBundleReceipt, AuditServiceSchema, BalanceRequest, BalanceResponse, CreditWriteError,
    ProbeError, ProbeServiceSchema,
};
#[cfg(feature = "zod")]
use super::{
    BytesProbeServiceSchema, HeaderProbeDocument, HeaderProbeError, HeaderProbeServiceSchema,
    LapsedProbeError, LapsedProbeServiceSchema, QuotedFilingProbeServiceSchema,
    QuotedProbeServiceSchema, ReservedProbeServiceSchema, SpooledProbeServiceSchema,
    StreamedProbeServiceSchema, UnitPingError, UnitPingRequest, UnitPingServiceSchema,
};
use std::env;
use std::env::temp_dir;
use std::fs;
use std::io::Write as _;
use std::io::stderr;
#[cfg(feature = "zod")]
use std::os::unix::fs::symlink;
#[cfg(feature = "zod")]
use std::path::Path;
use std::path::PathBuf;
use std::process::{Command, id};
use std::sync::Once;

/// Names the compiler to run, for a machine that has one somewhere other than `PATH`.
const COMPILER_VAR: &str = "TIXSCHEMA_TSC";

/// What every check compiles under. `--strict` is the bar a consuming codebase sets; `--pretty
/// false` keeps a diagnostic readable when it lands in an assertion message.
const UNDER: [&str; 10] = [
    "--noEmit",
    "--strict",
    "--pretty",
    "false",
    "--target",
    "es2020",
    "--lib",
    "es2020,dom",
    "--module",
    "preserve",
];

/// Said once per test binary when no compiler is reachable.
static STOOD_DOWN: Once = Once::new();

/// The operation the incomplete implementation below leaves out.
#[cfg(feature = "zod")]
const OMITTED: &str = "sweep";

/// The surface of `zod` the emitter names, declared globally because the bundle names `z` and
/// `ZodType` without importing them.
#[cfg(feature = "zod")]
const ZOD_SURFACE: &str = "type ZodIssue = { path: ReadonlyArray<PropertyKey>; message: string };

type ZodParsed<Parsed> =
  | { success: true; data: Parsed }
  | { success: false; error: { issues: ReadonlyArray<ZodIssue> } };

declare type ZodType<Parsed> = {
  safeParse(value: unknown): ZodParsed<Parsed>;
};

declare type ZodBuilder = ZodType<never> & {
  gt(bound: number): ZodBuilder;
  int(): ZodBuilder;
  lt(bound: number): ZodBuilder;
  max(bound: number): ZodBuilder;
  meta(data: { examples: ReadonlyArray<unknown> }): ZodBuilder;
  min(bound: number): ZodBuilder;
  prefault(value: unknown): ZodBuilder;
  transform(map: (value: never) => unknown): ZodBuilder;
};

declare const z: {
  boolean(): ZodBuilder;
  discriminatedUnion(key: string, arms: ReadonlyArray<ZodBuilder>): ZodBuilder;
  literal(value: string): ZodBuilder;
  null(): ZodBuilder;
  nullable(inner: ZodBuilder): ZodBuilder;
  number(): ZodBuilder;
  string(): ZodBuilder;
  strictObject(shape: Record<string, ZodBuilder>): ZodBuilder;
  undefined(): ZodBuilder;
  union(arms: ReadonlyArray<ZodBuilder>): ZodBuilder;
};
";

/// Names the directory whose `node_modules` holds the real `zod`, which a check of what a client
/// reads compiles against: the surface above types every parse as `never`, which any value fills.
#[cfg(feature = "zod")]
const MODULES_VAR: &str = "TIXSCHEMA_NODE_MODULES";

/// The project a check against the real `zod` compiles, its modules resolving `zod` through the
/// `node_modules` beside them.
#[cfg(feature = "zod")]
const ZOD_PROJECT: &str = r#"{
  "compilerOptions": {
    "strict": true,
    "noEmit": true,
    "module": "nodenext",
    "moduleResolution": "nodenext",
    "target": "ES2022",
    "lib": ["ES2022", "DOM"]
  },
  "files": ["bundle.ts", "caller.ts"]
}
"#;

/// Names the runtime the language service is run under, for a machine that has one somewhere other
/// than `PATH`.
#[cfg(feature = "zod")]
const NODE_VAR: &str = "TIXSCHEMA_NODE";

/// Asks the TypeScript language service, the service an editor asks, what it says of the modules
/// it is handed, the reader last.
#[cfg(feature = "zod")]
const DEPRECATED_READS: &str = include_str!("../generic_types_tests/deprecated_reads.mjs");

/// A caller of `KeyedProbeService` that reads each method and each arm of a result that writes a
/// map keyed by a 64-bit integer, and each that writes one keyed by a `u32` beside it.
#[cfg(feature = "zod")]
const KEYED_CALLER: &str = r#"import type * as lib from "./bundle.js";

declare const client: lib.KeyedProbeServiceClient;
declare const http: lib.KeyedProbeServiceHttpClient;
declare const implementation: lib.KeyedProbeServiceImpl<null>;
declare const stowed: lib.StowRequest;

export async function read(): Promise<unknown[]> {
  const read: unknown[] = [stowed.kept, stowed.note];
  const narrowed = await client.narrow({});
  if (narrowed.ok) read.push(narrowed.value);
  const tallied = await client.tally({});
  if (tallied.ok) read.push(tallied.value);
  read.push(await http.tally({}));
  const outcome = await implementation.tally(null, {});
  if (outcome.ok) read.push(outcome.value);
  return read;
}
"#;

/// What the TypeScript language service reports of [`KEYED_CALLER`]: each deprecated name it
/// reads, `6385`, and each deprecated method's signature, `6387`, in the order read.
#[cfg(feature = "zod")]
const KEYED_DEPRECATED: [&str; 6] = [
    "6385 kept",
    "6387 tally",
    "6385 value",
    "6387 tally",
    "6387 tally",
    "6385 value",
];

/// A caller reading each `bytes` reply of `BytesProbeService` off every TypeScript client as its
/// result declares it: the bytes as an `Array<number>`, their content type, then the header.
#[cfg(feature = "zod")]
const BYTES_CALLER: &str = r#"import {
  createBytesProbeServiceClient,
  createBytesProbeServiceHttpClient,
  createBytesProbeServiceWsTransport,
  type BytesProbeServiceHttpTransport,
  type BytesProbeServiceTransport,
  type BytesProbeServiceWsSocket,
} from "./bundle.js";

declare const http: BytesProbeServiceHttpTransport;
declare const bus: BytesProbeServiceTransport;
declare const socket: BytesProbeServiceWsSocket;

export async function read(): Promise<string> {
  const said: Array<string> = [];
  const clients = [
    createBytesProbeServiceHttpClient(http),
    createBytesProbeServiceClient(bus),
    createBytesProbeServiceClient(createBytesProbeServiceWsTransport(socket)),
  ];
  for (const client of clients) {
    const spooled = await client.spool("a");
    if (spooled.ok) {
      const [bytes, contentType, id]: [Array<number>, string, string] = spooled.value;
      said.push(`${bytes.length}:${contentType}:${id}`);
    } else if ("isServiceFault" in spooled.error) {
      said.push(spooled.error.fault.kind);
    } else {
      const [declared, reason] = spooled.error;
      said.push(`${declared.errorCode}:${reason ?? ""}`);
    }
    const dumped = await client.dump("a");
    if (dumped.ok) {
      const [bytes, contentType]: [Array<number>, string] = dumped.value;
      said.push(`${bytes.length}:${contentType}`);
    }
  }
  return said.join(",");
}
"#;

/// A caller of every TypeScript surface of `SpooledProbeService` and `StreamedProbeService`: the
/// `http_rest` client and dispatcher hold the streamed operation, and the transport-agnostic and
/// `ws_rpc` ones only the operations their envelope carries.
#[cfg(feature = "zod")]
const HTTP_ALONE_CALLER: &str = r#"import {
  attachSpooledProbeServiceWsDispatcher,
  attachStreamedProbeServiceWsDispatcher,
  createSpooledProbeServiceClient,
  createSpooledProbeServiceDispatcher,
  createSpooledProbeServiceHttpClient,
  createSpooledProbeServiceHttpDispatcher,
  createSpooledProbeServiceWsTransport,
  createStreamedProbeServiceClient,
  createStreamedProbeServiceHttpClient,
  createStreamedProbeServiceHttpDispatcher,
  createStreamedProbeServiceWsTransport,
  type SpooledProbeServiceHttpTransport,
  type SpooledProbeServiceSpoolOutcome,
  type SpooledProbeServiceStreamOutcome,
  type SpooledProbeServiceWsSocket,
  type StreamedProbeServiceHttpTransport,
  type StreamedProbeServiceStreamOutcome,
  type StreamedProbeServiceWsSocket,
} from "./bundle.js";

declare const http: SpooledProbeServiceHttpTransport;
declare const socket: SpooledProbeServiceWsSocket;
declare const streamedHttp: StreamedProbeServiceHttpTransport;
declare const streamedSocket: StreamedProbeServiceWsSocket;
declare const spooled: SpooledProbeServiceSpoolOutcome;
declare const streamed: SpooledProbeServiceStreamOutcome;
declare const alone: StreamedProbeServiceStreamOutcome;

const spool = async (_ctx: null, _req: string) => spooled;
export const dispatch = createSpooledProbeServiceDispatcher<null>({ spool });
export const detach = attachSpooledProbeServiceWsDispatcher<null>(socket, null, { spool }, () => {});
export const served = createSpooledProbeServiceHttpDispatcher<null>({ spool, stream: async () => streamed });
export const servedAlone = createStreamedProbeServiceHttpDispatcher<null>({ stream: async () => alone });
export const detachAlone = attachStreamedProbeServiceWsDispatcher<null>(streamedSocket, null, {}, () => {});

export async function read(): Promise<string> {
  const said: Array<string> = [];
  const enveloped = createSpooledProbeServiceClient(createSpooledProbeServiceWsTransport(socket));
  const spooledReply = await enveloped.spool("a");
  said.push(spooledReply.ok ? "spooled" : "refused");
  const streamedReply = await createSpooledProbeServiceHttpClient(http).stream("a");
  said.push(streamedReply.ok ? "streamed" : "refused");
  const aloneReply = await createStreamedProbeServiceHttpClient(streamedHttp).stream("a");
  said.push(aloneReply.ok ? "streamed" : "refused");
  const empty = createStreamedProbeServiceClient(createStreamedProbeServiceWsTransport(streamedSocket));
  said.push(String(Object.keys(empty).length));
  return said.join(",");
}
"#;

/// A caller asking the transport-agnostic client and dispatcher of `SpooledProbeService` for the
/// streamed operation, which only `http_rest` carries.
#[cfg(feature = "zod")]
const HTTP_ALONE_MISUSE: &str = r#"import {
  createSpooledProbeServiceClient,
  createSpooledProbeServiceDispatcher,
  type SpooledProbeServiceSpoolOutcome,
  type SpooledProbeServiceStreamOutcome,
  type SpooledProbeServiceTransport,
} from "./bundle.js";

declare const bus: SpooledProbeServiceTransport;
declare const spooled: SpooledProbeServiceSpoolOutcome;
declare const streamed: SpooledProbeServiceStreamOutcome;

export const called = createSpooledProbeServiceClient(bus).stream("a");
export const dispatch = createSpooledProbeServiceDispatcher<null>({
  spool: async () => spooled,
  stream: async () => streamed,
});
"#;

/// Everything above the implementation's members.
#[cfg(feature = "zod")]
const IMPLEMENTATION_HEAD: &str = r#"import {
  createProbeServiceDispatcher,
  type ProbeServiceExpireCreditOutcome,
  type ProbeServiceGetBalanceOutcome,
  type ProbeServiceProbeHeaderOutcome,
  type ProbeServiceSettleOutcome,
  type ProbeServiceSweepOutcome,
} from "./bundle";

type ProbeContext = { loggerName: string };

export const dispatch = createProbeServiceDispatcher<ProbeContext>({
"#;

#[cfg(feature = "zod")]
const IMPLEMENTATION_TAIL: &str = "});\n";

/// One member per operation the service declares.
#[cfg(feature = "zod")]
const IMPLEMENTATION_MEMBERS: [(&str, &str); 6] = [
    (
        "applyBundle",
        "  async applyBundle(ctx, req): Promise<void> {
    void `${ctx.loggerName}:${req.organizationId}:${req.bundleId}`;
  },
",
    ),
    (
        "expireCredit",
        r#"  async expireCredit(ctx, req): Promise<ProbeServiceExpireCreditOutcome> {
    void `${ctx.loggerName}:${req.organizationId}:${req.creditId}`;
    return { ok: false, error: { errorCode: "conflict" } };
  },
"#,
    ),
    (
        "getBalance",
        "  async getBalance(ctx, req): Promise<ProbeServiceGetBalanceOutcome> {
    void `${ctx.loggerName}:${req.organization_id}`;
    return { ok: true, value: { credits: 1 } };
  },
",
    ),
    (
        "settle",
        "  async settle(ctx, req): Promise<ProbeServiceSettleOutcome> {
    void `${ctx.loggerName}:${req.organization_id}`;
    return { ok: true, value: { applied: true } };
  },
",
    ),
    (
        OMITTED,
        r#"  async sweep(ctx, req): Promise<ProbeServiceSweepOutcome> {
    void `${ctx.loggerName}:${Object.keys(req).length}`;
    return { ok: false, error: { errorCode: "db-error" } };
  },
"#,
    ),
    (
        "probeHeader",
        "  async probeHeader(ctx, req, probeTag): Promise<ProbeServiceProbeHeaderOutcome> {
    void req;
    void `${ctx.loggerName}:${probeTag}`;
    return { ok: true, value: { credits: probeTag.length } };
  },
",
    ),
];

/// A caller reading what the client answers with: the value, the operation's own declared error, or
/// the fault behind the literal it narrows on.
#[cfg(feature = "zod")]
const CALLER: &str = r#"import {
  createProbeServiceClient,
  type ProbeServiceFaultKind,
  type ProbeServiceTransport,
} from "./bundle";

const transport: ProbeServiceTransport = {
  async notify(operation, payload, headers): Promise<void> {
    void `${operation}:${JSON.stringify(payload)}:${headers.length}`;
  },
  async request<Answered>(
    operation: string,
    payload: unknown,
    headers: ReadonlyArray<readonly [string, string]>,
  ): Promise<{ answered: Answered; headers: ReadonlyArray<readonly [string, string]> }> {
    throw new Error(`${operation}:${JSON.stringify(payload)}:${headers.length}`);
  },
};

export async function read(): Promise<string> {
  const answered = await createProbeServiceClient(transport).getBalance({
    organization_id: "acme",
  });
  if (answered.ok) {
    const credits: number = answered.value.credits;
    return `${credits}`;
  }
  if ("isServiceFault" in answered.error) {
    const kind: ProbeServiceFaultKind = answered.error.fault.kind;
    const field: string | undefined = answered.error.fault.field;
    return `${kind}:${field ?? ""}:${answered.error.fault.operation}`;
  }
  return answered.error.errorCode;
}
"#;

/// A caller binding the socket transport to a bare `WebSocket`, with `--lib es2020,dom` naming the
/// browser's own declaration for it.
#[cfg(feature = "zod")]
const WS_CALLER: &str = r#"import { createProbeServiceClient, createProbeServiceWsTransport } from "./bundle";

declare const socket: WebSocket;

export async function read(): Promise<string> {
  const client = createProbeServiceClient(createProbeServiceWsTransport(socket));
  const answered = await client.getBalance({ organization_id: "acme" });
  if (answered.ok) {
    return `${answered.value.credits}`;
  }
  return "failed";
}
"#;

/// Everything above the attachment's members, reaching `attachProbeServiceWsDispatcher` as its
/// third argument, with a required `onFault` after it.
#[cfg(feature = "zod")]
const ATTACHMENT_HEAD: &str = r#"import {
  attachProbeServiceWsDispatcher,
  type ProbeServiceExpireCreditOutcome,
  type ProbeServiceFaultKind,
  type ProbeServiceGetBalanceOutcome,
  type ProbeServiceProbeHeaderOutcome,
  type ProbeServiceSettleOutcome,
  type ProbeServiceSweepOutcome,
} from "./bundle";

type ProbeContext = { loggerName: string };

declare const socket: WebSocket;

attachProbeServiceWsDispatcher<ProbeContext>(socket, { loggerName: "probe" }, {
"#;

#[cfg(feature = "zod")]
const ATTACHMENT_TAIL: &str = "}, (fault) => {
  const kind: ProbeServiceFaultKind = fault.kind;
  void kind;
});
";

/// Everything above the HTTP implementation's members, reaching `createProbeServiceHttpDispatcher`.
#[cfg(feature = "zod")]
const HTTP_IMPLEMENTATION_HEAD: &str = r#"import {
  createProbeServiceHttpDispatcher,
  type ProbeServiceExpireCreditOutcome,
  type ProbeServiceGetBalanceOutcome,
  type ProbeServiceHttpRequest,
  type ProbeServiceProbeHeaderOutcome,
  type ProbeServiceSettleOutcome,
  type ProbeServiceSweepOutcome,
} from "./bundle";

type ProbeContext = { loggerName: string };

const dispatch = createProbeServiceHttpDispatcher<ProbeContext>({
"#;

#[cfg(feature = "zod")]
const HTTP_IMPLEMENTATION_TAIL: &str = r#"});

declare const request: ProbeServiceHttpRequest;

export async function read(): Promise<number> {
  const answered = await dispatch({ loggerName: "probe" }, request);
  return answered.status;
}
"#;

/// A caller reading `result.value` as `undefined` once `result.ok` narrows the arm.
#[cfg(feature = "zod")]
const UNIT_SUCCESS_CALLER: &str = r#"import {
  createUnitPingServiceClient,
  type UnitPingServiceTransport,
} from "./bundle";

const transport: UnitPingServiceTransport = {
  async notify(operation, payload, headers): Promise<void> {
    void `${operation}:${JSON.stringify(payload)}:${headers.length}`;
  },
  async request<Answered>(
    operation: string,
    payload: unknown,
    headers: ReadonlyArray<readonly [string, string]>,
  ): Promise<{ answered: Answered; headers: ReadonlyArray<readonly [string, string]> }> {
    throw new Error(`${operation}:${JSON.stringify(payload)}:${headers.length}`);
  },
};

export async function read(): Promise<boolean> {
  const answered = await createUnitPingServiceClient(transport).ping({ probe: "x" });
  if (answered.ok) {
    const value: undefined = answered.value;
    return value === undefined;
  }
  return false;
}
"#;

/// A caller destructuring a header tuple on both arms, each element typed as the operation
/// declared it — an absent optional header being `null`, the value its tuple slot holds.
#[cfg(feature = "zod")]
const HEADER_CALLER: &str = r#"import {
  createHeaderProbeServiceClient,
  createHeaderProbeServiceWsTransport,
  type HeaderProbeServiceTransport,
} from "./bundle";

const transport: HeaderProbeServiceTransport = {
  async notify(operation, payload, headers): Promise<void> {
    void `${operation}:${JSON.stringify(payload)}:${headers.length}`;
  },
  async request<Answered>(
    operation: string,
    payload: unknown,
    headers: ReadonlyArray<readonly [string, string]>,
  ): Promise<{ answered: Answered; headers: ReadonlyArray<readonly [string, string]> }> {
    throw new Error(`${operation}:${JSON.stringify(payload)}:${headers.length}`);
  },
};

declare const socket: WebSocket;

export async function read(overSocket: boolean): Promise<string> {
  const bound = overSocket ? createHeaderProbeServiceWsTransport(socket) : transport;
  const answered = await createHeaderProbeServiceClient(bound).read("doc", "acme", undefined);
  if (answered.ok) {
    const [document, etag, age] = answered.value;
    const title: string = document.title;
    const tag: string = etag;
    const aged: number | null = age;
    return `${title}:${tag}:${aged ?? ""}`;
  }
  if ("isServiceFault" in answered.error) {
    return answered.error.fault.kind;
  }
  const [declared, reason] = answered.error;
  const said: string | null = reason;
  return `${declared.errorCode}:${said ?? ""}`;
}
"#;

/// A caller reading a header tuple's elements over the `http_rest` client: an absent optional
/// `header_out`/`error_header_out` element is `null`, the value its tuple slot holds.
#[cfg(feature = "zod")]
const HEADER_HTTP_CALLER: &str = r#"import {
  createHeaderProbeServiceHttpClient,
  type HeaderProbeServiceHttpTransport,
} from "./bundle";

const transport: HeaderProbeServiceHttpTransport = {
  async send(request) {
    void request;
    return { status: 200, headers: [], body: new TextEncoder().encode("{}") };
  },
};

export async function read(): Promise<string> {
  const client = createHeaderProbeServiceHttpClient(transport);
  const answered = await client.read("doc", "acme", undefined);
  if (answered.ok) {
    const [document, etag, age] = answered.value;
    const title: string = document.title;
    const tag: string = etag;
    const aged: number | null = age;
    return `${title}:${tag}:${aged ?? ""}`;
  }
  if ("isServiceFault" in answered.error) {
    return answered.error.fault.kind;
  }
  const [declared, reason] = answered.error;
  const said: string | null = reason;
  return `${declared.errorCode}:${said ?? ""}`;
}
"#;

/// An implementation answering a header tuple on both arms, reached through the dispatcher
/// factory and the `ws_rpc` attachment alike.
#[cfg(feature = "zod")]
const HEADER_IMPLEMENTATION: &str = r#"import {
  attachHeaderProbeServiceWsDispatcher,
  createHeaderProbeServiceDispatcher,
  type HeaderProbeServiceDispatched,
  type HeaderProbeServiceImpl,
  type HeaderProbeServiceReadOutcome,
} from "./bundle";

type ProbeContext = { loggerName: string };

const implementation: HeaderProbeServiceImpl<ProbeContext> = {
  async read(ctx, req, tenant, trace): Promise<HeaderProbeServiceReadOutcome> {
    if (req === "") {
      return { ok: false, error: [{ errorCode: "missing" }, null] };
    }
    return { ok: true, value: [{ title: `${ctx.loggerName}:${req}` }, tenant, trace ?? null] };
  },
};

declare const socket: WebSocket;

export const detach = attachHeaderProbeServiceWsDispatcher<ProbeContext>(
  socket,
  { loggerName: "probe" },
  implementation,
  (fault) => void fault.kind,
);

export async function answer(): Promise<HeaderProbeServiceDispatched | undefined> {
  const dispatch = createHeaderProbeServiceDispatcher(implementation);
  return dispatch({ loggerName: "probe" }, "read", "doc", [["x-tenant", "\"acme\""]]);
}
"#;

/// Everything above the unit-success implementation's one member.
#[cfg(feature = "zod")]
const UNIT_SUCCESS_IMPLEMENTATION_HEAD: &str = r#"import {
  createUnitPingServiceDispatcher,
  type UnitPingServicePingOutcome,
} from "./bundle";

type PingContext = { loggerName: string };

export const dispatch = createUnitPingServiceDispatcher<PingContext>({
"#;

#[cfg(feature = "zod")]
const UNIT_SUCCESS_IMPLEMENTATION_TAIL: &str = "});\n";

/// Answers `{ ok: true }`, which the outcome type declares.
#[cfg(feature = "zod")]
const UNIT_SUCCESS_MEMBER_OK: &str = "  async ping(ctx, req): Promise<UnitPingServicePingOutcome> {\n    \
     void `${ctx.loggerName}:${req.probe}`;\n    \
     return { ok: true };\n  \
     },\n";

/// Answers `{ ok: true, value: [] }`, which the outcome type refuses.
#[cfg(feature = "zod")]
const UNIT_SUCCESS_MEMBER_OK_WITH_ARRAY: &str = "  async ping(ctx, req): \
     Promise<UnitPingServicePingOutcome> {\n    \
     void `${ctx.loggerName}:${req.probe}`;\n    \
     return { ok: true, value: [] };\n  \
     },\n";

/// Said once per test binary when no `zod` is reachable.
#[cfg(feature = "zod")]
static ZOD_STOOD_DOWN: Once = Once::new();

/// The implementation fixture, with every named operation but the ones listed as left out.
#[cfg(feature = "zod")]
fn implementation(without: &[&str]) -> String {
    let mut written = String::from(IMPLEMENTATION_HEAD);
    for (named, member) in IMPLEMENTATION_MEMBERS {
        if !without.contains(&named) {
            written.push_str(member);
        }
    }
    written.push_str(IMPLEMENTATION_TAIL);
    written
}

/// The attachment fixture, with every named operation but the ones listed as left out.
#[cfg(feature = "zod")]
fn attachment(without: &[&str]) -> String {
    let mut written = String::from(ATTACHMENT_HEAD);
    for (named, member) in IMPLEMENTATION_MEMBERS {
        if !without.contains(&named) {
            written.push_str(member);
        }
    }
    written.push_str(ATTACHMENT_TAIL);
    written
}

/// The bundle plus both socket surfaces: the seam `ts_ws_client()` publishes, which names the
/// socket type the attachment takes, and `ts_ws_service()` itself.
#[cfg(feature = "zod")]
fn bundle_with_ws_seam() -> String {
    format!(
        "{}\n\n{}\n\n{}",
        bundle(),
        ProbeServiceSchema::ts_ws_client(),
        ProbeServiceSchema::ts_ws_service(),
    )
}

/// The bundle plus the `http_rest` server: `ts_http_service()` drives `createProbeServiceDispatcher`
/// and reads `ProbeServiceFault`, both already in `bundle()`, so no further seam is needed beside
/// it.
#[cfg(feature = "zod")]
fn bundle_with_http_service() -> String {
    format!("{}\n\n{}", bundle(), ProbeServiceSchema::ts_http_service())
}

/// The HTTP implementation fixture, with every named operation but the ones listed as left out --
/// mirrors [`implementation`].
#[cfg(feature = "zod")]
fn http_implementation(without: &[&str]) -> String {
    let mut written = String::from(HTTP_IMPLEMENTATION_HEAD);
    for (named, member) in IMPLEMENTATION_MEMBERS {
        if !without.contains(&named) {
            written.push_str(member);
        }
    }
    written.push_str(HTTP_IMPLEMENTATION_TAIL);
    written
}

/// `UnitPingService`'s message types, client, and implementable service.
#[cfg(feature = "zod")]
fn unit_ping_bundle() -> String {
    [
        UnitPingRequest::ts_definition(),
        UnitPingRequest::zod_schema(),
        UnitPingError::ts_definition(),
        UnitPingError::zod_schema(),
        UnitPingServiceSchema::ts_definition(),
        UnitPingServiceSchema::ts_client(),
        UnitPingServiceSchema::ts_service(),
    ]
    .join("\n\n")
}

/// `HeaderProbeService`'s types, client, dispatcher, both `ws_rpc` surfaces and both `http_rest`
/// ones.
#[cfg(feature = "zod")]
fn header_probe_bundle() -> String {
    [
        HeaderProbeDocument::ts_definition(),
        HeaderProbeDocument::zod_schema(),
        HeaderProbeError::ts_definition(),
        HeaderProbeError::zod_schema(),
        HeaderProbeServiceSchema::ts_definition(),
        HeaderProbeServiceSchema::ts_client(),
        HeaderProbeServiceSchema::ts_service(),
        HeaderProbeServiceSchema::ts_ws_client(),
        HeaderProbeServiceSchema::ts_ws_service(),
        HeaderProbeServiceSchema::ts_http_client(),
        HeaderProbeServiceSchema::ts_http_service(),
    ]
    .join("\n\n")
}

/// The `http_rest` server of `SpooledProbeService`, whose `bytes` and `stream` replies answer a
/// declared error with no table, and the dispatcher it drives.
#[cfg(feature = "zod")]
fn spooled_probe_bundle() -> String {
    [
        HeaderProbeError::ts_definition(),
        HeaderProbeError::zod_schema(),
        SpooledProbeServiceSchema::ts_definition(),
        SpooledProbeServiceSchema::ts_service(),
        SpooledProbeServiceSchema::ts_http_service(),
    ]
    .join("\n\n")
}

/// Every surface of `LapsedProbeService`, whose untagged error answers one status whatever it
/// holds.
#[cfg(feature = "zod")]
fn lapsed_probe_bundle() -> String {
    [
        HeaderProbeDocument::ts_definition(),
        HeaderProbeDocument::zod_schema(),
        LapsedProbeError::ts_definition(),
        LapsedProbeError::zod_schema(),
        LapsedProbeServiceSchema::ts_definition(),
        LapsedProbeServiceSchema::ts_client(),
        LapsedProbeServiceSchema::ts_service(),
        LapsedProbeServiceSchema::ts_ws_client(),
        LapsedProbeServiceSchema::ts_ws_service(),
        LapsedProbeServiceSchema::ts_http_client(),
        LapsedProbeServiceSchema::ts_http_service(),
    ]
    .join("\n\n")
}

/// Every surface of `ReservedProbeService`, whose placeholders and header arguments are named
/// after words TypeScript refuses as a parameter or a local, and after locals the emitted
/// functions write themselves.
#[cfg(feature = "zod")]
fn reserved_probe_bundle() -> String {
    [
        HeaderProbeDocument::ts_definition(),
        HeaderProbeDocument::zod_schema(),
        HeaderProbeError::ts_definition(),
        HeaderProbeError::zod_schema(),
        ReservedProbeServiceSchema::ts_definition(),
        ReservedProbeServiceSchema::ts_client(),
        ReservedProbeServiceSchema::ts_service(),
        ReservedProbeServiceSchema::ts_ws_client(),
        ReservedProbeServiceSchema::ts_ws_service(),
        ReservedProbeServiceSchema::ts_http_client(),
        ReservedProbeServiceSchema::ts_http_service(),
    ]
    .join("\n\n")
}

/// Every surface of `QuotedProbeService`, then the `http_rest` surfaces of
/// `QuotedFilingProbeService`, whose names hold a quote, a backslash, a line break and the end of
/// a comment.
#[cfg(feature = "zod")]
fn quoted_probe_bundle() -> String {
    [
        HeaderProbeDocument::ts_definition(),
        HeaderProbeDocument::zod_schema(),
        HeaderProbeError::ts_definition(),
        HeaderProbeError::zod_schema(),
        QuotedProbeServiceSchema::ts_definition(),
        QuotedProbeServiceSchema::ts_client(),
        QuotedProbeServiceSchema::ts_service(),
        QuotedProbeServiceSchema::ts_ws_client(),
        QuotedProbeServiceSchema::ts_ws_service(),
        QuotedProbeServiceSchema::ts_http_client(),
        QuotedProbeServiceSchema::ts_http_service(),
        QuotedFilingProbeServiceSchema::ts_definition(),
        QuotedFilingProbeServiceSchema::ts_service(),
        QuotedFilingProbeServiceSchema::ts_http_client(),
        QuotedFilingProbeServiceSchema::ts_http_service(),
    ]
    .join("\n\n")
}

/// Every surface of `BytesProbeService`, beside the real `zod`.
#[cfg(feature = "zod")]
fn bytes_probe_bundle() -> String {
    [
        "import { z, type ZodType } from \"zod\";".to_owned(),
        HeaderProbeError::ts_definition(),
        HeaderProbeError::zod_schema(),
        BytesProbeServiceSchema::ts_definition(),
        BytesProbeServiceSchema::ts_client(),
        BytesProbeServiceSchema::ts_service(),
        BytesProbeServiceSchema::ts_ws_client(),
        BytesProbeServiceSchema::ts_ws_service(),
        BytesProbeServiceSchema::ts_http_client(),
        BytesProbeServiceSchema::ts_http_service(),
    ]
    .join("\n\n")
}

/// Every surface of `KeyedProbeService`, beside the real `zod`.
#[cfg(feature = "zod")]
fn keyed_probe_bundle() -> String {
    [
        "import { z, type ZodType } from \"zod\";".to_owned(),
        HeaderProbeError::ts_definition(),
        HeaderProbeError::zod_schema(),
        KeyedProbeServiceSchema::ts_definition(),
        KeyedProbeServiceSchema::ts_client(),
        KeyedProbeServiceSchema::ts_service(),
        KeyedProbeServiceSchema::ts_ws_client(),
        KeyedProbeServiceSchema::ts_ws_service(),
        KeyedProbeServiceSchema::ts_http_client(),
        KeyedProbeServiceSchema::ts_http_service(),
    ]
    .join("\n\n")
}

/// Every surface of `SpooledProbeService` and of `StreamedProbeService`, beside the real `zod`.
#[cfg(feature = "zod")]
fn http_alone_bundle() -> String {
    [
        "import { z, type ZodType } from \"zod\";".to_owned(),
        HeaderProbeError::ts_definition(),
        HeaderProbeError::zod_schema(),
        SpooledProbeServiceSchema::ts_definition(),
        SpooledProbeServiceSchema::ts_client(),
        SpooledProbeServiceSchema::ts_service(),
        SpooledProbeServiceSchema::ts_ws_client(),
        SpooledProbeServiceSchema::ts_ws_service(),
        SpooledProbeServiceSchema::ts_http_client(),
        SpooledProbeServiceSchema::ts_http_service(),
        StreamedProbeServiceSchema::ts_definition(),
        StreamedProbeServiceSchema::ts_client(),
        StreamedProbeServiceSchema::ts_service(),
        StreamedProbeServiceSchema::ts_ws_client(),
        StreamedProbeServiceSchema::ts_ws_service(),
        StreamedProbeServiceSchema::ts_http_client(),
        StreamedProbeServiceSchema::ts_http_service(),
    ]
    .join("\n\n")
}

/// [`compiled`], for `files` beside the real `zod` and under [`ZOD_PROJECT`], or `None` where
/// [`MODULES_VAR`] names no directory: a check of what a client reads is no check against the
/// surface above.
#[cfg(feature = "zod")]
fn compiled_beside_zod(named: &str, files: &[(&str, String)]) -> Option<(bool, String)> {
    compiled_beside_zod_then(named, files, |_| ()).map(|(accepted, said, ())| (accepted, said))
}

/// [`compiled_beside_zod`], with `then` handed the workspace once the compiler has run, before it
/// is removed.
#[cfg(feature = "zod")]
fn compiled_beside_zod_then<Then>(
    named: &str,
    files: &[(&str, String)],
    then: impl FnOnce(&Path) -> Then,
) -> Option<(bool, String, Then)> {
    let Some(modules) = env::var(MODULES_VAR)
        .ok()
        .map(|named_at| PathBuf::from(named_at).join("node_modules"))
    else {
        ZOD_STOOD_DOWN.call_once(|| {
            let notice = format!(
                "\ntixschema: no `zod` is reachable, so the clients of a `bytes` reply were NOT \
                 type-checked against it.\n  Name in {MODULES_VAR} a directory whose \
                 `node_modules` holds `zod`, and run `just typecheck-ts`.\n\n"
            );
            drop(stderr().write_all(notice.as_bytes()));
        });
        return None;
    };
    assert!(
        modules.join("zod").is_dir(),
        "{MODULES_VAR} names a directory whose `node_modules` holds no `zod`: {}",
        modules.display()
    );
    let named_compiler = env::var(COMPILER_VAR).ok();
    let compiler = named_compiler.clone().unwrap_or_else(|| "tsc".to_owned());
    let at = workspace(named);
    symlink(&modules, at.join("node_modules")).unwrap();
    fs::write(
        at.join("package.json"),
        "{ \"type\": \"module\", \"private\": true }\n",
    )
    .unwrap();
    fs::write(at.join("tsconfig.json"), ZOD_PROJECT).unwrap();
    for (called, written) in files {
        fs::write(at.join(called), written).unwrap();
    }
    let run = Command::new(&compiler)
        .args(["--pretty", "false", "-p", "tsconfig.json"])
        .current_dir(&at)
        .output();
    let after = run.is_ok().then(|| then(&at));
    fs::remove_dir_all(&at).unwrap();
    let reported = match run {
        Ok(reported) => reported,
        Err(failed) => {
            assert!(
                named_compiler.is_none(),
                "{COMPILER_VAR} names `{compiler}`, and no compiler could be started there: \
                 {failed}"
            );
            stand_down();
            return None;
        }
    };
    let said = format!(
        "{}{}",
        String::from_utf8_lossy(&reported.stdout),
        String::from_utf8_lossy(&reported.stderr)
    );
    after.map(|then_answered| (reported.status.success(), said, then_answered))
}

/// Said on the process's own stderr rather than through `eprintln!`, which `cargo test` captures
/// and only shows for a test that failed. A stand-down is a pass that proved nothing, so it has to
/// be visible on a run where everything passed.
fn stand_down() {
    STOOD_DOWN.call_once(|| {
        let notice = format!(
            "\ntixschema: no TypeScript compiler is reachable, so the emitted bundle was NOT \
             type-checked.\n  The `service_schema` type-check group stood down. Put `tsc` on PATH, \
             or name one in {COMPILER_VAR}, and run `just typecheck-ts`, which refuses to stand \
             down.\n\n"
        );
        drop(stderr().write_all(notice.as_bytes()));
    });
}

/// A directory of its own per check, named for the check and the process, so combinations running
/// one after another and tests running beside each other never share a file.
fn workspace(named: &str) -> PathBuf {
    let at = temp_dir().join(format!("tixschema-typecheck-{named}-{run}", run = id()));
    if at.exists() {
        fs::remove_dir_all(&at).unwrap();
    }
    fs::create_dir_all(&at).unwrap();
    at
}

/// Writes the named files into a workspace of their own, compiles them together, and answers
/// whether the compiler accepted them and everything it reported.
///
/// `None` says no compiler was reachable and nothing was compiled — never that a compile passed.
fn compiled(named: &str, files: &[(&str, String)]) -> Option<(bool, String)> {
    let named_compiler = env::var(COMPILER_VAR).ok();
    let compiler = named_compiler.clone().unwrap_or_else(|| "tsc".to_owned());
    let at = workspace(named);
    for (called, written) in files {
        fs::write(at.join(called), written).unwrap();
    }
    let run = Command::new(&compiler)
        .args(UNDER)
        .args(files.iter().map(|(called, _)| *called))
        .current_dir(&at)
        .output();
    fs::remove_dir_all(&at).unwrap();
    let Ok(reported) = run else {
        assert!(
            named_compiler.is_none(),
            "{COMPILER_VAR} names `{compiler}`, and no compiler could be started there: {}",
            run.unwrap_err()
        );
        stand_down();
        return None;
    };
    let said = format!(
        "{}{}",
        String::from_utf8_lossy(&reported.stdout),
        String::from_utf8_lossy(&reported.stderr)
    );
    Some((reported.status.success(), said))
}

/// The bundle beside whatever it needs in order to be read at all, which is the file set every
/// check starts from: an ambient declaration of the `zod` surface where the build publishes
/// schemas, and nothing whatever where it does not.
#[cfg(feature = "zod")]
fn bundled(written: String) -> Vec<(&'static str, String)> {
    vec![("zod.d.ts", ZOD_SURFACE.to_owned()), ("bundle.ts", written)]
}

/// The other half: a bundle written by a build with no schema surface names nothing it does not
/// itself declare, so it is compiled alone. Nothing supplies it a preamble, because the claim is
/// that it needs none.
#[cfg(not(feature = "zod"))]
fn bundled(written: String) -> Vec<(&'static str, String)> {
    vec![("bundle.ts", written)]
}

#[test]
fn the_bundle_a_consuming_codebase_writes_compiles_under_strict() {
    let Some((accepted, said)) = compiled("bundle", &bundled(bundle())) else {
        return;
    };
    assert!(accepted, "the emitted bundle does not compile:\n{said}");
}

/// Two services in one flat file, compiled.
#[test]
fn two_services_in_one_bundle_compile_together() {
    let mut both = vec![
        BalanceRequest::ts_definition(),
        BalanceResponse::ts_definition(),
        ApplyBundleReceipt::ts_definition(),
        ProbeError::ts_definition(),
        CreditWriteError::ts_definition(),
    ];
    both.extend(author_schemas());
    both.push(ProbeServiceSchema::ts_definition());
    both.extend(probe_seam());
    both.push(AuditServiceSchema::ts_definition());
    both.extend(audit_seam());
    let Some((accepted, said)) = compiled("two-services", &bundled(both.join("\n\n"))) else {
        return;
    };
    assert!(
        accepted,
        "a bundle carrying two services does not compile:\n{said}"
    );
}

#[cfg(feature = "zod")]
#[test]
fn a_complete_implementation_is_accepted_at_the_factory_call() {
    let mut files = bundled(bundle());
    files.push(("implementation.ts", implementation(&[])));
    files.push(("caller.ts", CALLER.to_owned()));
    let Some((accepted, said)) = compiled("complete", &files) else {
        return;
    };
    assert!(
        accepted,
        "an implementation answering every operation, and a caller reading what the client \
         answers with, do not compile against the bundle:\n{said}"
    );
}

/// The claim the whole construct rests on, settled by a compiler rather than read off a string:
/// an implementation missing one operation does not reach the dispatcher factory.
///
/// The file compiled here is the accepted one above with `sweep` dropped and nothing else changed,
/// so a refusal for any other reason is a different diagnostic. Recorded verbatim from tsc 7.0.2
/// under `--strict`:
///
/// ```text
/// implementation.ts(11,68): error TS2741: Property 'sweep' is missing in type '{ applyBundle(ctx: ProbeContext, req: ApplyBundleRequest): Promise<void>; expireCredit(ctx: ProbeContext, req: ExpireCreditRequest): Promise<...>; getBalance(ctx: ProbeContext, req: BalanceRequest): Promise<...>; settle(ctx: ProbeContext, req: BalanceRequest): Promise<...>; }' but required in type 'ProbeServiceImpl<ProbeContext>'.
/// ```
///
/// The assertions read the member's name and the interface's out of that text rather than the
/// error code, so the refusal has to be about the operation left out — a fixture that failed to
/// compile for some unrelated reason names neither.
#[cfg(feature = "zod")]
#[test]
fn an_implementation_missing_one_operation_is_refused_at_the_factory_call() {
    let mut files = bundled(bundle());
    files.push(("implementation.ts", implementation(&[OMITTED])));
    let Some((accepted, said)) = compiled("incomplete", &files) else {
        return;
    };
    assert!(
        !accepted,
        "an implementation answering four of five operations reached \
         `createProbeServiceDispatcher` and the compiler allowed it:\n{said}"
    );
    assert!(
        said.contains(OMITTED),
        "the refusal has to name the operation left out; a fixture refused for some other reason \
         would not. Got:\n{said}"
    );
    assert!(
        said.contains("is missing") && said.contains("ProbeServiceImpl"),
        "the refusal has to be a member missing from the service's own interface. Got:\n{said}"
    );
}

#[cfg(feature = "zod")]
#[test]
fn the_socket_transport_binds_a_browser_websocket() {
    let bundled_with_ws_client = format!("{}\n\n{}", bundle(), ProbeServiceSchema::ts_ws_client());
    let mut files = bundled(bundled_with_ws_client);
    files.push(("caller.ts", WS_CALLER.to_owned()));
    let Some((accepted, said)) = compiled("ws-socket", &files) else {
        return;
    };
    assert!(
        accepted,
        "a browser `WebSocket` assigned into the generated socket seam does not compile:\n{said}"
    );
}

#[cfg(feature = "zod")]
#[test]
fn a_complete_implementation_is_accepted_at_the_dispatcher_attachment() {
    let mut files = bundled(bundle_with_ws_seam());
    files.push(("attachment.ts", attachment(&[])));
    let Some((accepted, said)) = compiled("ws-attach-complete", &files) else {
        return;
    };
    assert!(
        accepted,
        "an implementation answering every operation, attached to a browser `WebSocket` with a \
         required `onFault`, does not compile:\n{said}"
    );
}

#[cfg(feature = "zod")]
#[test]
fn an_implementation_missing_one_operation_is_refused_at_the_dispatcher_attachment() {
    let mut files = bundled(bundle_with_ws_seam());
    files.push(("attachment.ts", attachment(&[OMITTED])));
    let Some((accepted, said)) = compiled("ws-attach-incomplete", &files) else {
        return;
    };
    assert!(
        !accepted,
        "an implementation answering four of five operations reached \
         `attachProbeServiceWsDispatcher` and the compiler allowed it:\n{said}"
    );
    assert!(
        said.contains(OMITTED),
        "the refusal has to name the operation left out. Got:\n{said}"
    );
    assert!(
        said.contains("is missing") && said.contains("ProbeServiceImpl"),
        "the refusal has to be a member missing from the service's own interface. Got:\n{said}"
    );
}

#[cfg(feature = "zod")]
#[test]
fn a_complete_implementation_is_accepted_at_the_http_dispatcher_factory() {
    let mut files = bundled(bundle_with_http_service());
    files.push(("implementation.ts", http_implementation(&[])));
    let Some((accepted, said)) = compiled("http-complete", &files) else {
        return;
    };
    assert!(
        accepted,
        "an implementation answering every operation does not compile against \
         `createProbeServiceHttpDispatcher`:\n{said}"
    );
}

#[cfg(feature = "zod")]
#[test]
fn an_implementation_missing_one_operation_is_refused_at_the_http_dispatcher_factory() {
    let mut files = bundled(bundle_with_http_service());
    files.push(("implementation.ts", http_implementation(&[OMITTED])));
    let Some((accepted, said)) = compiled("http-incomplete", &files) else {
        return;
    };
    assert!(
        !accepted,
        "an implementation answering four of five operations reached \
         `createProbeServiceHttpDispatcher` and the compiler allowed it:\n{said}"
    );
    assert!(
        said.contains(OMITTED),
        "the refusal has to name the operation left out. Got:\n{said}"
    );
    assert!(
        said.contains("is missing") && said.contains("ProbeServiceImpl"),
        "the refusal has to be a member missing from the service's own interface. Got:\n{said}"
    );
}

/// A caller assigns `result.value` to a `const` typed `undefined`.
#[cfg(feature = "zod")]
#[test]
fn a_unit_success_callers_value_narrows_to_undefined() {
    let mut files = bundled(unit_ping_bundle());
    files.push(("caller.ts", UNIT_SUCCESS_CALLER.to_owned()));
    let Some((accepted, said)) = compiled("unit-success-caller", &files) else {
        return;
    };
    assert!(
        accepted,
        "a caller reading a unit success's `value` as `undefined` does not compile:\n{said}"
    );
}

#[cfg(feature = "zod")]
#[test]
fn a_unit_success_implementation_answering_ok_true_is_accepted_at_the_dispatcher_factory() {
    let written = format!(
        "{UNIT_SUCCESS_IMPLEMENTATION_HEAD}{UNIT_SUCCESS_MEMBER_OK}{UNIT_SUCCESS_IMPLEMENTATION_TAIL}"
    );
    let mut files = bundled(unit_ping_bundle());
    files.push(("implementation.ts", written));
    let Some((accepted, said)) = compiled("unit-success-ok", &files) else {
        return;
    };
    assert!(
        accepted,
        "an implementation answering `{{ ok: true }}` for a unit success does not compile against \
         `createUnitPingServiceDispatcher`:\n{said}"
    );
}

/// `{ ok: true, value: [] }` is refused. Recorded verbatim from tsc 7.0.2 under `--strict`:
///
/// ```text
/// implementation.ts(11,24): error TS2353: Object literal may only specify known properties, and 'value' does not exist in type '{ ok: true; }'.
/// ```
#[cfg(feature = "zod")]
#[test]
fn a_unit_success_implementation_answering_ok_true_with_value_is_refused_at_the_dispatcher_factory()
{
    let written = format!(
        "{UNIT_SUCCESS_IMPLEMENTATION_HEAD}{UNIT_SUCCESS_MEMBER_OK_WITH_ARRAY}{UNIT_SUCCESS_IMPLEMENTATION_TAIL}"
    );
    let mut files = bundled(unit_ping_bundle());
    files.push(("implementation.ts", written));
    let Some((accepted, said)) = compiled("unit-success-array", &files) else {
        return;
    };
    assert!(
        !accepted,
        "an implementation answering `{{ ok: true, value: [] }}` for a unit success reached \
         `createUnitPingServiceDispatcher` and the compiler allowed it:\n{said}"
    );
    assert!(
        said.contains("'value' does not exist in type") && said.contains("{ ok: true; }"),
        "the refusal has to be about the excess `value` member. Got:\n{said}"
    );
}

#[cfg(feature = "zod")]
#[test]
fn a_header_tuple_caller_reads_each_element_as_declared() {
    let mut files = bundled(header_probe_bundle());
    files.push(("caller.ts", HEADER_CALLER.to_owned()));
    let Some((accepted, said)) = compiled("header-tuple-caller", &files) else {
        return;
    };
    assert!(
        accepted,
        "a caller destructuring a header tuple does not compile:\n{said}"
    );
}

#[cfg(feature = "zod")]
#[test]
fn a_header_tuple_http_caller_reads_each_element_as_declared() {
    let mut files = bundled(header_probe_bundle());
    files.push(("caller.ts", HEADER_HTTP_CALLER.to_owned()));
    let Some((accepted, said)) = compiled("header-tuple-http-caller", &files) else {
        return;
    };
    assert!(
        accepted,
        "a caller destructuring a header tuple over the http_rest client does not compile:\n{said}"
    );
}

#[cfg(feature = "zod")]
#[test]
fn a_header_tuple_implementation_is_accepted_at_the_dispatcher_and_the_attachment() {
    let mut files = bundled(header_probe_bundle());
    files.push(("implementation.ts", HEADER_IMPLEMENTATION.to_owned()));
    let Some((accepted, said)) = compiled("header-tuple-implementation", &files) else {
        return;
    };
    assert!(
        accepted,
        "an implementation answering header tuples does not compile:\n{said}"
    );
}

#[cfg(feature = "zod")]
#[test]
fn a_service_named_after_reserved_words_type_checks_on_every_surface() {
    let files = bundled(reserved_probe_bundle());
    let Some((accepted, said)) = compiled("reserved-words", &files) else {
        return;
    };
    assert!(
        accepted,
        "a service named after reserved words does not compile:\n{said}"
    );
}

#[cfg(feature = "zod")]
#[test]
fn a_service_whose_names_hold_a_quote_type_checks_on_every_surface() {
    let files = bundled(quoted_probe_bundle());
    let Some((accepted, said)) = compiled("quoted-names", &files) else {
        return;
    };
    assert!(
        accepted,
        "a service whose names hold a quote does not compile:\n{said}"
    );
}

/// The `http_rest` dispatcher of an operation whose declared error answers one status whatever it
/// holds compiles, and answers that status: 422 for `HeaderProbeService`'s `read` and for each
/// operation of `SpooledProbeService`, which declare no table, as the Rust dispatcher answers with
/// no table, and 410 for each operation of `LapsedProbeService`, whose untagged error every row of
/// the table maps to it.
#[cfg(feature = "zod")]
#[test]
fn a_declared_error_of_one_status_type_checks_at_the_http_dispatcher() {
    for (named, written, answered) in [
        (
            "one-status-header",
            header_probe_bundle(),
            vec!["return headerProbeServiceHttpJson(422, headers, declaredError);"],
        ),
        (
            "one-status-spooled",
            spooled_probe_bundle(),
            vec![
                "return spooledProbeServiceHttpJson(422, headers, declaredError);",
                "return spooledProbeServiceHttpJson(422, [], envelope.error);",
            ],
        ),
        (
            "one-status-untagged",
            lapsed_probe_bundle(),
            vec![
                "return lapsedProbeServiceHttpJson(410, headers, declaredError);",
                "return lapsedProbeServiceHttpJson(410, [], envelope.error);",
            ],
        ),
    ] {
        for line in answered {
            assert!(
                written.contains(line),
                "the dispatcher does not answer `{line}`:\n{written}"
            );
        }
        let Some((accepted, said)) = compiled(named, &bundled(written)) else {
            return;
        };
        assert!(
            accepted,
            "a declared error of one status does not compile at the http dispatcher:\n{said}"
        );
    }
}

/// Every TypeScript client of a `bytes` operation compiles under `--strict` against the real
/// `zod`, and answers the bytes as the `Array<number>` its result declares: the `http_rest`
/// client reads them off the seam's binary body, and the transport-agnostic client, over a bus
/// and over `ws_rpc`, parses the pair of the bytes and their content type the dispatcher answers.
#[cfg(feature = "zod")]
#[test]
fn every_client_of_a_bytes_operation_type_checks_against_the_real_zod() {
    let files = [
        ("bundle.ts", bytes_probe_bundle()),
        ("caller.ts", BYTES_CALLER.to_owned()),
    ];
    let Some((accepted, said)) = compiled_beside_zod("bytes-clients", &files) else {
        return;
    };
    assert!(
        accepted,
        "a client of a `bytes` operation does not compile against the real `zod`:\n{said}"
    );
}

/// Every TypeScript surface of a service with an operation only `http_rest` carries compiles
/// under `--strict` against the real `zod`, a service of no other operation among them: the
/// `http_rest` client and dispatcher hold the operation, and the transport-agnostic and `ws_rpc`
/// client and dispatcher name neither it nor `StreamedAnswer`, so a caller asking them for it is
/// refused.
#[cfg(feature = "zod")]
#[test]
fn a_service_with_an_operation_only_http_rest_carries_type_checks_against_the_real_zod() {
    for enveloped in [
        SpooledProbeServiceSchema::ts_client(),
        SpooledProbeServiceSchema::ts_ws_client(),
        StreamedProbeServiceSchema::ts_client(),
        StreamedProbeServiceSchema::ts_ws_client(),
    ] {
        assert!(
            !enveloped.contains("StreamedAnswer") && !enveloped.contains("stream("),
            "a `ws_rpc` or transport-agnostic client names the streamed operation:\n{enveloped}"
        );
    }
    let files = [
        ("bundle.ts", http_alone_bundle()),
        ("caller.ts", HTTP_ALONE_CALLER.to_owned()),
    ];
    let Some((accepted, said)) = compiled_beside_zod("http-alone", &files) else {
        return;
    };
    assert!(
        accepted,
        "a service with an operation only `http_rest` carries does not compile:\n{said}"
    );
    let misused = [
        ("bundle.ts", http_alone_bundle()),
        ("caller.ts", HTTP_ALONE_MISUSE.to_owned()),
    ];
    let Some((compiled, told)) = compiled_beside_zod("http-alone-misused", &misused) else {
        return;
    };
    assert!(
        !compiled
            && told.contains(
                "error TS2339: Property 'stream' does not exist on type \
                 'SpooledProbeServiceClient'."
            )
            && told.contains("'stream' does not exist in type"),
        "a caller asking the transport-agnostic client or dispatcher for the streamed operation \
         is not refused:\n{told}"
    );
}

/// Each method and each arm of a result of `KeyedProbeService` that writes a map keyed by a 64-bit
/// integer carries a `@deprecated` note, which `tsc --strict` prints nothing for against the real
/// `zod`, and which the TypeScript language service, the service an editor asks, reports where it
/// is read; one keyed by a `u32` carries none.
#[cfg(feature = "zod")]
#[test]
fn a_map_keyed_by_a_64_bit_integer_is_marked_where_an_editor_reads_it_on_every_surface() {
    let files = [
        ("bundle.ts", keyed_probe_bundle()),
        ("caller.ts", KEYED_CALLER.to_owned()),
    ];
    let named_node = env::var(NODE_VAR).ok();
    let node = named_node.clone().unwrap_or_else(|| "node".to_owned());
    let Some((accepted, said, run)) = compiled_beside_zod_then("keyed", &files, |at| {
        assert!(
            at.join("node_modules/typescript").is_dir(),
            "{MODULES_VAR} names a directory whose `node_modules` holds no `typescript`, which \
             the language service is read from"
        );
        fs::write(at.join("deprecated_reads.mjs"), DEPRECATED_READS).unwrap();
        Command::new(&node)
            .args(["deprecated_reads.mjs", "bundle.ts", "caller.ts"])
            .current_dir(at)
            .output()
    }) else {
        return;
    };
    assert!(accepted && said.is_empty(), "tsc --strict reports:\n{said}");
    let Ok(ran) = run else {
        assert!(
            named_node.is_none(),
            "{NODE_VAR} names `{node}`, and no runtime could be started there: {}",
            run.unwrap_err()
        );
        return;
    };
    assert_eq!(
        String::from_utf8_lossy(&ran.stdout).trim(),
        KEYED_DEPRECATED.join("\n"),
        "the language service does not report each note where it is read:\n{}",
        String::from_utf8_lossy(&ran.stderr)
    );
}
