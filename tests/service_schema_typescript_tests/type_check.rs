//! The emitted bundle put through a real TypeScript compiler.
//!
//! Every other assertion about the published TypeScript in this repository reads strings. This
//! group does not: it writes the bundle a consuming codebase would write, hands it to `tsc
//! --strict`, and reads the verdict. What that settles is the one claim the construct rests on and
//! no string test can reach — an implementation missing a single operation is refused where it
//! reaches the dispatcher factory, and the same implementation with the operation present compiles
//! clean.
//!
//! **Where the compiler comes from.** `tsc` is looked up on `PATH`, or at whatever
//! `TIXSCHEMA_TSC` names. A repository cannot assume one is installed, so a build that finds none
//! stands down rather than failing: the notice below goes to the process's own stderr, which
//! `cargo test` does not capture, so a run that proved nothing here says so on the terminal.
//! `just typecheck-ts` is the entry point that refuses to stand down.
//!
//! **What is checked and what is not.** The bundle names `z` and `ZodType` without importing
//! them — by design, the crate emits no preamble — so this group supplies an ambient declaration
//! of the surface the emitter actually uses. That declaration is a floor, not `zod`: it makes the
//! schema *expressions* well-typed without claiming each one infers its own type. Everything else
//! is checked for real, the interface and the factory included, and neither mentions `zod`.

use super::the_bundle_one_registration_line_produces::{
    audit_seam, author_schemas, bundle, probe_seam,
};
use super::{
    ApplyBundleReceipt, AuditServiceSchema, BalanceRequest, BalanceResponse, CreditWriteError,
    ProbeError, ProbeServiceSchema,
};
#[cfg(feature = "zod")]
use super::{
    HeaderProbeDocument, HeaderProbeError, HeaderProbeServiceSchema, UnitPingError,
    UnitPingRequest, UnitPingServiceSchema,
};
use std::env;
use std::env::temp_dir;
use std::fs;
use std::io::Write as _;
use std::io::stderr;
use std::path::PathBuf;
use std::process::{Command, id};
use std::sync::Once;

/// Names the compiler to run, for a machine that has one somewhere other than `PATH`. Set, and a
/// compiler that cannot be started is a failure rather than a stand-down: somebody said where it
/// was.
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

/// The surface of `zod` the emitter actually names, declared globally because the bundle names
/// `z` and `ZodType` without importing them.
///
/// `ZodBuilder` answers `never`, which is assignable into every `ZodType<T>` the bundle annotates
/// a schema with — so a builder chain satisfies its annotation without this declaration having to
/// reimplement zod's inference. `safeParse` is typed exactly as zod types it, which is what makes
/// the dispatcher's `impl.getBalance(ctx, received.data)` a real check rather than one against
/// `unknown`.
#[cfg(feature = "zod")]
const ZOD_SURFACE: &str = "type ZodIssue = { path: ReadonlyArray<PropertyKey>; message: string };

type ZodParsed<Parsed> =
  | { success: true; data: Parsed }
  | { success: false; error: { issues: ReadonlyArray<ZodIssue> } };

declare type ZodType<Parsed> = {
  safeParse(value: unknown): ZodParsed<Parsed>;
};

declare type ZodBuilder = ZodType<never> & {
  int(): ZodBuilder;
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

/// Everything above the implementation's members. The object literal reaches the factory
/// unannotated and with the context named explicitly, so what refuses an incomplete one is the
/// call rather than an annotation written here.
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

/// One member per operation the service declares. The incomplete fixture is this list with
/// [`OMITTED`] dropped and nothing else changed, so the two files differ by exactly one member and
/// a slip in either is a slip in both.
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

/// A caller reading what the client answers with: the value, the operation's own declared error,
/// or the fault behind the literal it narrows on. Nothing here asserts — it compiling at all is
/// what says the published result types narrow the way the design claims.
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

/// A caller binding the socket transport to a bare `WebSocket`, with `--lib es2020,dom` naming
/// the browser's own declaration for it. Nothing here asserts either — a browser socket satisfying
/// the seam with no adapter is what compiling at all says.
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

/// Everything above the attachment's members: the same [`IMPLEMENTATION_MEMBERS`] the bare factory
/// is checked with, reaching `attachProbeServiceWsDispatcher` as its third argument instead of
/// `createProbeServiceDispatcher`'s only one, with a required `onFault` after it.
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

/// Everything above the HTTP implementation's members: the same [`IMPLEMENTATION_MEMBERS`] the
/// bare factory and the `ws_rpc` attachment are checked with, reaching
/// `createProbeServiceHttpDispatcher` instead.
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

// ---------------------------------------------------------------------------------------------
// `UnitPingService`: a standalone one-operation unit-success service, unrelated to `ProbeService`.
// ---------------------------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------------------------
// `HeaderProbeService`: headers both ways, over the generic client and the `ws_rpc` pair.
// ---------------------------------------------------------------------------------------------

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

/// `HeaderProbeService`'s types, client, dispatcher, and both `ws_rpc` surfaces.
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
    ]
    .join("\n\n")
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

/// The bundle a consuming codebase writes, compiled.
///
/// This is what the structural checks in the file beside this one cannot do: a bundle whose
/// emitted text is well-formed to a reader and rejected by a parser reads identically to one that
/// compiles. It runs in every build that writes TypeScript, and in a build with no schema surface
/// the bundle is handed to the compiler entirely on its own — nothing declares a name for it, so
/// a clean compile is also what says it carries no unresolved one.
#[test]
fn the_bundle_a_consuming_codebase_writes_compiles_under_strict() {
    let Some((accepted, said)) = compiled("bundle", &bundled(bundle())) else {
        return;
    };
    assert!(accepted, "the emitted bundle does not compile:\n{said}");
}

/// Two services in one flat file, compiled. The check beside this one reads the declared names and
/// compares them for duplicates; this one asks the compiler, which also sees a collision between a
/// name one service declares and one the other's generated code refers to.
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

/// The positive half of the seal, which no string test can give: an implementation that answers
/// every operation is accepted where it reaches the dispatcher factory, and a caller narrows what
/// the client answers with.
///
/// Compiled together, so the run that says the incomplete implementation below is refused is a run
/// against a file set that is otherwise known to compile.
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

/// What no string test can give the socket seam: a browser's own `WebSocket`, typed by `--lib
/// es2020,dom` rather than by anything this crate declares, satisfies it with no adapter written
/// in between.
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

/// The positive half of the attachment's own seal: an implementation answering every operation is
/// accepted where it reaches `attachProbeServiceWsDispatcher`, bound to a browser `WebSocket`, with
/// a required `onFault` whose parameter narrows to the published fault kind.
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

/// The negative half: an implementation missing one operation, handed to the attachment exactly as
/// it is handed to the bare dispatcher factory above, is refused the same way.
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

/// The positive half of `ts_http_service()`'s own seal: the same implementation is accepted where
/// it reaches `createProbeServiceHttpDispatcher`, wrapping the same `ProbeServiceImpl<Ctx>`.
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

/// The negative half: an implementation missing one operation, handed to
/// `createProbeServiceHttpDispatcher` exactly as above, is refused the same way.
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

/// `{ ok: true }` is accepted at the dispatcher factory.
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

/// A caller reads a header tuple's elements as the operation declared them, over the generic
/// seam and the `ws_rpc` transport alike.
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

/// An implementation answering header tuples is accepted at the dispatcher factory and at the
/// `ws_rpc` attachment, and the dispatcher answers the envelope beside its headers.
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
