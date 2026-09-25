//! Headers both ways, run by node against the Rust `ws_rpc` twins of `StampClientService`: the
//! emitted TypeScript client writes the frames the Rust client writes and reads the replies the
//! Rust dispatcher writes, the emitted attachment answers the Rust client's own frames the way the
//! Rust dispatcher does, and the generic client reads a stub AMQP transport's headers as the Rust
//! transport encodes them.
//!
//! Beside `node` itself, this leg reaches for the `zod` package through `TIXSCHEMA_NODE_MODULES`,
//! standing down and naming that variable when it is missing. `just test-emitted` resolves it up
//! front and refuses to stand down.

use super::runtime::{node_modules, ran_with_modules, stand_down_modules};
use super::stamp_client_service_schema::CallError;
use super::stamp_ws_rpc_client::{FrameSession, FrameWriter, StampClientServiceClient, ping_frame};
use super::stamp_ws_rpc_transport;
use super::tests::{StampBackEnd, StampClientServiceSchema, StampError, StampReceipt};
use alloc::sync::Arc;
use core::future::{Future, Ready, ready};
use core::pin::{Pin, pin};
use core::task::{Context as PollContext, Poll, Waker};
use serde_json::{Value, json};
use std::sync::Mutex;

const RUNTIME_VAR: &str = "TIXSCHEMA_NODE";

const REQUIRED_PACKAGES: &[&str] = &["zod"];

/// A socket the emitted transport and attachment both bind to: `send` hands every written frame to
/// the driver, `deliver` stands in for a frame the far side wrote.
const HELPERS: &str = r#"
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
function described(result) {
  if (result.ok) return { ok: true, value: result.value, ageIsNull: result.value[2] === null };
  if ("isServiceFault" in result.error) return { ok: false, fault: result.error.fault };
  return { ok: false, error: result.error, reasonIsNull: result.error[1] === null };
}
const CALLS = [["a", "acme", 7], ["b", "acme", undefined], ["refuse", "acme", 3], ["refuse", "acme", undefined]];
"#;

/// Every call the client groups make, in order: the four `stamp` calls of `CALLS`, then `mark`;
/// and the first heartbeat probe a second transport writes.
const WRITES_FRAMES_DRIVER: &str = r#"
async function main() {
  const sent = [];
  const socket = fakeSocket((text) => sent.push(text));
  const client = createStampClientServiceClient(
    createStampClientServiceWsTransport(socket, { heartbeat: false }),
  );
  for (const [label, tenant, trace] of CALLS) void client.stamp(label, tenant, trace);
  await client.mark("m", "acme");
  const probes = [];
  const probing = createStampClientServiceWsTransport(
    fakeSocket((text) => probes.push(text)),
    { heartbeat: { intervalMs: 1, timeoutMs: 1000 } },
  );
  await until(() => probes.length > 0);
  probing.close();
  console.log(JSON.stringify({ sent, ping: probes[0] }));
  process.exit(0);
}
main().catch((error) => { console.error(error); process.exit(1); });
"#;

/// `REPLIES`, keyed by frame id, is written above this by the Rust side.
const READS_REPLIES_DRIVER: &str = "
async function main() {
  const socket = fakeSocket((text) => {
    const { id } = JSON.parse(text);
    if (id !== undefined && REPLIES[id] !== undefined) queueMicrotask(() => socket.deliver(REPLIES[id]));
  });
  const client = createStampClientServiceClient(
    createStampClientServiceWsTransport(socket, { heartbeat: false }),
  );
  const results = [];
  for (const [label, tenant, trace] of CALLS) results.push(described(await client.stamp(label, tenant, trace)));
  console.log(JSON.stringify(results));
  process.exit(0);
}
main().catch((error) => { console.error(error); process.exit(1); });
";

