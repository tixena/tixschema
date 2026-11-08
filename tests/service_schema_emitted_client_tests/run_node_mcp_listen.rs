//! The emitted TypeScript `mcp` server run by node over `subscriptions/listen`: it writes and
//! answers the bytes the Rust dispatcher writes and answers to the same requests, handed the same
//! changes, on stdio, on an endpoint of two services and over plain HTTP terms, and every message
//! is held to the official 2026-07-28 schema. Each stream ends on the events it observes.

use super::mcp_library_service::{BookError, BookInfo, BookRequest};
use super::mcp_listen_rows::{assert_matches, input};
use super::runtime::{compiled_with_modules, node_modules, ran_with_modules, stand_down_modules};
use super::{DepotServiceSchema, LibraryServiceSchema};
use crate::mcp_streams::NODE_RESPOND;
use serde_json::Value;

/// Names the runtime to run, for a machine that has one somewhere other than `PATH`.
const RUNTIME_VAR: &str = "TIXSCHEMA_NODE";

/// Names the compiler the emitted text is type-checked with.
const COMPILER_VAR: &str = "TIXSCHEMA_TSC";

/// The one package the drivers import at runtime: `zod`, for the schemas the dispatcher judges by.
const REQUIRED_PACKAGES: &[&str] = &["zod"];

const DRIVER: &str = r#"
const nothing = {};
const changesOf = (script) => {
  const queue = [...script];
  return async () => queue.shift();
};

function lines() {
  const written = [];
  return { written, write: async (message) => { written.push(message); } };
}

