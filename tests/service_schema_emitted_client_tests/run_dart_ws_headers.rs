//! Headers both ways, run by the Dart VM against the Rust `ws_rpc` twins of
//! `StampClientService` — the Dart twin of [`super::run_node_ws_headers`].

#![cfg(feature = "dart")]

use super::runtime::ran;
use super::stamp_client_service_schema::CallError;
use super::stamp_ws_rpc_client::{FrameSession, FrameWriter, StampClientServiceClient, ping_frame};
use super::stamp_ws_rpc_transport;
use super::tests::{
    StampBackEnd, StampClientServiceSchema, StampError, StampReceipt, stamp_error_dart,
    stamp_receipt_dart,
};
use alloc::sync::Arc;
use core::future::{Future, Ready, ready};
use core::pin::{Pin, pin};
use core::task::{Context as PollContext, Poll, Waker};
use serde_json::{Value, json};
use std::sync::Mutex;

/// Names the runtime to run, for a machine that has one somewhere other than `PATH`.
const RUNTIME_VAR: &str = "TIXSCHEMA_DART";

/// Polls rather than sleeping a fixed duration, every driver below waits on this.
const HELPERS: &str = "
Future<bool> until(bool Function() predicate, {int timeoutMs = 2000}) async {
  final start = DateTime.now();
  while (!predicate() && DateTime.now().difference(start).inMilliseconds < timeoutMs) {
    await Future<void>.delayed(const Duration(milliseconds: 5));
  }
  return predicate();
}
";

/// The Dart mirror of the Rust and TypeScript twins' own `CALLS`.
const WRITES_FRAMES_DRIVER: &str = "
void main() async {
  final sentRaw = <dynamic>[];
  final outbound = StreamController<dynamic>();
  outbound.stream.listen(sentRaw.add);
  final inbound = StreamController<dynamic>();
  final transport = StampClientServiceWsTransport(
    sink: outbound.sink,
    stream: inbound.stream,
    heartbeat: StampClientServiceWsHeartbeat.off(),
  );
  final client = StampClientServiceWsClient(transport);
  client.stamp('a', 'acme', 7);
  client.stamp('b', 'acme', null);
  client.stamp('refuse', 'acme', 3);
  client.stamp('refuse', 'acme', null);
  await client.mark('m', 'acme');
  await Future<void>.delayed(Duration.zero);
  print(jsonEncode(sentRaw.map((frame) => frame as String).toList()));
}
";

/// The same four calls again (ids are deterministic, so they line up with `REPLIES`), then a
/// fifth carrying a hand-built reply missing its required `etag`.
const READS_REPLIES_DRIVER: &str = "
void main() async {
  final sentRaw = <dynamic>[];
  final outbound = StreamController<dynamic>();
  outbound.stream.listen(sentRaw.add);
  final inbound = StreamController<dynamic>();
  final transport = StampClientServiceWsTransport(
    sink: outbound.sink,
    stream: inbound.stream,
    heartbeat: StampClientServiceWsHeartbeat.off(),
  );
  final client = StampClientServiceWsClient(transport);

  Future<String> lastSentId() async {
    await Future<void>.delayed(Duration.zero);
    final frame = jsonDecode(sentRaw.last as String) as Map<String, dynamic>;
    return frame['id'] as String;
  }

  Map<String, dynamic> described(Object result) {
    if (result is StampClientServiceStampResultOk) {
      final value = result.value;
      return {'kind': 'ok', 'etag': value.$2, 'ageIsNull': value.$3 == null};
    }
    if (result is StampClientServiceStampResultOperation) {
      return {'kind': 'operation', 'reasonIsNull': result.error.$2 == null};
    }
    final fault = (result as StampClientServiceStampResultFault).fault;
    return {'kind': 'fault', 'faultKind': fault.kind.name};
  }

  final results = <Map<String, dynamic>>[];

  final first = client.stamp('a', 'acme', 7);
  inbound.add(REPLIES[await lastSentId()]);
  results.add(described(await first));

  final second = client.stamp('b', 'acme', null);
  inbound.add(REPLIES[await lastSentId()]);
  results.add(described(await second));

  final third = client.stamp('refuse', 'acme', 3);
  inbound.add(REPLIES[await lastSentId()]);
  results.add(described(await third));

  final fourth = client.stamp('refuse', 'acme', null);
  inbound.add(REPLIES[await lastSentId()]);
  results.add(described(await fourth));

  final fifth = client.stamp('x', 'acme', null);
  final fifthId = await lastSentId();
  inbound.add(jsonEncode(<String, dynamic>{
    'kind': 'reply',
    'id': fifthId,
    'service': 'StampClientService',
    'ok': true,
    'value': <String, dynamic>{'label': 'x', 'tenant': 'acme'},
  }));
  results.add(described(await fifth));

  print(jsonEncode(results));
}
";