/// `FRAMES` is written above this by the Rust side. The implementation answers exactly as
/// `StampBackEnd` does.
const ANSWERS_FRAMES_DRIVER: &str = r#"
async function main() {
  const written = [];
  const marked = [];
  const faults = [];
  const socket = fakeSocket((text) => written.push(text));
  attachStampClientServiceWsDispatcher(socket, {}, {
    async stamp(ctx, label, tenant, trace) {
      if (label === "refuse") {
        return { ok: false, error: [{ errorCode: "refused" }, trace === undefined ? null : `refused at ${trace}`] };
      }
      return { ok: true, value: [{ label, tenant }, `etag-${tenant}`, trace === undefined ? null : trace + 1] };
    },
    async mark(ctx, label, tenant) {
      marked.push(`${label} ${tenant}`);
    },
  }, (fault) => faults.push(fault));
  for (const frame of FRAMES) socket.deliver(frame);
  await until(() => written.length === EXPECTED_REPLIES && marked.length === 1 && faults.length === 1);
  console.log(JSON.stringify({ written, marked, faults }));
  process.exit(0);
}
main().catch((error) => { console.error(error); process.exit(1); });
"#;

/// `ANSWERS`, keyed by label, is written above this by the Rust side: what an AMQP transport hands
/// back, the envelope beside the reply's headers each JSON-encoded.
const AMQP_STUB_DRIVER: &str = r#"
async function main() {
  const carried = [];
  const client = createStampClientServiceClient({
    async notify(operation, payload, headers) {
      carried.push({ operation, payload, headers });
    },
    async request(operation, payload, headers) {
      carried.push({ operation, payload, headers });
      return ANSWERS[payload];
    },
  });
  const results = {};
  for (const [label, tenant, trace] of [...CALLS, ["noetag", "acme", undefined], ["badage", "acme", undefined], ["wrongage", "acme", undefined]]) {
    results[`${label}:${trace}`] = described(await client.stamp(label, tenant, trace));
  }
  await client.mark("m", "acme");
  console.log(JSON.stringify({ carried, results }));
  process.exit(0);
}
main().catch((error) => { console.error(error); process.exit(1); });
"#;

/// The service's own emitted surface: its messages, the client and dispatcher, and both `ws_rpc`
/// artifacts.
fn emitted() -> String {
    [
        "import { z } from \"zod\";".to_owned(),
        StampReceipt::ts_definition(),
        StampReceipt::zod_schema(),
        StampError::ts_definition(),
        StampError::zod_schema(),
        StampClientServiceSchema::ts_definition(),
        StampClientServiceSchema::ts_service(),
        StampClientServiceSchema::ts_client(),
        StampClientServiceSchema::ts_ws_client(),
        StampClientServiceSchema::ts_ws_service(),
    ]
    .join("\n\n")
}

/// Runs `driver` beside the emitted surface, with `preamble` declaring whatever the Rust side
/// computed for it, and reads the one JSON line it prints — or stands down, naming
/// `TIXSCHEMA_NODE_MODULES`, where `zod` is not reachable.
fn run(named: &str, preamble: &str, driver: &str) -> Option<Value> {
    let Some(modules) = node_modules(REQUIRED_PACKAGES) else {
        stand_down_modules(REQUIRED_PACKAGES, "the emitted ws_rpc headers");
        return None;
    };
    let module = format!("{}\n\n{HELPERS}\n\n{preamble}\n\n{driver}", emitted());
    let wrote = ran_with_modules(named, RUNTIME_VAR, "node", "headers.mts", &module, &modules)?;
    Some(serde_json::from_str(wrote.trim()).unwrap())
}

fn poll_by_hand<Answered>(pinned: Pin<&mut Answered>) -> Poll<Answered::Output>
where
    Answered: Future,
{
    pinned.poll(&mut PollContext::from_waker(Waker::noop()))
}

/// The Rust dispatcher's answer to one frame. `StampBackEnd` never suspends, so one poll answers.
fn rust_answer(backend: &StampBackEnd, frame: &str) -> Option<String> {
    match poll_by_hand(pin!(stamp_ws_rpc_transport::answer(frame, backend, &())).as_mut()) {
        Poll::Ready(answered) => answered,
        Poll::Pending => None,
    }
}

/// A function that puts a text frame on the wire, recording it in `sent` instead.
fn recording(
    sent: &Arc<Mutex<Vec<String>>>,
) -> impl Fn(String) -> Ready<Result<(), String>> + Send + Sync + 'static {
    let mailbox = Arc::clone(sent);
    move |text| {
        mailbox.lock().unwrap().push(text);
        ready(Ok(()))
    }
}

