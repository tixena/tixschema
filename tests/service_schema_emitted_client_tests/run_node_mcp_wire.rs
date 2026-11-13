//! The emitted TypeScript clients and servers of `ShelfWireService` on the `http_rest` and `ws_rpc`
//! transports, run by node: an operation answering content blocks or prompt messages is written
//! and read in the block form the `mcp` transport writes, which is what the Rust dispatchers write.

use super::ShelfWireServiceSchema;
use super::mcp_wire_rust::{BLOCKS, MESSAGES, PAGE, over_http, over_ws};
use super::mcp_wire_service::{ShelfCount, ShelfWireError, ShelfWireRequest};
use super::runtime::{compiled_with_modules, node_modules, ran_with_modules, stand_down_modules};
use serde_json::{Value, json};

const RUNTIME_VAR: &str = "TIXSCHEMA_NODE";
const COMPILER_VAR: &str = "TIXSCHEMA_TSC";
const REQUIRED_PACKAGES: &[&str] = &["zod"];

const DRIVER: &str = r##"
function fakeSocket(onSend) {
  const listeners = { message: new Set(), close: new Set() };
  return {
    send(text) { onSend(text); },
    close() { for (const listener of listeners.close) listener(); },
    addEventListener(type, listener) { listeners[type].add(listener); },
    removeEventListener(type, listener) { listeners[type].delete(listener); },
    deliver(text) { for (const listener of listeners.message) listener({ data: text }); },
  };
}

const blocks = [
  { type: "text", text: "Eleven books." },
  { type: "image", data: new Uint8Array([0x89, 0x50, 0x4e, 0x47]), mimeType: "image/png" },
  { type: "audio", data: new Uint8Array([0xff, 0xfb, 0x90]), mimeType: "audio/mpeg" },
  { type: "resource_link", uri: "shelf://history", name: "history", title: "History", mimeType: "text/markdown" },
  { type: "resource", uri: "shelf://history/index", mimeType: "text/markdown", text: "# History" },
  { type: "resource", uri: "shelf://history/cover", blob: new Uint8Array([9, 8, 7]) },
];
const messages = [
  { role: "user", content: { type: "text", text: "Review the history shelf." } },
  { role: "assistant", content: { type: "text", text: "Which books?" } },
];
const page = {
  resources: [{ uri: "shelf://history", name: "history", mimeType: "text/markdown", size: 11 }],
  nextCursor: "page-2",
};
const counting = { taskId: "count-1", createdAt: "2026-07-28T09:00:00Z", lastUpdatedAt: "2026-07-28T09:00:00Z", ttlMs: null };
const impl = {
  async countStatus(ctx, taskId) {
    return taskId === "count-1"
      ? { ok: true, value: { ...counting, status: "completed", result: { books: 11 } } }
      : { ok: false, error: { errorCode: "not-found" } };
  },
  async describeShelf(ctx, req) {
    return req.shelf === "history" ? { ok: true, value: blocks } : { ok: false, error: { errorCode: "not-found" } };
  },
  async listShelves(ctx, cursor) {
    return cursor === undefined || cursor === null
      ? { ok: true, value: page }
      : { ok: false, error: { errorCode: "not-found" } };
  },
  async reviewShelf(ctx, shelf) {
    return shelf === "history" ? { ok: true, value: messages } : { ok: false, error: { errorCode: "not-found" } };
  },
  async startCount(ctx, req) {
    return req.shelf === "history" ? { ok: true, value: counting } : { ok: false, error: { errorCode: "not-found" } };
  },
};

const wired = (result, wire) => (result.ok ? { ok: true, value: result.value.map(wire) } : result);
const bytes = (text) => new TextEncoder().encode(text);

async function readBy(client) {
  return {
    describe: wired(await client.describeShelf({ shelf: "history" }), McpContent$Wire),
    review: wired(await client.reviewShelf("history"), McpPromptMessage$Wire),
    list: await client.listShelves(undefined),
    missing: await client.reviewShelf("fiction"),
  };
}

