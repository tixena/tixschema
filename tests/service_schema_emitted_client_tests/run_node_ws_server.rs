//! The emitted `ws_rpc` server run by node against the `ws` package, driven by Node's built-in
//! `WebSocket` and by the real emitted client — the eight scenarios the design proved by hand.
//!
//! Beside `node` itself, this leg reaches for the `ws` and `zod` packages through
//! `TIXSCHEMA_NODE_MODULES`, standing down and naming that variable when either is missing. `just
//! test-emitted` resolves it up front and refuses to stand down.

use super::runtime::{node_modules, ran_with_modules, stand_down_modules};
use super::tests::{
    ConversationClientServiceSchema, ConversationId, WatchClientServiceSchema, WatchError,
    WatchRequest, WindowError, WindowPage,
};

/// Names the runtime to run, for a machine that has one somewhere other than `PATH`.
const RUNTIME_VAR: &str = "TIXSCHEMA_NODE";

/// The packages this leg's driver imports at runtime: `ws` for the listener, `zod` for the schemas
/// the emitted transport checks a reply against.
const REQUIRED_PACKAGES: &[&str] = &["ws", "zod"];

/// `onceEmitter` waits on a Node `EventEmitter` (the `ws` server and its raw sockets); `onceEvent`
/// waits on the DOM-shaped `addEventListener` seam (Node's built-in client `WebSocket`); `until`
/// polls a predicate rather than sleeping a guessed duration.
const HELPERS: &str = "
function onceEmitter(emitter, event) {
  return new Promise((resolve) => emitter.once(event, resolve));
}
function onceEvent(target, type) {
  return new Promise((resolve) => target.addEventListener(type, resolve, { once: true }));
}
function sleep(ms) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}
async function until(predicate, timeoutMs = 2000) {
  const start = Date.now();
  while (!predicate() && Date.now() - start < timeoutMs) {
    await sleep(5);
  }
  return predicate();
}
";

/// `window` answers the two-field page every scenario reads off; a `conversationId` of
/// `"slow-4"` is scenario 4's own hook to hold the handler open past the client's close, so it can
/// observe the connection's cancellation signal directly rather than racing a fixed sleep against it.
const IMPL_AND_MAKE_SERVER: &str = r#"
let lastHandlerSawAborted;
let handlerReturned;
const handlerDone = new Promise((resolve) => { handlerReturned = resolve; });
const impl = {
  async purgeConversation() {},
  async window(ctx, req) {
    if (req.conversationId === "slow-4") {
      await until(() => ctx.signal.aborted, 2000);
      lastHandlerSawAborted = ctx.signal.aborted;
      handlerReturned();
      return { ok: true, value: { items: ["slow"] } };
    }
    return { ok: true, value: { items: [req.conversationId, "connection " + ctx.n] } };
  },
};

function makeServer(overrides = {}) {
  let n = 0;
  return createConversationClientServiceWsServer(
    impl,
    (connection) => ({ n: ++n, signal: connection.signal }),
    { onFault: () => {}, ...overrides },
  );
}
"#;

/// Scenario 1: the real emitted client calls `window()` and `purgeConversation()` over the real
/// emitted transport, and the connection is counted while open and dropped once the client closes.
const SCENARIO_1_DRIVER: &str = r#"
async function main() {
  const server = makeServer();
  const wss = new WebSocketServer({ port: 0 });
  wss.on("connection", (socket) => server.accept(socket));
  await onceEmitter(wss, "listening");
  const port = wss.address().port;

  const rawSocket = new WebSocket(`ws://127.0.0.1:${port}`);
  await onceEvent(rawSocket, "open");
  const transport = createConversationClientServiceWsTransport(rawSocket);
  const client = createConversationClientServiceClient(transport);
  const windowResult = await client.window({ conversationId: "abc" });
  await client.purgeConversation("abc");
  const connectionsWhileOpen = server.connections().length;
  rawSocket.close();
  await onceEvent(rawSocket, "close");
  await until(() => server.connections().length === 0);
  const connectionsAfterClose = server.connections().length;
  wss.close();
  console.log(JSON.stringify({ window: windowResult, connectionsWhileOpen, connectionsAfterClose }));
  process.exit(0);
}
main().catch((error) => { console.error(error); process.exit(1); });
"#;