/// A Rust client over a session whose every written frame is recorded.
fn rust_client() -> (
    StampClientServiceClient<FrameSession>,
    Arc<Mutex<Vec<String>>>,
) {
    let sent = Arc::new(Mutex::new(Vec::new()));
    let session = FrameSession::new(recording(&sent));
    (StampClientServiceClient::new(session), sent)
}

fn parsed(text: &str) -> Value {
    serde_json::from_str(text).unwrap()
}

/// A reply frame with its fault's `detail` taken out: the words are each side's own decoder's,
/// and everything else about the fault is the contract.
fn without_fault_detail(mut reply: Value) -> Value {
    if let Some(fault) = reply
        .pointer_mut("/error/fault")
        .and_then(Value::as_object_mut)
    {
        fault.remove("detail");
    }
    reply
}

/// The frames the TypeScript client writes for `CALLS` and `mark`, and its heartbeat probe.
fn typescript_client_frames() -> Option<(Vec<String>, String)> {
    run("ws-headers-writes", "", WRITES_FRAMES_DRIVER).map(|written| frames_of(&written))
}

/// What [`WRITES_FRAMES_DRIVER`] printed, read back: the frames it sent, and the probe.
fn frames_of(written: &Value) -> (Vec<String>, String) {
    let sent = written["sent"]
        .as_array()
        .unwrap()
        .iter()
        .map(|frame| frame.as_str().unwrap().to_owned())
        .collect();
    (sent, written["ping"].as_str().unwrap().to_owned())
}

#[test]
fn the_typescript_client_writes_the_frames_the_rust_client_writes() {
    let Some((written, ping)) = typescript_client_frames() else {
        return;
    };
    let (client, sent) = rust_client();
    let mut calls = [
        Box::pin(client.stamp("a".to_owned(), "acme".to_owned(), Some(7))),
        Box::pin(client.stamp("b".to_owned(), "acme".to_owned(), None)),
        Box::pin(client.stamp("refuse".to_owned(), "acme".to_owned(), Some(3))),
        Box::pin(client.stamp("refuse".to_owned(), "acme".to_owned(), None)),
    ];
    for call in &mut calls {
        assert!(poll_by_hand(call.as_mut()).is_pending());
    }
    assert!(poll_by_hand(pin!(client.mark("m".to_owned(), "acme".to_owned())).as_mut()).is_ready());
    let rust: Vec<Value> = sent
        .lock()
        .unwrap()
        .iter()
        .map(|frame| parsed(frame))
        .collect();
    let typescript: Vec<Value> = written.iter().map(|frame| parsed(frame)).collect();
    assert_eq!(typescript, rust);
    assert_eq!(
        typescript[0],
        json!({
            "kind": "request",
            "id": "1",
            "service": "StampClientService",
            "operation": "stamp",
            "payload": "a",
            "headers": { "x-tenant": "acme", "x-trace": 7_u32 },
        }),
        "each header value crosses as the JSON value it is"
    );
    assert_eq!(
        typescript[1].get("headers"),
        Some(&json!({ "x-tenant": "acme", "x-trace": null })),
        "an optional header holding nothing crosses as `null`, as the Rust client writes a `None`"
    );
    assert_eq!(
        ping,
        ping_frame(),
        "the heartbeat probe is the Rust client's own"
    );
    client.transport().close("the run is over");
}