/// `FRAMES` and `EXPECTED_REPLIES` are written above this by the Rust side. The implementation
/// answers exactly as `StampBackEnd` does.
const ANSWERS_FRAMES_DRIVER: &str = "
void main() async {
  final written = <Map<String, dynamic>>[];
  final marked = <String>[];
  final faults = <String>[];
  final served = StreamController<Map<String, dynamic>>();
  final detach = attachStampClientServiceWsDispatcher<void>(
    (inbound: served.stream, send: written.add),
    null,
    StampClientServiceHandlers<void>(
      stamp: (ctx, label, tenant, trace) async {
        if (label == 'refuse') {
          throw (StampErrorRefused(), trace == null ? null : 'refused at $trace');
        }
        return (
          StampReceipt(label: label, tenant: tenant),
          'etag-$tenant',
          trace == null ? null : trace + 1,
        );
      },
      mark: (ctx, label, tenant) async {
        marked.add('$label $tenant');
      },
    ),
    onFault: (fault) => faults.add(fault.kind.name),
  );
  for (final frame in FRAMES) {
    served.add(jsonDecode(frame as String) as Map<String, dynamic>);
  }
  await until(() => written.length == EXPECTED_REPLIES && faults.length == 2);
  detach();
  print(jsonEncode(<String, dynamic>{
    'written': written.map((reply) => jsonEncode(reply)).toList(),
    'marked': marked,
    'faults': faults,
  }));
}
";

/// The service's own emitted surface: its sibling types and both `ws_rpc` artifacts.
fn emitted() -> String {
    [
        "import 'dart:async';".to_owned(),
        "import 'dart:convert';".to_owned(),
        stamp_receipt_dart::dart_definition(),
        stamp_error_dart::dart_definition(),
        StampClientServiceSchema::dart_definition(),
        StampClientServiceSchema::dart_ws_client(),
    ]
    .join("\n\n")
}