/// Scenario 2: a bare `{"kind":"ping"}` frame draws exactly one `{"kind":"pong"}`.
const SCENARIO_2_DRIVER: &str = r#"
async function main() {
  const server = makeServer();
  const wss = new WebSocketServer({ port: 0 });
  wss.on("connection", (socket) => server.accept(socket));
  await onceEmitter(wss, "listening");
  const port = wss.address().port;

  const rawSocket = new WebSocket(`ws://127.0.0.1:${port}`);
  await onceEvent(rawSocket, "open");
  const received = [];
  rawSocket.addEventListener("message", (event) => received.push(String(event.data)));
  rawSocket.send(JSON.stringify({ kind: "ping" }));
  await sleep(50);
  rawSocket.close();
  await onceEvent(rawSocket, "close");
  wss.close();
  console.log(JSON.stringify({ received }));
  process.exit(0);
}
main().catch((error) => { console.error(error); process.exit(1); });
"#;

/// Scenario 3: a peer sending nothing is pinged once at the idle interval and closed after the
/// pong timeout; a peer sending a frame every 50 ms for 650 ms is never probed.
const SCENARIO_3_DRIVER: &str = r#"
async function main() {
  const server = makeServer({ heartbeat: { intervalMs: 150, timeoutMs: 100 } });
  const wss = new WebSocketServer({ port: 0 });
  wss.on("connection", (socket) => server.accept(socket));
  await onceEmitter(wss, "listening");
  const port = wss.address().port;

  const silentStart = Date.now();
  const silent = new WebSocket(`ws://127.0.0.1:${port}`);
  await onceEvent(silent, "open");
  let pingAtMs = null;
  silent.addEventListener("message", (event) => {
    const frame = JSON.parse(String(event.data));
    if (frame.kind === "ping" && pingAtMs === null) pingAtMs = Date.now() - silentStart;
  });
  const silentClosed = onceEvent(silent, "close");

  const busy = new WebSocket(`ws://127.0.0.1:${port}`);
  await onceEvent(busy, "open");
  let busyPingsReceived = 0;
  busy.addEventListener("message", (event) => {
    const frame = JSON.parse(String(event.data));
    if (frame.kind === "ping") busyPingsReceived += 1;
  });
  const busyLoop = (async () => {
    const stopAt = Date.now() + 650;
    while (Date.now() < stopAt) {
      if (busy.readyState === WebSocket.OPEN) busy.send(JSON.stringify({ kind: "noop" }));
      await sleep(50);
    }
  })();

  await silentClosed;
  const silentClosedAtMs = Date.now() - silentStart;
  await busyLoop;
  const busyStillOpen = busy.readyState === WebSocket.OPEN;
  busy.close();
  await onceEvent(busy, "close");
  wss.close();
  console.log(JSON.stringify({ pingAtMs, silentClosedAtMs, busyPingsReceived, busyStillOpen }));
  process.exit(0);
}
main().catch((error) => { console.error(error); process.exit(1); });
"#;

/// Scenario 4: a client closing 100 ms into an in-flight handler leaves the connection's signal
/// aborted, the handler observes it, and the raw socket's `send` is never called again afterward.
const SCENARIO_4_DRIVER: &str = r#"
async function main() {
  let capturedConnection;
  let sendCalls = 0;
  const server = makeServer({ heartbeat: { intervalMs: 150, timeoutMs: 100 } });
  const wss = new WebSocketServer({ port: 0 });
  wss.on("connection", (socket) => {
    const rawSend = socket.send.bind(socket);
    socket.send = (data) => { sendCalls += 1; rawSend(data); };
    capturedConnection = server.accept(socket);
  });
  await onceEmitter(wss, "listening");
  const port = wss.address().port;

  const client = new WebSocket(`ws://127.0.0.1:${port}`);
  await onceEvent(client, "open");
  client.send(JSON.stringify({
    kind: "request", id: "1", service: "ConversationClientService", operation: "window",
    payload: { conversationId: "slow-4" },
  }));
  await sleep(100);
  client.close();
  await onceEvent(client, "close");
  await until(() => capturedConnection.signal.aborted);
  const connectionAborted = capturedConnection.signal.aborted;
  const sendCallsAtAbort = sendCalls;
  await handlerDone;
  await sleep(20);
  const sendCallsAfterHandler = sendCalls;
  wss.close();
  console.log(JSON.stringify({
    connectionAborted,
    handlerSawAborted: lastHandlerSawAborted,
    sendCallsAfterAbort: sendCallsAfterHandler - sendCallsAtAbort,
  }));
  process.exit(0);
}
main().catch((error) => { console.error(error); process.exit(1); });
"#;

