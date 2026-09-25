//! Headers both ways, run by a real Swift toolchain against the Rust `ws_rpc` twin of
//! `StampClientService`: the emitted Swift client writes the frames the Rust client writes and
//! reads the replies the Rust dispatcher writes. Swift has no `ws_rpc` server or attachment, so
//! there is no dispatcher side to run here — see [`super::run_node_ws_headers`] for that half.
//!
//! Every scenario runs `swift main.swift` in immediate mode, standing down exactly as
//! [`super::run_swift`] does where no Swift toolchain is reachable.

#![cfg(feature = "swift")]

use super::runtime::ran;
use super::stamp_client_service_schema::{
    stamp_client_service_fault_fields_swift, stamp_client_service_fault_kind_swift,
};
use super::stamp_ws_rpc_client::{FrameSession, StampClientServiceClient};
use super::stamp_ws_rpc_transport;
use super::tests::{
    StampBackEnd, StampClientServiceSchema, stamp_error_swift, stamp_receipt_swift,
};
use alloc::sync::Arc;
use core::future::{Future, Ready, ready};
use core::pin::{Pin, pin};
use core::task::{Context as PollContext, Poll, Waker};
use serde_json::Value;
use std::sync::Mutex;

const RUNTIME_VAR: &str = "TIXSCHEMA_SWIFT";

/// A socket the emitted transport binds to, mirroring `run_swift.rs`'s own `InMemorySocket`.
const HELPERS: &str = "
final class RelaySocket: StampClientServiceWsSocket, @unchecked Sendable {
  private let lock = NSLock()
  private var _sent: [String] = []
  var onMessage: (@Sendable (String) -> Void)?
  var onClose: (@Sendable () -> Void)?

  var sent: [String] {
    lock.lock()
    defer { lock.unlock() }
    return _sent
  }

  func send(_ text: String) {
    lock.lock()
    _sent.append(text)
    lock.unlock()
  }

  func close() {
    onClose?()
  }

  func deliver(_ text: String) {
    onMessage?(text)
  }
}

struct FrameProbe: Decodable {
  let id: String?
}

func requestId(from sent: String?) -> String? {
  guard let sent, let data = sent.data(using: .utf8),
        let frame = try? JSONDecoder().decode(FrameProbe.self, from: data)
  else { return nil }
  return frame.id
}

func sleepMs(_ ms: UInt64) async {
  try? await Task.sleep(nanoseconds: ms * 1_000_000)
}
";

/// The five reply scenarios, each a Swift string literal already carrying the placeholder id
/// [`genuine_reply`] bakes in: header present, the optional element absent, a declared error with
/// and without its own header element, and a required header missing entirely.
struct CannedReplies {
    a: String,
    b: String,
    missing_etag: String,
    refused: String,
    refused_no_reason: String,
}

/// The emitted service surface a Swift driver runs against.
fn emitted() -> String {
    [
        "import Foundation".to_owned(),
        stamp_receipt_swift::swift_definition(),
        stamp_error_swift::swift_definition(),
        stamp_client_service_fault_fields_swift::swift_definition(),
        stamp_client_service_fault_kind_swift::swift_definition(),
        StampClientServiceSchema::swift_http_client(),
        StampClientServiceSchema::swift_ws_client(),
    ]
    .join("\n\n")
}

/// Runs `driver` beside the emitted surface and reads the one JSON line it prints, or stands
/// down where no Swift toolchain is reachable.
fn run(named: &str, driver: &str) -> Option<Value> {
    let module = format!("{}\n\n{HELPERS}\n\n{driver}", emitted());
    let wrote = ran(named, RUNTIME_VAR, "swift", "main.swift", &module)?;
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

/// A function that puts a text frame on the wire, recording it instead.
fn recording(
    sent: &Arc<Mutex<Vec<String>>>,
) -> impl Fn(String) -> Ready<Result<(), String>> + Send + Sync + 'static {
    let mailbox = Arc::clone(sent);
    move |text| {
        mailbox.lock().unwrap().push(text);
        ready(Ok(()))
    }
}

/// A Rust client over a session whose every written frame is recorded — a fresh one per call, so
/// its own counter always starts at `"1"`, the id a caller can safely bake into a driver as a
/// placeholder and replace with the id the Swift client assigned it.
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

/// `frame` with its own `id` removed — the one field the Rust and the Swift client are free to
/// number differently.
fn without_id(mut frame: Value) -> Value {
    if let Some(object) = frame.as_object_mut() {
        object.remove("id");
    }
    frame
}