#[test]
fn the_typescript_client_reads_the_replies_the_rust_dispatcher_writes() {
    let Some((frames, _ping)) = typescript_client_frames() else {
        return;
    };
    let backend = StampBackEnd::default();
    let mut replies = serde_json::Map::new();
    for frame in &frames {
        if let Some(reply) = rust_answer(&backend, frame) {
            let id = parsed(frame)["id"].as_str().unwrap().to_owned();
            replies.insert(id, Value::String(reply));
        }
    }
    assert_eq!(
        backend.marked(),
        vec!["m acme".to_owned()],
        "the Rust dispatcher read the one-way call's header off the TypeScript frame"
    );
    let Some(results) = run(
        "ws-headers-reads",
        &format!("const REPLIES = {};", Value::Object(replies)),
        READS_REPLIES_DRIVER,
    ) else {
        return;
    };
    assert_eq!(
        results,
        json!([
            {
                "ok": true,
                "value": [{ "label": "a", "tenant": "acme" }, "etag-acme", 8_u32],
                "ageIsNull": false,
            },
            {
                "ok": true,
                "value": [{ "label": "b", "tenant": "acme" }, "etag-acme", null],
                "ageIsNull": true,
            },
            {
                "ok": false,
                "error": [{ "errorCode": "refused" }, "refused at 3"],
                "reasonIsNull": false,
            },
            {
                "ok": false,
                "error": [{ "errorCode": "refused" }, null],
                "reasonIsNull": true,
            },
        ])
    );
}

#[test]
fn the_typescript_attachment_answers_the_rust_client_the_way_the_rust_dispatcher_does() {
    let (client, sent) = rust_client();
    let mut first = pin!(client.stamp("a".to_owned(), "acme".to_owned(), Some(7)));
    let mut second = pin!(client.stamp("b".to_owned(), "acme".to_owned(), None));
    let mut third = pin!(client.stamp("refuse".to_owned(), "acme".to_owned(), Some(3)));
    let mut fourth = pin!(client.stamp("refuse".to_owned(), "acme".to_owned(), None));
    assert!(poll_by_hand(first.as_mut()).is_pending());
    assert!(poll_by_hand(second.as_mut()).is_pending());
    assert!(poll_by_hand(third.as_mut()).is_pending());
    assert!(poll_by_hand(fourth.as_mut()).is_pending());
    // A one-way push goes out over a writer, the way a Rust server pushes to a peer it holds no
    // correlation map for.
    let pusher = StampClientServiceClient::new(FrameWriter::new(recording(&sent)));
    assert!(poll_by_hand(pin!(pusher.mark("m".to_owned(), "acme".to_owned())).as_mut()).is_ready());
    let mut frames = sent.lock().unwrap().clone();
    // Frames no Rust client writes, but a peer could: a request and a notify each missing the
    // required header.
    frames.push(
        r#"{"kind":"request","id":"99","service":"StampClientService","operation":"stamp","payload":"c"}"#
            .to_owned(),
    );
    frames.push(
        r#"{"kind":"notify","service":"StampClientService","operation":"mark","payload":"n"}"#
            .to_owned(),
    );
    let Some(answered) = run(
        "ws-headers-answers",
        &format!(
            "const FRAMES = {};\nconst EXPECTED_REPLIES = 5;",
            Value::from(frames.clone())
        ),
        ANSWERS_FRAMES_DRIVER,
    ) else {
        return;
    };
    assert_eq!(answered["marked"], json!(["m acme"]));
    assert_eq!(answered["faults"][0]["kind"], json!("failed-validation"));
    assert_eq!(answered["faults"][0]["field"], json!("x-tenant"));

    let backend = StampBackEnd::default();
    let written: Vec<String> = answered["written"]
        .as_array()
        .unwrap()
        .iter()
        .map(|reply| reply.as_str().unwrap().to_owned())
        .collect();
    for frame in frames.iter().filter(|frame| frame.contains("\"request\"")) {
        let id = parsed(frame)["id"].clone();
        let typescript = written
            .iter()
            .map(|reply| parsed(reply))
            .find(|reply| reply["id"] == id)
            .unwrap();
        let rust = parsed(&rust_answer(&backend, frame).unwrap());
        assert_eq!(
            without_fault_detail(typescript),
            without_fault_detail(rust),
            "the reply to {frame}"
        );
    }

    for reply in &written {
        assert!(matches!(
            poll_by_hand(pin!(client.transport().deliver(reply)).as_mut()),
            Poll::Ready(())
        ));
    }
    let receipt = |label: &str| StampReceipt {
        label: label.to_owned(),
        tenant: "acme".to_owned(),
    };
    assert_eq!(
        poll_by_hand(first.as_mut()),
        Poll::Ready(Ok((receipt("a"), "etag-acme".to_owned(), Some(8))))
    );
    assert_eq!(
        poll_by_hand(second.as_mut()),
        Poll::Ready(Ok((receipt("b"), "etag-acme".to_owned(), None)))
    );
    assert_eq!(
        poll_by_hand(third.as_mut()),
        Poll::Ready(Err(CallError::Operation((
            StampError::Refused,
            Some("refused at 3".to_owned())
        ))))
    );
    assert_eq!(
        poll_by_hand(fourth.as_mut()),
        Poll::Ready(Err(CallError::Operation((StampError::Refused, None))))
    );
}