/// Scenario 5: two accepted connections are counted, and `closeAll()` empties the set and closes
/// both client sockets.
const SCENARIO_5_DRIVER: &str = r#"
async function main() {
  const server = makeServer();
  const wss = new WebSocketServer({ port: 0 });
  wss.on("connection", (socket) => server.accept(socket));
  await onceEmitter(wss, "listening");
  const port = wss.address().port;

  const a = new WebSocket(`ws://127.0.0.1:${port}`);
  const b = new WebSocket(`ws://127.0.0.1:${port}`);
  await Promise.all([onceEvent(a, "open"), onceEvent(b, "open")]);
  await until(() => server.connections().length === 2);
  const connectionsBefore = server.connections().length;
  const aClosed = onceEvent(a, "close");
  const bClosed = onceEvent(b, "close");
  server.closeAll();
  await Promise.all([aClosed, bClosed]);
  await until(() => server.connections().length === 0);
  const connectionsAfter = server.connections().length;
  wss.close();
  console.log(JSON.stringify({ connectionsBefore, connectionsAfter }));
  process.exit(0);
}
main().catch((error) => { console.error(error); process.exit(1); });
"#;

/// Scenario 6: an `error` event emitted on an accepted `ws` socket does not throw, is reported
/// once through `onSocketError`, and drops the connection.
const SCENARIO_6_DRIVER: &str = r#"
async function main() {
  let capturedSocket;
  let onSocketErrorCalls = 0;
  const server = makeServer({ onSocketError: () => { onSocketErrorCalls += 1; } });
  const wss = new WebSocketServer({ port: 0 });
  wss.on("connection", (socket) => {
    capturedSocket = socket;
    server.accept(socket);
  });
  await onceEmitter(wss, "listening");
  const port = wss.address().port;

  const client = new WebSocket(`ws://127.0.0.1:${port}`);
  await onceEvent(client, "open");
  await until(() => server.connections().length === 1);

  let threw = false;
  const clientClosed = onceEvent(client, "close");
  try {
    capturedSocket.emit("error", new Error("simulated"));
  } catch {
    threw = true;
  }
  await clientClosed;
  await until(() => server.connections().length === 0);
  const connectionsAfter = server.connections().length;
  wss.close();
  console.log(JSON.stringify({ threw, onSocketErrorCalls, connectionsAfter }));
  process.exit(0);
}
main().catch((error) => { console.error(error); process.exit(1); });
"#;

/// Scenario 7: the per-connection cost, measured with in-memory stand-in sockets rather than real
/// `ws` connections. Heap growth is printed on the Rust side, never asserted — the design's own
/// figures are a single unpinned run, not a budget.
const SCENARIO_7_DRIVER: &str = r#"
function stubSocket() {
  const listeners = {};
  const on = (type) => (listeners[type] ??= []);
  return {
    send() {},
    close() {
      for (const listener of on("close").slice()) listener();
    },
    addEventListener(type, listener) { on(type).push(listener); },
    removeEventListener(type, listener) {
      listeners[type] = on(type).filter((registered) => registered !== listener);
    },
  };
}

function main() {
  const server = makeServer();
  const before = process.memoryUsage().heapUsed;
  const acceptStart = Date.now();
  for (let i = 0; i < 10000; i += 1) {
    server.accept(stubSocket());
  }
  const acceptMs = Date.now() - acceptStart;
  const after = process.memoryUsage().heapUsed;
  const connectionsAfterAccept = server.connections().length;
  const closeStart = Date.now();
  server.closeAll();
  const closeMs = Date.now() - closeStart;
  const connectionsAfterClose = server.connections().length;
  console.log(JSON.stringify({
    connectionsAfterAccept, connectionsAfterClose, acceptMs, closeMs, heapGrowthBytes: after - before,
  }));
  process.exit(0);
}
main();
"#;

/// Scenario 8: a second service reached through `connection.share()` answers only its own frames,
/// one `pong` answers one `ping`, a third service's frame is answered by nobody, closing the
/// connection detaches the shared attachment, and `share` after close throws.
const SHARE_DRIVER: &str = r#"
let sharedDetached = false;
function attachInventoryServiceStub(socket) {
  const onMessage = (event) => {
    let frame;
    try { frame = JSON.parse(String(event.data)); } catch { return; }
    if (typeof frame !== "object" || frame === null || frame.service !== "InventoryService") return;
    if (frame.kind === "request") {
      socket.send(JSON.stringify({
        kind: "reply", id: frame.id, service: "InventoryService", ok: true, value: { counted: 7 },
      }));
    }
  };
  socket.addEventListener("message", onMessage);
  return () => {
    sharedDetached = true;
    socket.removeEventListener("message", onMessage);
  };
}