/// Every `stamp` frame in `frames`, id stripped.
fn stamp_frames(frames: &[Value]) -> Vec<Value> {
    frames
        .iter()
        .filter(|frame| frame["operation"] == "stamp")
        .cloned()
        .map(without_id)
        .collect()
}

/// Each frame in `swift` matches exactly one frame in `rust`, id aside — a multiset match by
/// value rather than by position, since `sent`'s own JSON object keys are not written in a
/// canonical order and the four calls start concurrently under `async let`.
fn assert_stamp_frames_match(swift: &[Value], rust: &[Value]) {
    let mut remaining = rust.to_vec();
    for frame in swift {
        let found = remaining.iter().position(|candidate| candidate == frame);
        assert!(
            found.is_some(),
            "no Rust stamp frame equals {frame:#?}. Got: {swift:#?} vs {rust:#?}"
        );
        remaining.remove(found.unwrap());
    }
    assert!(
        remaining.is_empty(),
        "Rust sent stamp frames Swift did not. Got: {swift:#?} vs {rust:#?}"
    );
}

/// One `stamp` call's own reply, computed against the real Rust dispatcher through a fresh Rust
/// client (so the request frame — and the reply echoing its id — always carries the placeholder
/// id `"1"`), ready for a Swift driver to graft its own real id onto.
fn genuine_reply(label: &str, tenant: &str, trace: Option<u32>) -> String {
    let backend = StampBackEnd::default();
    let (client, sent) = rust_client();
    let mut call = pin!(client.stamp(label.to_owned(), tenant.to_owned(), trace));
    assert!(poll_by_hand(call.as_mut()).is_pending());
    let frame = sent.lock().unwrap()[0].clone();
    rust_answer(&backend, &frame).unwrap()
}

/// A Swift string literal carrying `text`, escaping only what a genuine reply or a hand-built one
/// can contain: `"` and `\`.
fn swift_string_literal(text: &str) -> String {
    format!("\"{}\"", text.replace('\\', "\\\\").replace('"', "\\\""))
}

#[test]
fn the_swift_client_writes_the_frames_the_rust_client_writes() {
    let driver = r#"
func main() async {
  let socket = RelaySocket()
  let transport = StampClientServiceWsTransport(socket: socket, options: .init(heartbeat: nil))
  async let a = transport.stamp("a", tenant: "acme", trace: 7)
  async let b = transport.stamp("b", tenant: "acme", trace: nil)
  async let refuse1 = transport.stamp("refuse", tenant: "acme", trace: 3)
  async let refuse2 = transport.stamp("refuse", tenant: "acme", trace: nil)
  await sleepMs(50)
  try? await transport.mark("m", tenant: "acme")
  await sleepMs(20)
  let sent = socket.sent
  await transport.close()
  _ = await (a, b, refuse1, refuse2)
  print(String(data: try! JSONEncoder().encode(sent), encoding: .utf8)!)
}
await main()
"#;
    let Some(written) = run("swift-ws-headers-writes", driver) else {
        return;
    };
    let sent: Vec<Value> = written
        .as_array()
        .unwrap()
        .iter()
        .map(|frame| parsed(frame.as_str().unwrap()))
        .collect();

    let (client, rust_sent) = rust_client();
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
    let rust: Vec<Value> = rust_sent
        .lock()
        .unwrap()
        .iter()
        .map(|f| parsed(f))
        .collect();

    assert_stamp_frames_match(&stamp_frames(&sent), &stamp_frames(&rust));
    let swift_mark = sent.iter().find(|frame| frame["operation"] == "mark");
    let rust_mark = rust.iter().find(|frame| frame["operation"] == "mark");
    assert_eq!(
        swift_mark.cloned().map(without_id),
        rust_mark.cloned().map(without_id),
        "the mark frame. Got: {sent:#?} vs {rust:#?}"
    );
}

fn canned_replies() -> CannedReplies {
    CannedReplies {
        a: swift_string_literal(&genuine_reply("a", "acme", Some(7))),
        b: swift_string_literal(&genuine_reply("b", "acme", None)),
        // No real dispatcher ever omits a required `header_out` element; built by hand, mirroring
        // the malformed frames `run_node_ws_headers.rs`'s own answers group feeds its dispatcher.
        missing_etag: swift_string_literal(
            r#"{"kind":"reply","id":"1","service":"StampClientService","ok":true,"value":{"label":"c","tenant":"acme"},"headers":{"x-age":9}}"#,
        ),
        refused: swift_string_literal(&genuine_reply("refuse", "acme", Some(3))),
        refused_no_reason: swift_string_literal(&genuine_reply("refuse", "acme", None)),
    }
}