/// Runs `driver` beside the emitted surface and `preamble`, and reads the one JSON line it
/// printed — or stands down where no Dart runtime is reachable.
fn run(named: &str, preamble: &str, driver: &str) -> Option<Value> {
    let module = [
        emitted(),
        HELPERS.to_owned(),
        preamble.to_owned(),
        driver.to_owned(),
    ]
    .join("\n\n");
    let wrote = ran(named, RUNTIME_VAR, "dart", "headers.dart", &module)?;
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

/// The frames the Dart client writes for the four `stamp` calls and `mark`.
fn dart_client_frames() -> Option<Value> {
    run("dart-ws-headers-writes", "", WRITES_FRAMES_DRIVER)
}

/// [`dart_client_frames`]'s own payload, read back once a caller already knows a run happened.
fn frame_texts(written: &Value) -> Vec<String> {
    written
        .as_array()
        .unwrap()
        .iter()
        .map(|frame| frame.as_str().unwrap().to_owned())
        .collect()
}

#[test]
fn the_dart_client_writes_the_frames_the_rust_client_writes() {
    let Some(written) = dart_client_frames() else {
        return;
    };
    let sent = frame_texts(&written);
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
        .map(|frame| parsed(frame))
        .collect();
    let dart: Vec<Value> = sent.iter().map(|frame| parsed(frame)).collect();
    assert_eq!(dart, rust);
    assert_eq!(
        dart[0]["headers"],
        json!({ "x-tenant": "acme", "x-trace": 7_u32 }),
        "each header value crosses as the JSON value it is"
    );
    assert_eq!(
        dart[1]["headers"],
        json!({ "x-tenant": "acme", "x-trace": null }),
        "an optional header holding nothing crosses as `null`, as the Rust client writes a `None`"
    );
    client.transport().close("the run is over");
}

#[test]
fn the_dart_client_reads_the_replies_the_rust_dispatcher_writes() {
    let Some(written) = dart_client_frames() else {
        return;
    };
    let frames = frame_texts(&written);
    let backend = StampBackEnd::default();
    let mut replies = serde_json::Map::new();
    for frame in &frames {
        if let Some(reply) = rust_answer(&backend, frame) {
            let id = parsed(frame)["id"].as_str().unwrap().to_owned();
            replies.insert(id, Value::String(reply));
        }
    }
    let Some(results) = run(
        "dart-ws-headers-reads",
        &format!("const REPLIES = {};", Value::Object(replies)),
        READS_REPLIES_DRIVER,
    ) else {
        return;
    };
    assert_eq!(
        results,
        json!([
            { "kind": "ok", "etag": "etag-acme", "ageIsNull": false },
            { "kind": "ok", "etag": "etag-acme", "ageIsNull": true },
            { "kind": "operation", "reasonIsNull": false },
            { "kind": "operation", "reasonIsNull": true },
            { "kind": "fault", "faultKind": "failedValidation" },
        ])
    );
}

#[test]
fn the_dart_attachment_answers_the_rust_client_the_way_the_rust_dispatcher_does() {
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
        "dart-ws-headers-answers",
        &format!(
            "const FRAMES = {};\nconst EXPECTED_REPLIES = 5;",
            Value::from(frames.clone())
        ),
        ANSWERS_FRAMES_DRIVER,
    ) else {
        return;
    };
    assert_eq!(answered["marked"], json!(["m acme"]));
    assert_eq!(
        answered["faults"].as_array().unwrap().len(),
        2,
        "both frames missing their required header fault, and reach no handler. Got: {answered}"
    );
    assert_eq!(answered["faults"][0], json!("failedValidation"));

    let backend = StampBackEnd::default();
    let written: Vec<String> = answered["written"]
        .as_array()
        .unwrap()
        .iter()
        .map(|reply| reply.as_str().unwrap().to_owned())
        .collect();
    for frame in frames.iter().filter(|frame| frame.contains("\"request\"")) {
        let id = parsed(frame)["id"].clone();
        let dart = written
            .iter()
            .map(|reply| parsed(reply))
            .find(|reply| reply["id"] == id)
            .unwrap();
        let rust = parsed(&rust_answer(&backend, frame).unwrap());
        assert_eq!(
            without_fault_detail(dart),
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

/// The heartbeat probe the Dart transport writes is the Rust client's own — a lone regression
/// check `run_dart_ws.rs` has no fixture for, since none of its own services declare headers.
#[test]
fn the_dart_transport_s_heartbeat_probe_is_the_rust_client_s_own() {
    let Some(written) = run(
        "dart-ws-headers-ping",
        "",
        "
void main() async {
  final probes = <dynamic>[];
  final outbound = StreamController<dynamic>();
  outbound.stream.listen(probes.add);
  final inbound = StreamController<dynamic>();
  final transport = StampClientServiceWsTransport(
    sink: outbound.sink,
    stream: inbound.stream,
    heartbeat: const StampClientServiceWsHeartbeat(
      interval: Duration(milliseconds: 1),
      timeout: Duration(seconds: 1000),
    ),
  );
  await until(() => probes.isNotEmpty);
  transport.close();
  print(probes.first);
}
",
    ) else {
        return;
    };
    assert_eq!(
        written,
        serde_json::from_str::<Value>(&ping_frame()).unwrap()
    );
}