async function main() {
  const server = makeServer();
  const wss = new WebSocketServer({ port: 0 });
  let connection;
  wss.on("connection", (socket) => {
    connection = server.accept(socket);
    connection.share(attachInventoryServiceStub);
  });
  await onceEmitter(wss, "listening");
  const port = wss.address().port;

  const client = new WebSocket(`ws://127.0.0.1:${port}`);
  await onceEvent(client, "open");
  const received = [];
  client.addEventListener("message", (event) => received.push(String(event.data)));

  client.send(JSON.stringify({
    kind: "request", id: "2", service: "InventoryService", operation: "count", payload: {},
  }));
  client.send(JSON.stringify({ kind: "ping" }));
  client.send(JSON.stringify({
    kind: "request", id: "1", service: "ConversationClientService", operation: "window",
    payload: { conversationId: "abc" },
  }));
  client.send(JSON.stringify({
    kind: "request", id: "3", service: "UnknownService", operation: "noop", payload: {},
  }));

  await until(() => received.length >= 3);
  await sleep(50);
  const frames = received.map((raw) => JSON.parse(raw));
  const pongCount = frames.filter((frame) => frame.kind === "pong").length;
  const thirdServiceAnswers = frames.filter((frame) => frame.service === "UnknownService").length;

  const clientClosed = onceEvent(client, "close");
  connection.close();
  await clientClosed;
  await until(() => server.connections().length === 0);
  const connectionsAfterClose = server.connections().length;

  let shareAfterCloseThrew = false;
  let shareAfterCloseMessage = "";
  try {
    connection.share(attachInventoryServiceStub);
  } catch (error) {
    shareAfterCloseThrew = true;
    shareAfterCloseMessage = String(error.message);
  }

  wss.close();
  console.log(JSON.stringify({
    frames, pongCount, thirdServiceAnswers, connectionsAfterClose, sharedDetached,
    shareAfterCloseThrew, shareAfterCloseMessage,
  }));
  process.exit(0);
}
main().catch((error) => { console.error(error); process.exit(1); });
"#;

/// A hand-written peer that answers every `request` frame with `"value": null`.
const UNIT_SUCCESS_DRIVER: &str = r#"
async function main() {
  const wss = new WebSocketServer({ port: 0 });
  wss.on("connection", (socket) => {
    socket.on("message", (data) => {
      const frame = JSON.parse(String(data));
      if (frame.kind !== "request") return;
      socket.send(JSON.stringify({ kind: "reply", id: frame.id, service: frame.service, ok: true, value: null }));
    });
  });
  await onceEmitter(wss, "listening");
  const port = wss.address().port;

  const rawSocket = new WebSocket(`ws://127.0.0.1:${port}`);
  await onceEvent(rawSocket, "open");
  const transport = createWatchClientServiceWsTransport(rawSocket);
  const client = createWatchClientServiceClient(transport);
  const result = await client.watch({ topic: "x" });
  rawSocket.close();
  wss.close();
  console.log(JSON.stringify({
    ok: result.ok,
    hasValueKey: "value" in result,
    valueIsUndefined: result.value === undefined,
  }));
  process.exit(0);
}
main().catch((error) => { console.error(error); process.exit(1); });
"#;

/// The service's own emitted surface: every message, the service's types, the implementable
/// interface, the client, and the three `ws_rpc` artifacts a connection-accepting server needs.
fn emitted() -> String {
    [
        "import { z } from \"zod\";".to_owned(),
        "import { WebSocketServer } from \"ws\";".to_owned(),
        WindowPage::ts_definition(),
        WindowPage::zod_schema(),
        WindowError::ts_definition(),
        WindowError::zod_schema(),
        ConversationId::ts_definition(),
        ConversationId::zod_schema(),
        ConversationClientServiceSchema::ts_definition(),
        ConversationClientServiceSchema::ts_service(),
        ConversationClientServiceSchema::ts_client(),
        ConversationClientServiceSchema::ts_ws_client(),
        ConversationClientServiceSchema::ts_ws_service(),
        ConversationClientServiceSchema::ts_ws_server(),
    ]
    .join("\n\n")
}