/// Fires each scenario in turn against one transport, grafting the reply's own placeholder id
/// onto whatever id the transport actually assigned before delivering it, and prints the
/// described outcomes as one JSON object.
fn reads_driver(canned: &CannedReplies) -> String {
    let CannedReplies {
        a,
        b,
        missing_etag,
        refused,
        refused_no_reason,
    } = canned;
    format!(
        r#"
struct StampReport: Codable {{
  let ok: Bool
  let label: String?
  let etag: String?
  let ageIsNil: Bool?
  let errorCode: String?
  let reasonIsNil: Bool?
  let faultKind: String?
}}

func describe(
  _ outcome: Result<(StampReceipt, String, UInt32?), StampClientServiceStampFailure>
) -> StampReport {{
  switch outcome {{
  case .success(let value):
    return StampReport(
      ok: true, label: value.0.label, etag: value.1, ageIsNil: value.2 == nil,
      errorCode: nil, reasonIsNil: nil, faultKind: nil
    )
  case .failure(.declared(let error)):
    let code: String
    switch error.0 {{ case .refused: code = "refused" }}
    return StampReport(
      ok: false, label: nil, etag: nil, ageIsNil: nil,
      errorCode: code, reasonIsNil: error.1 == nil, faultKind: nil
    )
  case .failure(.fault(let fault)):
    return StampReport(
      ok: false, label: nil, etag: nil, ageIsNil: nil,
      errorCode: nil, reasonIsNil: nil, faultKind: fault.kind.rawValue
    )
  }}
}}

/// Fires one call, grafts `canned`'s own placeholder id onto the request the transport actually
/// sent, delivers it, and answers the described outcome — fully sequential, so two calls sharing
/// one label (`refuse`) never race for the same reply.
func scenario(
  _ transport: StampClientServiceWsTransport,
  _ socket: RelaySocket,
  label: String,
  trace: UInt32?,
  canned: String
) async -> StampReport {{
  async let outcome = transport.stamp(label, tenant: "acme", trace: trace)
  await sleepMs(30)
  let id = requestId(from: socket.sent.last)!
  socket.deliver(canned.replacingOccurrences(of: "\"id\":\"1\"", with: "\"id\":\"\(id)\""))
  return describe(await outcome)
}}

func main() async {{
  let socket = RelaySocket()
  let transport = StampClientServiceWsTransport(socket: socket, options: .init(heartbeat: nil))
  var results: [String: StampReport] = [:]
  results["a"] = await scenario(transport, socket, label: "a", trace: 7, canned: {a})
  results["b"] = await scenario(transport, socket, label: "b", trace: nil, canned: {b})
  results["refused"] = await scenario(
    transport, socket, label: "refuse", trace: 3, canned: {refused}
  )
  results["refusedNoReason"] = await scenario(
    transport, socket, label: "refuse", trace: nil, canned: {refused_no_reason}
  )
  results["missingEtag"] = await scenario(
    transport, socket, label: "c", trace: nil, canned: {missing_etag}
  )
  await transport.close()
  print(String(data: try! JSONEncoder().encode(results), encoding: .utf8)!)
}}
await main()
"#
    )
}

#[test]
fn the_swift_client_reads_the_replies_the_rust_dispatcher_writes() {
    let canned = canned_replies();
    let driver = reads_driver(&canned);
    let Some(results) = run("swift-ws-headers-reads", &driver) else {
        return;
    };
    assert_eq!(results["a"]["ok"], true, "got: {results:#?}");
    assert_eq!(results["a"]["label"], "a", "got: {results:#?}");
    assert_eq!(results["a"]["etag"], "etag-acme", "got: {results:#?}");
    assert_eq!(
        results["a"]["ageIsNil"], false,
        "the optional element is present when the backend answers one. Got: {results:#?}"
    );
    assert_eq!(
        results["b"]["ageIsNil"], true,
        "the optional element decodes as absent rather than faulting. Got: {results:#?}"
    );
    assert_eq!(results["refused"]["ok"], false, "got: {results:#?}");
    assert_eq!(
        results["refused"]["errorCode"], "refused",
        "got: {results:#?}"
    );
    assert_eq!(
        results["refused"]["reasonIsNil"], false,
        "the declared error's own header element is present. Got: {results:#?}"
    );
    assert_eq!(
        results["refusedNoReason"]["reasonIsNil"], true,
        "an absent declared-error header decodes as `nil` rather than faulting. \
         Got: {results:#?}"
    );
    assert_eq!(
        results["missingEtag"]["faultKind"], "failed-validation",
        "a missing required header is a fault, not a silent default. Got: {results:#?}"
    );
}