async function main() {
  const input = JSON.parse(INPUT);
  const dispatch = createLibraryServiceMcpDispatcher(nothing);
  const library = createLibraryServiceMcpRoute(nothing);
  const depot = createDepotServiceMcpRoute(nothing);
  const workspace = libraryServiceMcpEndpoint(libraryServiceMcpPart, depotServiceMcpPart);
  const open = () => ({});

  const stdio = [];
  for (const row of input.listens) {
    const out = lines();
    const reply = await dispatch(row.request, open, out.write, new LibraryServiceMcpInFlight(), changesOf(row.script));
    stdio.push({ reply: reply ?? null, written: out.written });
  }

  const out = lines();
  const served = await libraryServiceMcpServe(
    input.listens[0].request, "Workspace", undefined, workspace, open,
    { writer: out.write, inFlight: new LibraryServiceMcpInFlight(), changes: changesOf(input.listens[0].script) },
    async (ctx, part, method, params) => (part === 0 ? library(ctx, method, params) : part === 1 ? depot(ctx, method, params) : undefined));
  const workspaceReport = { reply: served ?? null, written: out.written };

  const http = [];
  for (const row of input.http) {
    const over = createLibraryServiceMcpHttpDispatcher(nothing, { allowedOrigins: [] }, async () => ({ ctx: {} }), changesOf(row.script));
    const response = await respond(over, { method: row.method, path: row.path, headers: row.headers, body: new TextEncoder().encode(row.body) });
    http.push({ status: response.status, headers: response.headers.map(([name, value]) => [name, value]), body: new TextDecoder().decode(response.body) });
  }

  const first = input.http[0];
  const plainOver = createLibraryServiceMcpHttpDispatcher(nothing, { allowedOrigins: [] }, async () => ({ ctx: {} }));
  const plainResponse = await respond(plainOver, { method: first.method, path: first.path, headers: first.headers, body: new TextEncoder().encode(first.body) });
  const plainOut = lines();
  const plainReply = await dispatch(input.listens[0].request, open, plainOut.write, new LibraryServiceMcpInFlight());

  const discoverOut = lines();
  const listening = await dispatch(input.discover, open, discoverOut.write, new LibraryServiceMcpInFlight(), changesOf([]));
  const plainDiscover = await dispatch(input.discover, open, discoverOut.write, new LibraryServiceMcpInFlight());

  const [listenLine, cancelLine] = input.cancel;
  const cancelOut = lines();
  const waiting = [];
  const stdout = async (message) => {
    cancelOut.written.push(message);
    for (const wake of waiting.splice(0)) wake();
  };
  const inFlight = new LibraryServiceMcpInFlight();
  const pending = dispatch(listenLine, open, stdout, inFlight, () => new Promise(() => {}));
  while (cancelOut.written.length < 1) await new Promise((resolve) => waiting.push(resolve));
  const start = cancelOut.written.length;
  const stopped = await dispatch(cancelLine, open, stdout, inFlight);
  if (stopped !== undefined) throw new Error("a notification was answered");
  const cancelledReply = await pending;

  const goneOut = lines();
  const goneReply = await dispatch(input.gone.request, open, async () => { throw new Error("the client has gone"); }, new LibraryServiceMcpInFlight(), changesOf(input.gone.script));

  const asRequest = (row) => ({ method: row.method, path: row.path, headers: row.headers, body: new TextEncoder().encode(row.body) });
  const recording = (goneAfter) => {
    const seen = [];
    const changed = [];
    const stream = {
      attempts: 0,
      seen,
      until: async (done) => {
        while (!done()) await new Promise((resolve) => changed.push(resolve));
      },
      take() {
        stream.attempts += 1;
        const refused = goneAfter !== undefined && seen.length >= goneAfter;
        for (const wake of changed.splice(0)) wake();
        if (refused) throw new Error("the client has gone");
      },
      async begin(status, headers) {
        stream.take();
        seen.push({ began: [status, headers.map(([name, value]) => [name, value])] });
        for (const wake of changed.splice(0)) wake();
      },
      async write(event) {
        stream.take();
        seen.push({ wrote: event });
        for (const wake of changed.splice(0)) wake();
      },
    };
    return stream;
  };
  const makeFeed = () => {
    const pending = [];
    let waiting;
    let closed = false;
    const settle = () => {
      if (waiting === undefined) return;
      if (pending.length > 0) {
        const resolve = waiting;
        waiting = undefined;
        resolve(pending.shift());
      } else if (closed) {
        const resolve = waiting;
        waiting = undefined;
        resolve(undefined);
      }
    };
    return {
      next: () => new Promise((resolve) => { waiting = resolve; settle(); }),
      push(change) { pending.push(change); settle(); },
      close() { closed = true; settle(); },
    };
  };
  const streams = {};
  {
    const stream = recording();
    const feed = makeFeed();
    const over = createLibraryServiceMcpHttpDispatcher(nothing, { allowedOrigins: [] }, async () => ({ ctx: {} }), () => feed.next());
    let ended = false;
    const running = over(asRequest(input.streamed), stream).then((answered) => {
      ended = true;
      return answered;
    });
    const snapshots = [];
    await stream.until(() => stream.seen.length >= 2);
    snapshots.push(stream.seen.slice());
    let expected = 2;
    for (const change of [{ kind: "resourcesListChanged" }, { kind: "resourceUpdated", uri: "library://covers/book-1" }, { kind: "keepAlive" }]) {
      feed.push(change);
      expected += 1;
      await stream.until(() => stream.seen.length >= expected);
      snapshots.push(stream.seen.slice());
    }
    const endedWhileOpen = ended;
    feed.close();
    const answered = await running;
    streams.ordered = { endedWhileOpen, full: stream.seen.slice(), returned: answered !== undefined, snapshots };
  }
  {
    const stream = recording(1);
    let taken;
    const over = createLibraryServiceMcpHttpDispatcher(nothing, { allowedOrigins: [] }, async (request, call) => {
      taken = call;
      return { ctx: {} };
    }, () => new Promise(() => {}));
    const answered = await over(asRequest(input.streamed), stream);
    streams.gone = { cancelled: taken.cancelled, returned: answered !== undefined, seen: stream.seen.slice() };
  }

  console.log(JSON.stringify({
    streams,
    stdio,
    workspace: workspaceReport,
    http,
    plain: {
      http: { status: plainResponse.status, headers: plainResponse.headers.map(([name, value]) => [name, value]), body: new TextDecoder().decode(plainResponse.body) },
      stdio: plainReply ?? null,
    },
    discover: { listening: listening ?? null, plain: plainDiscover ?? null },
    cancelled: { reply: cancelledReply ?? null, running: 0, start, written: cancelOut.written },
    gone: { reply: goneReply ?? null, written: goneOut.written },
  }));
  process.exit(0);
}
main().catch((error) => { console.error(error); process.exit(1); });
"#;