/// Runs one scenario's driver against `server.mts`, or stands down (naming
/// `TIXSCHEMA_NODE_MODULES`) when `ws` or `zod` is not reachable through it.
fn run_scenario(named: &str, driver: &str) -> Option<serde_json::Value> {
    let Some(modules) = node_modules(REQUIRED_PACKAGES) else {
        stand_down_modules(REQUIRED_PACKAGES, "the emitted WebSocket server");
        return None;
    };
    let module = format!(
        "{}\n\n{HELPERS}\n\n{IMPL_AND_MAKE_SERVER}\n\n{driver}",
        emitted()
    );
    let wrote = ran_with_modules(named, RUNTIME_VAR, "node", "server.mts", &module, &modules)?;
    Some(serde_json::from_str(wrote.trim()).unwrap())
}

/// Scenario 8 alone, in its own entry — the design ran it in a second process, and it needs a
/// clean connection set.
fn run_share_scenario() -> Option<serde_json::Value> {
    let Some(modules) = node_modules(REQUIRED_PACKAGES) else {
        stand_down_modules(REQUIRED_PACKAGES, "the emitted WebSocket server");
        return None;
    };
    let module = format!(
        "{}\n\n{HELPERS}\n\n{IMPL_AND_MAKE_SERVER}\n\n{SHARE_DRIVER}",
        emitted()
    );
    let wrote = ran_with_modules(
        "ws-server-share",
        RUNTIME_VAR,
        "node",
        "share.mts",
        &module,
        &modules,
    )?;
    Some(serde_json::from_str(wrote.trim()).unwrap())
}

/// `WatchClientService`'s message, client, and `ws_rpc` client transport -- no server.
fn emitted_unit_success() -> String {
    [
        "import { z } from \"zod\";".to_owned(),
        "import { WebSocketServer } from \"ws\";".to_owned(),
        WatchRequest::ts_definition(),
        WatchRequest::zod_schema(),
        WatchError::ts_definition(),
        WatchError::zod_schema(),
        WatchClientServiceSchema::ts_definition(),
        WatchClientServiceSchema::ts_client(),
        WatchClientServiceSchema::ts_ws_client(),
    ]
    .join("\n\n")
}

fn run_unit_success_scenario() -> Option<serde_json::Value> {
    let Some(modules) = node_modules(REQUIRED_PACKAGES) else {
        stand_down_modules(REQUIRED_PACKAGES, "the emitted WebSocket client");
        return None;
    };
    let module = format!(
        "{}\n\n{HELPERS}\n\n{UNIT_SUCCESS_DRIVER}",
        emitted_unit_success()
    );
    let wrote = ran_with_modules(
        "ws-unit-success",
        RUNTIME_VAR,
        "node",
        "unit-success.mts",
        &module,
        &modules,
    )?;
    Some(serde_json::from_str(wrote.trim()).unwrap())
}

#[test]
fn the_emitted_client_is_served_and_the_connection_is_counted() {
    let Some(result) = run_scenario("ws-server-served", SCENARIO_1_DRIVER) else {
        return;
    };
    assert_eq!(
        result["window"],
        serde_json::json!({"ok": true, "value": {"items": ["abc", "connection 1"]}}),
        "got: {result:#?}"
    );
    assert_eq!(result["connectionsWhileOpen"], 1_i32, "got: {result:#?}");
    assert_eq!(result["connectionsAfterClose"], 0_i32, "got: {result:#?}");
}

#[test]
fn a_ping_frame_draws_exactly_one_pong() {
    let Some(result) = run_scenario("ws-server-ping", SCENARIO_2_DRIVER) else {
        return;
    };
    assert_eq!(
        result["received"],
        serde_json::json!(["{\"kind\":\"pong\"}"]),
        "got: {result:#?}"
    );
}

#[test]
fn a_silent_peer_is_probed_and_closed_and_a_busy_one_never_is() {
    let Some(result) = run_scenario("ws-server-heartbeat", SCENARIO_3_DRIVER) else {
        return;
    };
    let ping_ms = result["pingAtMs"].as_i64();
    assert!(ping_ms.is_some(), "no ping arrived. got: {result:#?}");
    let ping_at = ping_ms.unwrap();
    assert!(
        (140..=300).contains(&ping_at),
        "pinged once at ~150 ms. got: {result:#?}"
    );
    let closed_at = result["silentClosedAtMs"].as_i64().unwrap();
    assert!(
        (250..=400).contains(&closed_at),
        "closed by the server between 250 and 400 ms. got: {result:#?}"
    );
    assert_eq!(result["busyPingsReceived"], 0_i32, "got: {result:#?}");
    assert_eq!(result["busyStillOpen"], true, "got: {result:#?}");
}