async function main() {
  const served = createShelfWireServiceHttpDispatcher(impl);
  const request = (path, payload) => ({
    method: "POST",
    path,
    query: "",
    headers: [["content-type", "application/json"]],
    body: bytes(JSON.stringify(payload)),
  });
  const text = async (path, payload) => new TextDecoder().decode((await served({}, request(path, payload))).body);
  const written = {
    describe: await text("/describe-shelf", { shelf: "history" }),
    review: await text("/review-shelf", "history"),
    list: await text("/list-shelves", null),
  };

  const dispatch = createShelfWireServiceDispatcher(impl);
  const clients = {
    http: createShelfWireServiceHttpClient({ async send(sent) { return served({}, sent); } }),
    rust: createShelfWireServiceHttpClient({
      async send(sent) {
        const answer = RUST_HTTP[sent.path];
        return { status: answer.status, headers: answer.headers, body: new Uint8Array(answer.body) };
      },
    }),
    bus: createShelfWireServiceClient({
      async notify() {},
      async request(operation, payload, headers) {
        return JSON.parse(JSON.stringify(await dispatch({}, operation, JSON.parse(JSON.stringify(payload ?? null)), headers)));
      },
    }),
  };
  let serverSocket;
  const clientSocket = fakeSocket((frame) => queueMicrotask(() => serverSocket.deliver(frame)));
  serverSocket = fakeSocket((frame) => queueMicrotask(() => clientSocket.deliver(frame)));
  attachShelfWireServiceWsDispatcher(serverSocket, {}, impl, () => {});
  clients.socket = createShelfWireServiceClient(createShelfWireServiceWsTransport(clientSocket, { heartbeat: false }));
  const rustSocket = fakeSocket((frame) => {
    const sent = JSON.parse(frame);
    const reply = { ...JSON.parse(RUST_REPLIES[sent.operation]), id: sent.id };
    queueMicrotask(() => rustSocket.deliver(JSON.stringify(reply)));
  });
  clients.rustSocket = createShelfWireServiceClient(createShelfWireServiceWsTransport(rustSocket, { heartbeat: false }));

  const read = {};
  for (const [named, client] of Object.entries(clients)) read[named] = await readBy(client);

  const replies = [];
  let settle;
  const settled = new Promise((resolve) => { settle = resolve; });
  const framed = fakeSocket((frame) => { replies.push(JSON.parse(frame)); if (replies.length === FRAMES.length) settle(); });
  attachShelfWireServiceWsDispatcher(framed, {}, impl, () => {});
  for (const frame of FRAMES) framed.deliver(frame);
  await settled;
  console.log(JSON.stringify({ written, read, replies }));
  process.exit(0);
}
main().catch((error) => { console.error(error); process.exit(1); });
"##;

const USAGE: &str = r#"import {
  ShelfWireRequest$Schema,
  createShelfWireServiceClient,
  createShelfWireServiceHttpClient,
  type McpContent,
  type McpPromptMessage,
  type ShelfWireServiceImpl,
  McpContent$Schema,
} from "./bundle.js";

declare const impl: ShelfWireServiceImpl<number>;
declare const transport: Parameters<typeof createShelfWireServiceClient>[0];

export const block: McpContent = McpContent$Schema.parse({ type: "text", text: "a" });
export async function usage() {
  const client = createShelfWireServiceClient(transport);
  const described = await client.describeShelf(ShelfWireRequest$Schema.parse({ shelf: "history" }));
  if (described.ok) {
    const first: McpContent | undefined = described.value[0];
    return first === undefined ? 0 : 1;
  }
  const reviewed = await client.reviewShelf("history");
  if (reviewed.ok) {
    const first: McpPromptMessage | undefined = reviewed.value[0];
    return first === undefined ? 0 : 1;
  }
  return impl === undefined ? 2 : 3;
}
export const http = createShelfWireServiceHttpClient;
"#;

fn bundle() -> String {
    [
        "import { z, type ZodType, type $brand } from \"zod\";".to_owned(),
        ShelfWireError::ts_definition(),
        ShelfWireError::zod_schema(),
        ShelfWireRequest::ts_definition(),
        ShelfWireRequest::zod_schema(),
        ShelfCount::ts_definition(),
        ShelfCount::zod_schema(),
        ShelfWireServiceSchema::ts_definition(),
        ShelfWireServiceSchema::ts_mcp_content(),
        ShelfWireServiceSchema::ts_service(),
        ShelfWireServiceSchema::ts_http_service(),
        ShelfWireServiceSchema::ts_http_client(),
        ShelfWireServiceSchema::ts_client(),
        ShelfWireServiceSchema::ts_ws_client(),
        ShelfWireServiceSchema::ts_ws_service(),
    ]
    .join("\n\n")
}