/// What an AMQP transport hands the client for one reply: the envelope, and each header
/// JSON-encoded exactly as the Rust transport's `encoded_header` writes it.
fn amqp_answer(envelope: &Value, headers: &[(&str, Value)]) -> Value {
    let encoded: Vec<Value> = headers
        .iter()
        .map(|(name, value)| json!([name, serde_json::to_string(value).unwrap()]))
        .collect();
    json!({ "answered": envelope, "headers": encoded })
}

#[test]
fn the_generic_client_reads_a_stub_amqp_transport_s_headers() {
    let receipt = |label: &str| json!({ "label": label, "tenant": "acme" });
    let refused = json!({ "errorCode": "refused" });
    let answers = json!({
        "a": amqp_answer(
            &json!({ "ok": true, "value": receipt("a") }),
            &[("etag", json!("etag-acme")), ("x-age", json!(8_u32))],
        ),
        "b": amqp_answer(
            &json!({ "ok": true, "value": receipt("b") }),
            &[("etag", json!("etag-acme"))],
        ),
        "refuse": amqp_answer(
            &json!({ "ok": false, "error": refused }),
            &[("x-reason", json!("refused at 3"))],
        ),
        "noetag": amqp_answer(&json!({ "ok": true, "value": receipt("noetag") }), &[]),
        "badage": {
            "answered": { "ok": true, "value": receipt("badage") },
            "headers": [["etag", "\"etag-acme\""], ["x-age", "not json"]],
        },
        "wrongage": amqp_answer(
            &json!({ "ok": true, "value": receipt("wrongage") }),
            &[("etag", json!("etag-acme")), ("x-age", json!("eight"))],
        ),
    });
    let Some(ran) = run(
        "ws-headers-amqp",
        &format!("const ANSWERS = {answers};"),
        AMQP_STUB_DRIVER,
    ) else {
        return;
    };
    assert_eq!(
        ran["carried"][0],
        json!({
            "operation": "stamp",
            "payload": "a",
            "headers": [["x-tenant", "\"acme\""], ["x-trace", "7"]],
        }),
        "each header value is handed over JSON-encoded, as the AMQP headers table carries it"
    );
    assert_eq!(
        ran["carried"][1]["headers"],
        json!([["x-tenant", "\"acme\""], ["x-trace", "null"]])
    );
    assert_eq!(
        ran["carried"].as_array().unwrap().last().unwrap(),
        &json!({ "operation": "mark", "payload": "m", "headers": [["x-tenant", "\"acme\""]] })
    );
    let results = &ran["results"];
    assert_eq!(
        results["a:7"],
        json!({ "ok": true, "value": [receipt("a"), "etag-acme", 8_u32], "ageIsNull": false })
    );
    assert_eq!(
        results["b:undefined"],
        json!({ "ok": true, "value": [receipt("b"), "etag-acme", null], "ageIsNull": true })
    );
    assert_eq!(
        results["refuse:3"],
        json!({ "ok": false, "error": [refused, "refused at 3"], "reasonIsNull": false })
    );
    for (called, field) in [
        ("noetag:undefined", "etag"),
        ("badage:undefined", "x-age"),
        ("wrongage:undefined", "x-age"),
    ] {
        assert_eq!(
            results[called]["fault"]["kind"],
            json!("failed-validation"),
            "{called}: {results}"
        );
        assert_eq!(results[called]["fault"]["field"], json!(field), "{called}");
        assert_eq!(
            results[called]["fault"]["operation"],
            json!("stamp"),
            "{called}"
        );
    }
}