#[test]
fn an_in_flight_call_observes_cancellation_and_writes_nothing_after_close() {
    let Some(result) = run_scenario("ws-server-cancel", SCENARIO_4_DRIVER) else {
        return;
    };
    assert_eq!(result["connectionAborted"], true, "got: {result:#?}");
    assert_eq!(result["handlerSawAborted"], true, "got: {result:#?}");
    assert_eq!(result["sendCallsAfterAbort"], 0_i32, "got: {result:#?}");
}

#[test]
fn close_all_empties_the_connection_set() {
    let Some(result) = run_scenario("ws-server-closeall", SCENARIO_5_DRIVER) else {
        return;
    };
    assert_eq!(result["connectionsBefore"], 2_i32, "got: {result:#?}");
    assert_eq!(result["connectionsAfter"], 0_i32, "got: {result:#?}");
}

#[test]
fn a_socket_error_is_reported_and_closes_the_connection() {
    let Some(result) = run_scenario("ws-server-error", SCENARIO_6_DRIVER) else {
        return;
    };
    assert_eq!(result["threw"], false, "got: {result:#?}");
    assert_eq!(result["onSocketErrorCalls"], 1_i32, "got: {result:#?}");
    assert_eq!(result["connectionsAfter"], 0_i32, "got: {result:#?}");
}

#[test]
fn ten_thousand_connections_cost_what_the_design_measured() {
    let Some(result) = run_scenario("ws-server-cost", SCENARIO_7_DRIVER) else {
        return;
    };
    assert_eq!(
        result["connectionsAfterAccept"], 10_000_i32,
        "got: {result:#?}"
    );
    assert_eq!(result["connectionsAfterClose"], 0_i32, "got: {result:#?}");
    eprintln!(
        "tixschema: 10,000 stand-in connections accepted in {} ms, closed in {} ms, heap grew by \
         {} bytes",
        result["acceptMs"], result["closeMs"], result["heapGrowthBytes"]
    );
}

#[test]
fn two_services_share_one_socket_through_the_connection() {
    let Some(result) = run_share_scenario() else {
        return;
    };
    let frames = result["frames"].as_array().unwrap();
    let inventory_reply = frames
        .iter()
        .find(|frame| frame["service"] == "InventoryService");
    assert!(
        inventory_reply.is_some(),
        "the shared stand-in never answered. got: {result:#?}"
    );
    assert_eq!(
        *inventory_reply.unwrap(),
        serde_json::json!({
            "kind": "reply", "id": "2", "service": "InventoryService", "ok": true,
            "value": {"counted": 7_i32},
        }),
        "got: {result:#?}"
    );
    let conversation_reply = frames
        .iter()
        .find(|frame| frame["service"] == "ConversationClientService");
    assert!(
        conversation_reply.is_some(),
        "the owning service never answered. got: {result:#?}"
    );
    assert_eq!(
        *conversation_reply.unwrap(),
        serde_json::json!({
            "kind": "reply", "id": "1", "service": "ConversationClientService", "ok": true,
            "value": {"items": ["abc", "connection 1"]},
        }),
        "got: {result:#?}"
    );
    assert_eq!(result["pongCount"], 1_i32, "got: {result:#?}");
    assert_eq!(result["thirdServiceAnswers"], 0_i32, "got: {result:#?}");
    assert_eq!(result["connectionsAfterClose"], 0_i32, "got: {result:#?}");
    assert_eq!(result["sharedDetached"], true, "got: {result:#?}");
    assert_eq!(result["shareAfterCloseThrew"], true, "got: {result:#?}");
    assert_eq!(
        result["shareAfterCloseMessage"], "the connection is closed",
        "got: {result:#?}"
    );
}

/// A `"value": null` reply normalizes to `value: undefined`.
#[test]
fn a_rust_shaped_unit_success_reply_normalizes_to_value_undefined() {
    let Some(result) = run_unit_success_scenario() else {
        return;
    };
    assert_eq!(
        result,
        serde_json::json!({ "ok": true, "hasValueKey": true, "valueIsUndefined": true }),
        "got: {result:#?}"
    );
}