/// The answers the Rust dispatchers write, which the driver hands the clients and compares the
/// servers against.
fn preamble() -> (String, Vec<Value>) {
    let history = json!({ "shelf": "history" });
    let http = |path: &str, payload: &Value| {
        let (status, body) = over_http(path, payload);
        json!({
            "status": status,
            "headers": [["content-type", "application/json"]],
            "body": body.into_bytes(),
        })
    };
    let ws = |operation: &str, payload: &Value| over_ws(operation, payload);
    let (describe_frame, describe_reply) = ws("describe-shelf", &history);
    let (review_frame, review_reply) = ws("review-shelf", &json!("history"));
    let (list_frame, list_reply) = ws("list-shelves", &Value::Null);
    let rust_http = json!({
        "/describe-shelf": http("/describe-shelf", &history),
        "/review-shelf": http("/review-shelf", &json!("history")),
        "/list-shelves": http("/list-shelves", &Value::Null),
    });
    let rust_replies = json!({ "describe-shelf": describe_reply, "list-shelves": list_reply, "review-shelf": review_reply });
    let frames = json!([describe_frame, review_frame, list_frame]);
    let replies = vec![
        serde_json::from_str(&describe_reply).unwrap(),
        serde_json::from_str(&review_reply).unwrap(),
        serde_json::from_str(&list_reply).unwrap(),
    ];
    (
        format!(
            "const RUST_HTTP = {rust_http};\nconst RUST_REPLIES = {rust_replies};\nconst FRAMES = {frames};"
        ),
        replies,
    )
}

fn run() -> Option<(Value, Vec<Value>)> {
    let Some(modules) = node_modules(REQUIRED_PACKAGES) else {
        stand_down_modules(
            REQUIRED_PACKAGES,
            "the emitted TypeScript clients and servers of the shelf service",
        );
        return None;
    };
    let (declared, replies) = preamble();
    let source = format!("{}\n\n{declared}\n{DRIVER}", bundle());
    let wrote = ran_with_modules(
        "mcp-wire",
        RUNTIME_VAR,
        "node",
        "mcp-wire.mts",
        &source,
        &modules,
    )?;
    Some((serde_json::from_str(wrote.trim()).unwrap(), replies))
}

#[test]
fn the_typescript_clients_read_and_the_servers_write_the_block_form_the_rust_dispatchers_write() {
    let Some((report, replies)) = run() else {
        return;
    };
    let blocks: Value = serde_json::from_str(BLOCKS).unwrap();
    let messages: Value = serde_json::from_str(MESSAGES).unwrap();
    assert_eq!(report["written"]["describe"], BLOCKS);
    assert_eq!(report["written"]["review"], MESSAGES);
    let page: Value = serde_json::from_str(PAGE).unwrap();
    assert_eq!(
        serde_json::from_str::<Value>(report["written"]["list"].as_str().unwrap()).unwrap(),
        page
    );
    for named in ["http", "rust", "bus", "socket", "rustSocket"] {
        let read = &report["read"][named];
        assert_eq!(
            read["describe"],
            json!({ "ok": true, "value": blocks }),
            "{named}"
        );
        assert_eq!(
            read["review"],
            json!({ "ok": true, "value": messages }),
            "{named}"
        );
    }
    for named in ["http", "bus", "socket"] {
        assert_eq!(
            report["read"][named]["missing"],
            json!({ "ok": false, "error": { "errorCode": "not-found" } }),
            "{named}"
        );
    }
    let written = report["replies"].as_array().unwrap();
    assert_eq!(written.len(), replies.len());
    for (got, wanted) in written.iter().zip(&replies) {
        assert_eq!(got, wanted);
    }
}

#[test]
fn the_typescript_text_of_the_shelf_service_type_checks() {
    let Some(modules) = node_modules(REQUIRED_PACKAGES) else {
        stand_down_modules(
            REQUIRED_PACKAGES,
            "the type-check of the shelf service's text",
        );
        return;
    };
    let text = bundle();
    let Some((succeeded, said)) = compiled_with_modules(
        "mcp-wire-types",
        COMPILER_VAR,
        &[("bundle.ts", &text), ("usage.ts", USAGE)],
        &modules,
    ) else {
        return;
    };
    assert!(succeeded, "the emitted text does not type-check:\n{said}");
}