/// A caller that hands in changes the way a consumer does.
const USAGE_CALLER: &str = r#"import {
  createLibraryServiceMcpDispatcher,
  createLibraryServiceMcpHttpDispatcher,
  type LibraryServiceImpl,
  type LibraryServiceMcpChange,
  type LibraryServiceMcpChanges,
  type LibraryServiceMcpInFlight,
  type LibraryServiceMcpWriter,
} from "./bundle.js";

declare const impl: LibraryServiceImpl<number>;
declare const inFlight: LibraryServiceMcpInFlight;
declare const writer: LibraryServiceMcpWriter;

const queue: Array<LibraryServiceMcpChange> = [
  { kind: "resourcesListChanged" },
  { kind: "resourceUpdated", uri: "library://covers/book-1" },
  { kind: "task", taskId: "export:1" },
  { kind: "keepAlive" },
];
export const changes: LibraryServiceMcpChanges = async () => queue.shift();
export const dispatcher: (
  text: string,
  ctx: () => number,
  writer: LibraryServiceMcpWriter,
  inFlight: LibraryServiceMcpInFlight,
  changes?: LibraryServiceMcpChanges,
) => Promise<string | undefined> = createLibraryServiceMcpDispatcher(impl);
export const answered: Promise<string | undefined> = dispatcher("{}", () => 1, writer, inFlight, changes);
export const over = createLibraryServiceMcpHttpDispatcher(
  impl,
  { allowedOrigins: [] },
  async () => ({ ctx: 1 }),
  changes,
);
"#;

fn bundle() -> String {
    [
        "import { z, type ZodType, type $brand } from \"zod\";".to_owned(),
        BookRequest::ts_definition(),
        BookRequest::zod_schema(),
        BookInfo::ts_definition(),
        BookInfo::zod_schema(),
        BookError::ts_definition(),
        BookError::zod_schema(),
        LibraryServiceSchema::ts_definition(),
        LibraryServiceSchema::ts_service(),
        LibraryServiceSchema::ts_mcp_service(),
        DepotServiceSchema::ts_definition(),
        DepotServiceSchema::ts_service(),
        DepotServiceSchema::ts_mcp_service(),
    ]
    .join("\n\n")
}

/// The report of the one node run; `None` where no node toolchain was reachable.
fn driven() -> Option<Value> {
    let Some(modules) = node_modules(REQUIRED_PACKAGES) else {
        stand_down_modules(
            REQUIRED_PACKAGES,
            "the emitted TypeScript MCP server over subscriptions",
        );
        return None;
    };
    let literal = Value::String(input().to_string());
    let source = format!(
        "{}\n\n{}\nconst INPUT = {literal};\n{DRIVER}",
        bundle(),
        NODE_RESPOND
    );
    let wrote = ran_with_modules(
        "mcp-listen",
        RUNTIME_VAR,
        "node",
        "mcp-listen.mts",
        &source,
        &modules,
    )?;
    Some(serde_json::from_str(wrote.trim()).unwrap())
}

#[test]
fn the_typescript_server_writes_and_answers_what_the_rust_dispatcher_does_on_a_listen() {
    let Some(report) = driven() else {
        return;
    };
    assert_matches(&report);
}

#[test]
fn the_listen_text_type_checks_under_strict() {
    let Some(modules) = node_modules(REQUIRED_PACKAGES) else {
        stand_down_modules(
            REQUIRED_PACKAGES,
            "the type-check of the emitted MCP text over subscriptions",
        );
        return;
    };
    let text = bundle();
    let Some((succeeded, said)) = compiled_with_modules(
        "mcp-listen-types",
        COMPILER_VAR,
        &[("bundle.ts", &text), ("usage.ts", USAGE_CALLER)],
        &modules,
    ) else {
        return;
    };
    assert!(succeeded, "the emitted text does not type-check:\n{said}");
}
