//! The emitted Swift client run under a Swift toolchain: the codec rows the Swift spike proved —
//! renamed and optional fields, the tagged and untagged enum forms, a tuple, a generic struct,
//! non-string map keys — round-tripped through the emitted `Codable` text, the `http_rest`
//! client's own URLs (the same three `run_dart.rs` asserts), and the `ws_rpc` client's own
//! scenarios against an in-memory socket.
//!
//! Every group runs `swift main.swift` in immediate mode — no package manifest, Foundation only —
//! standing down exactly as [`super::run_dart`] does where no Swift toolchain is reachable.

#![cfg(feature = "swift")]

use super::runtime::ran;
use super::tests::conversation_client_service_schema::{
    conversation_client_service_fault_fields_swift, conversation_client_service_fault_kind_swift,
};
use super::tests::stamp_client_service_schema::{
    stamp_client_service_fault_fields_swift, stamp_client_service_fault_kind_swift,
};
use super::tests::swift_codec_fixture::{
    CodecAdjacentTagged, CodecEnvelope, CodecExternalTagged, CodecInternalTagged, CodecMapKeys,
    CodecOptionsRow, CodecPrimary, CodecTuplePoint, CodecUnitField, CodecUnitPayload,
    CodecUntagged, codec_adjacent_tagged_swift, codec_envelope_swift, codec_external_tagged_swift,
    codec_internal_tagged_swift, codec_map_keys_swift, codec_options_row_swift,
    codec_primary_swift, codec_tuple_point_swift, codec_unit_field_swift, codec_unit_payload_swift,
    codec_untagged_swift,
};
use super::tests::{
    ConversationClientServiceSchema, EchoClientServiceSchema, StampClientServiceSchema,
    ThumbnailClientServiceSchema, conversation_id_swift, echo_client_service_schema,
    echo_range_error_swift, echo_range_response_swift, stamp_error_swift, stamp_receipt_swift,
    thumbnail_client_service_schema, thumbnail_error_swift, window_error_swift, window_page_swift,
    window_request_swift,
};
use std::collections::HashMap;

/// Names the runtime to run, for a machine that has one somewhere other than `PATH`.
const RUNTIME_VAR: &str = "TIXSCHEMA_SWIFT";

/// Group 2's own driver: a recording `ConversationClientServiceHttpTransport` answering 200 for
/// every call but `DELETE`, which it answers 204 — then the three URL assertions `run_dart.rs`
/// makes of the same three calls.
const REST_DRIVER: &str = r##"
actor Recorder: ConversationClientServiceHttpTransport {
  private(set) var sent: [[String: String]] = []

  func send(_ request: ConversationClientServiceHttpRequest) async throws -> ConversationClientServiceHttpResponse {
    sent.append(["method": request.method, "path": request.path, "query": request.query])
    if request.method == "DELETE" {
      return ConversationClientServiceHttpResponse(status: 204, headers: [], body: Data())
    }
    return ConversationClientServiceHttpResponse(status: 200, headers: [], body: Data(#"{"items":[]}"#.utf8))
  }
}

let recorder = Recorder()
let client = ConversationClientServiceHttpClient(transport: recorder)
_ = await client.window(WindowRequest(conversationId: "652f1a3b4c5d6e7f8a9b0c1d", limit: 10))
_ = await client.window(WindowRequest(conversationId: "652f1a3b4c5d6e7f8a9b0c1d", limit: nil))
try! await client.purgeConversation(ConversationId(value: "652f1a3b4c5d6e7f8a9b0c1d"))
let sent = await recorder.sent
print(String(data: try! JSONEncoder().encode(sent), encoding: .utf8)!)
"##;

/// Group 3's own driver: five scenarios against an in-memory socket fed captured frames, printed
/// as one JSON report.
const WS_DRIVER: &str = r#"
final class InMemorySocket: ConversationClientServiceWsSocket, @unchecked Sendable {
  private let lock = NSLock()
  private var _sent: [String] = []
  private var _closed = false

  var onMessage: (@Sendable (String) -> Void)?
  var onClose: (@Sendable () -> Void)?

  var sent: [String] {
    lock.lock()
    defer { lock.unlock() }
    return _sent
  }

  var isClosed: Bool {
    lock.lock()
    defer { lock.unlock() }
    return _closed
  }

  func send(_ text: String) {
    lock.lock()
    _sent.append(text)
    lock.unlock()
  }

  func close() {
    lock.lock()
    let already = _closed
    _closed = true
    lock.unlock()
    if !already { onClose?() }
  }

  func deliver(_ text: String) {
    onMessage?(text)
  }
}

func requestId(from sent: String?) -> String? {
  guard let sent, let data = sent.data(using: .utf8),
        let frame = try? JSONDecoder().decode(ConversationClientServiceWsFrameProbe.self, from: data)
  else { return nil }
  return frame.id
}

// A ping is answered with one pong.
let pingSocket = InMemorySocket()
let pingTransport = ConversationClientServiceWsTransport(socket: pingSocket, options: .init(heartbeat: nil))
pingSocket.deliver("{\"kind\":\"ping\"}")
try? await Task.sleep(nanoseconds: 50_000_000)
let pongSeen = pingSocket.sent.contains("{\"kind\":\"pong\"}")
await pingTransport.close()

// A silent socket closes between the heartbeat interval and its own timeout.
let heartbeatSocket = InMemorySocket()
let heartbeatStart = Date()
let heartbeatTransport = ConversationClientServiceWsTransport(
  socket: heartbeatSocket,
  options: .init(heartbeat: .init(intervalMs: 150, timeoutMs: 100))
)
while !heartbeatSocket.isClosed {
  try? await Task.sleep(nanoseconds: 5_000_000)
}
let closeElapsedMs = Int(Date().timeIntervalSince(heartbeatStart) * 1000)
await heartbeatTransport.close()

// A reply that will not become the declared success answers a failed-validation fault.
let invalidSocket = InMemorySocket()
let invalidTransport = ConversationClientServiceWsTransport(socket: invalidSocket, options: .init(heartbeat: nil))
async let invalidOutcome = invalidTransport.window(WindowRequest(conversationId: "abc", limit: nil))
try? await Task.sleep(nanoseconds: 50_000_000)
let invalidId = requestId(from: invalidSocket.sent.first)!
invalidSocket.deliver("{\"kind\":\"reply\",\"id\":\"\(invalidId)\",\"service\":\"ConversationClientService\",\"ok\":true,\"value\":7}")
var failedValidationKind = "none"
if case .failure(.fault(let fault)) = await invalidOutcome {
  failedValidationKind = fault.kind.rawValue
}
await invalidTransport.close()

// Closing with a request still pending settles it with a transport-failure fault.
let closingSocket = InMemorySocket()
let closingTransport = ConversationClientServiceWsTransport(socket: closingSocket, options: .init(heartbeat: nil))
async let closingOutcome = closingTransport.window(WindowRequest(conversationId: "abc", limit: nil))
try? await Task.sleep(nanoseconds: 50_000_000)
await closingTransport.close()
var transportFailureKind = "none"
if case .failure(.fault(let fault)) = await closingOutcome {
  transportFailureKind = fault.kind.rawValue
}

// A frame naming another service is dropped; the genuine reply still resolves the call.
let foreignSocket = InMemorySocket()
let foreignTransport = ConversationClientServiceWsTransport(socket: foreignSocket, options: .init(heartbeat: nil))
async let foreignOutcome = foreignTransport.window(WindowRequest(conversationId: "abc", limit: nil))
try? await Task.sleep(nanoseconds: 50_000_000)
let foreignId = requestId(from: foreignSocket.sent.first)!
foreignSocket.deliver("{\"kind\":\"reply\",\"id\":\"\(foreignId)\",\"service\":\"OtherService\",\"ok\":true,\"value\":{\"items\":[]}}")
foreignSocket.deliver("{\"kind\":\"reply\",\"id\":\"\(foreignId)\",\"service\":\"ConversationClientService\",\"ok\":true,\"value\":{\"items\":[]}}")
var foreignResolvedOk = false
if case .success = await foreignOutcome {
  foreignResolvedOk = true
}
await foreignTransport.close()

struct Report: Encodable {
  let pong: Bool
  let closeElapsedMs: Int
  let failedValidationKind: String
  let transportFailureKind: String
  let foreignResolvedOk: Bool
}
let report = Report(
  pong: pongSeen,
  closeElapsedMs: closeElapsedMs,
  failedValidationKind: failedValidationKind,
  transportFailureKind: transportFailureKind,
  foreignResolvedOk: foreignResolvedOk
)
print(String(data: try! JSONEncoder().encode(report), encoding: .utf8)!)
"#;

/// A stub `ThumbnailClientServiceHttpTransport` answering by path alone, driving the emitted
/// client's own declared-error and `header_out` decode - the Swift twin of the Node and Dart
/// client tests on the same fixture.
const THUMBNAIL_DRIVER: &str = r##"
struct ThumbnailReport: Codable {
  let kind: String
  let errorCode: String?
  let errorHeaderOut: String?
  let headerOut: String?
}

struct ThumbnailRecorder: ThumbnailClientServiceHttpTransport {
  func send(_ request: ThumbnailClientServiceHttpRequest) async throws -> ThumbnailClientServiceHttpResponse {
    if request.path == "/thumbnails/missing" {
      return ThumbnailClientServiceHttpResponse(status: 404, headers: [("x-thumbnail-reason", "archived")], body: Data(#"{"errorCode":"not-found"}"#.utf8))
    }
    if request.path == "/thumbnails/gone" {
      return ThumbnailClientServiceHttpResponse(status: 404, headers: [], body: Data(#"{"errorCode":"not-found"}"#.utf8))
    }
    return ThumbnailClientServiceHttpResponse(status: 200, headers: [("content-type", "image/png")], body: Data("PNGDATA".utf8))
  }
}

func describeThumbnail(_ result: Result<(Data, String, String?), ThumbnailClientServiceGetThumbnailFailure>) -> ThumbnailReport {
  switch result {
  case .success(let value):
    return ThumbnailReport(kind: "ok", errorCode: nil, errorHeaderOut: nil, headerOut: value.2)
  case .failure(.declared(let error)):
    let code: String
    switch error.0 {
    case .notFound: code = "not-found"
    }
    return ThumbnailReport(kind: "operation", errorCode: code, errorHeaderOut: error.1, headerOut: nil)
  case .failure(.fault):
    return ThumbnailReport(kind: "fault", errorCode: nil, errorHeaderOut: nil, headerOut: nil)
  }
}

let thumbnailClient = ThumbnailClientServiceHttpClient(transport: ThumbnailRecorder())
let thumbnailMissing = await thumbnailClient.getThumbnail("missing")
let thumbnailGone = await thumbnailClient.getThumbnail("gone")
let thumbnailAnon = await thumbnailClient.getThumbnail("anon")
let thumbnailReport: [String: ThumbnailReport] = [
  "missing": describeThumbnail(thumbnailMissing),
  "gone": describeThumbnail(thumbnailGone),
  "anon": describeThumbnail(thumbnailAnon),
]
print(String(data: try! JSONEncoder().encode(thumbnailReport), encoding: .utf8)!)
"##;

/// A stub `EchoClientServiceHttpTransport` recording whether `send` was ever called, driving the
/// emitted client's own `header_in` legality check on a value carrying a line feed.
const ECHO_DRIVER: &str = r##"
actor EchoRecorder: EchoClientServiceHttpTransport {
  private(set) var sendCalled = false

  func send(_ request: EchoClientServiceHttpRequest) async throws -> EchoClientServiceHttpResponse {
    sendCalled = true
    return EchoClientServiceHttpResponse(status: 200, headers: [], body: Data(#"{"received":"unreachable"}"#.utf8))
  }
}

let echoRecorder = EchoRecorder()
let echoClient = EchoClientServiceHttpClient(transport: echoRecorder)
let echoRefused = await echoClient.echoRange("doc-1", byte_range: "bytes=0-10\nX-Injected: yes")
var echoFaultKind = ""
if case .failure(.fault(let fault)) = echoRefused {
  echoFaultKind = fault.kind.rawValue
}
struct EchoReport: Codable {
  let sendCalled: Bool
  let faultKind: String
}
let echoReport = EchoReport(sendCalled: await echoRecorder.sendCalled, faultKind: echoFaultKind)
print(String(data: try! JSONEncoder().encode(echoReport), encoding: .utf8)!)
"##;

/// A stub `StampClientServiceHttpTransport` answering with whatever response headers it is given,
/// driving the emitted client's own numeric `header_out` decode: present, absent, and a value
/// that will not parse as its declared `UInt32`.
const STAMP_HEADER_WIDTH_DRIVER: &str = r##"
struct StampRecorder: StampClientServiceHttpTransport {
  let headers: [(String, String)]
  func send(_ request: StampClientServiceHttpRequest) async throws -> StampClientServiceHttpResponse {
    StampClientServiceHttpResponse(status: 200, headers: headers, body: Data(#"{"label":"a","tenant":"acme"}"#.utf8))
  }
}

struct AgeReport: Codable {
  let age: UInt32?
  let ageIsNil: Bool
  let faultKind: String?
}

func describeAge(_ result: Result<(StampReceipt, String, UInt32?), StampClientServiceStampFailure>) -> AgeReport {
  switch result {
  case .success(let value):
    return AgeReport(age: value.2, ageIsNil: value.2 == nil, faultKind: nil)
  case .failure(.declared):
    return AgeReport(age: nil, ageIsNil: true, faultKind: nil)
  case .failure(.fault(let fault)):
    return AgeReport(age: nil, ageIsNil: true, faultKind: fault.kind.rawValue)
  }
}

func ageResult(_ headers: [(String, String)]) async -> AgeReport {
  let client = StampClientServiceHttpClient(transport: StampRecorder(headers: headers))
  return describeAge(await client.stamp("a", tenant: "acme", trace: nil))
}

struct Report: Codable {
  let present: AgeReport
  let absent: AgeReport
  let malformed: AgeReport
}
let report = Report(
  present: await ageResult([("etag", "etag-acme"), ("x-age", "8")]),
  absent: await ageResult([("etag", "etag-acme")]),
  malformed: await ageResult([("etag", "etag-acme"), ("x-age", "soon")])
)
print(String(data: try! JSONEncoder().encode(report), encoding: .utf8)!)
"##;

// -------------------------------------------------------------------------------------------
// The generated types and clients every group but the codec one drives.
// -------------------------------------------------------------------------------------------

/// The generated classes both clients call, both clients, and a driver.
fn client_module(driver: &str) -> String {
    [
        "import Foundation".to_owned(),
        conversation_id_swift::swift_definition(),
        window_request_swift::swift_definition(),
        window_page_swift::swift_definition(),
        window_error_swift::swift_definition(),
        conversation_client_service_fault_fields_swift::swift_definition(),
        conversation_client_service_fault_kind_swift::swift_definition(),
        ConversationClientServiceSchema::swift_http_client(),
        ConversationClientServiceSchema::swift_ws_client(),
        driver.to_owned(),
    ]
    .join("\n\n")
}

// -------------------------------------------------------------------------------------------
// Group 1: the codec rows the Swift spike proved, decoded from the JSON Rust wrote and re-encoded.
// -------------------------------------------------------------------------------------------

/// One row: the name it prints under, the Swift type it decodes into, and the JSON Rust wrote
/// for it — the same value the printed, re-encoded JSON must equal.
fn codec_rows() -> Vec<(&'static str, &'static str, serde_json::Value)> {
    vec![
        (
            "options",
            "CodecOptionsRow",
            serde_json::to_value(CodecOptionsRow {
                nullable_field: None,
                omitted_optional: None,
                plain_field: "hello".to_owned(),
                present_optional: Some(9),
            })
            .unwrap(),
        ),
        (
            "external_tagged",
            "CodecExternalTagged",
            serde_json::to_value(CodecExternalTagged::Foo {
                a: "ex-a".to_owned(),
            })
            .unwrap(),
        ),
        (
            "internal_tagged",
            "CodecInternalTagged",
            serde_json::to_value(CodecInternalTagged::Foo {
                a: "in-a".to_owned(),
            })
            .unwrap(),
        ),
        (
            "adjacent_tagged",
            "CodecAdjacentTagged",
            serde_json::to_value(CodecAdjacentTagged::Flag(true)).unwrap(),
        ),
        (
            "untagged",
            "CodecUntagged",
            serde_json::to_value(CodecUntagged::Text {
                text: "hi".to_owned(),
            })
            .unwrap(),
        ),
        (
            "tuple_point",
            "CodecTuplePoint",
            serde_json::to_value(CodecTuplePoint {
                label: "pt".to_owned(),
                pair: ("a".to_owned(), 7),
            })
            .unwrap(),
        ),
        (
            "envelope",
            "CodecEnvelope<String>",
            serde_json::to_value(CodecEnvelope {
                note: "n1".to_owned(),
                value: "payload".to_owned(),
            })
            .unwrap(),
        ),
        (
            "map_keys",
            "CodecMapKeys",
            serde_json::to_value(CodecMapKeys {
                counters: HashMap::from([(7, "seven".to_owned())]),
                tiers: HashMap::from([(CodecPrimary::Primary, "p".to_owned())]),
            })
            .unwrap(),
        ),
        (
            "unit_field",
            "CodecUnitField",
            serde_json::to_value(CodecUnitField {
                label: "marker".to_owned(),
                payload: CodecUnitPayload,
            })
            .unwrap(),
        ),
    ]
}

/// Rust's `"` and `\` are the only characters any row above can write, so escaping just those two
/// is enough to carry `json` inside a Swift string literal.
fn swift_string_literal(json: &str) -> String {
    format!("\"{}\"", json.replace('\\', "\\\\").replace('"', "\\\""))
}

/// One `decode/re-encode/print` block per row, each guarded so a decode failure prints a message
/// rather than aborting the rows after it.
fn codec_driver(rows: &[(&str, &str, serde_json::Value)]) -> String {
    rows.iter()
        .map(|(name, swift_type, value)| {
            let literal = swift_string_literal(&value.to_string());
            format!(
                "do {{\n  \
                 let decoded = try JSONDecoder().decode({swift_type}.self, from: {literal}.data(using: .utf8)!)\n  \
                 let reencoded = try JSONEncoder().encode(decoded)\n  \
                 print(\"{name}\\t\\(String(data: reencoded, encoding: .utf8)!)\")\n\
                 }} catch {{\n  \
                 print(\"{name}\\tERROR \\(error)\")\n\
                 }}"
            )
        })
        .collect::<Vec<_>>()
        .join("\n\n")
}

fn codec_module(driver: &str) -> String {
    [
        "import Foundation".to_owned(),
        codec_options_row_swift::swift_definition(),
        codec_external_tagged_swift::swift_definition(),
        codec_internal_tagged_swift::swift_definition(),
        codec_adjacent_tagged_swift::swift_definition(),
        codec_untagged_swift::swift_definition(),
        codec_tuple_point_swift::swift_definition(),
        codec_envelope_swift::swift_definition(),
        codec_primary_swift::swift_definition(),
        codec_map_keys_swift::swift_definition(),
        codec_unit_payload_swift::swift_definition(),
        codec_unit_field_swift::swift_definition(),
        driver.to_owned(),
    ]
    .join("\n\n")
}

#[test]
fn every_codec_row_round_trips_through_codable() {
    let rows = codec_rows();
    let driver = codec_driver(&rows);
    let Some(written) = ran(
        "swift",
        RUNTIME_VAR,
        "swift",
        "main.swift",
        &codec_module(&driver),
    ) else {
        return;
    };
    for (name, _, expected) in &rows {
        let prefix = format!("{name}\t");
        let found = written.lines().find(|line| line.starts_with(&prefix));
        assert!(
            found.is_some(),
            "no printed line for row `{name}`. Got:\n{written}"
        );
        let line = found.unwrap();
        let payload = &line[prefix.len()..];
        assert!(
            !payload.starts_with("ERROR"),
            "row `{name}` did not decode and re-encode: {payload}"
        );
        let parsed = serde_json::from_str::<serde_json::Value>(payload);
        assert!(
            parsed.is_ok(),
            "row `{name}` printed invalid JSON `{payload}`: {parsed:?}"
        );
        let actual = parsed.unwrap();
        assert_eq!(
            &actual, expected,
            "row `{name}` round-tripped to a different value. Got: {actual:#?}"
        );
    }
}

// -------------------------------------------------------------------------------------------
// Group 2: the REST client's own URLs — the same three `run_dart.rs` asserts.
// -------------------------------------------------------------------------------------------

/// What the recorder captured, or `None` where no runtime was reachable.
fn driven_rest() -> Option<Vec<serde_json::Value>> {
    let written = ran(
        "swift",
        RUNTIME_VAR,
        "swift",
        "main.swift",
        &client_module(REST_DRIVER),
    )?;
    let parsed: serde_json::Value = serde_json::from_str(written.trim()).unwrap();
    assert!(parsed.is_array(), "expected a JSON array, got: {parsed:#?}");
    parsed.as_array().cloned()
}

#[test]
fn a_lone_placeholder_sends_the_field_it_names_and_never_the_rendered_message() {
    let Some(sent) = driven_rest() else {
        return;
    };
    assert_eq!(
        sent[0]["path"], "/v1/conversations/652f1a3b4c5d6e7f8a9b0c1d/window",
        "the placeholder is filled by the field it names. Got: {sent:#?}"
    );
    assert!(
        !sent[0]["path"].as_str().unwrap().contains("%7B"),
        "rendering the whole message puts its own map spelling in the segment. Got: {sent:#?}"
    );
}

#[test]
fn a_field_the_path_does_not_bind_reaches_the_query_string() {
    let Some(sent) = driven_rest() else {
        return;
    };
    assert_eq!(
        sent[0]["query"], "limit=10",
        "`limit` is bound to no placeholder, so the query string is the only place left for it. \
         Got: {sent:#?}"
    );
    assert_eq!(
        sent[1]["query"], "",
        "the same operation with `limit` absent sends no key for it rather than `limit=null`. \
         Got: {sent:#?}"
    );
}

#[test]
fn a_scalar_message_is_still_the_whole_segment() {
    let Some(sent) = driven_rest() else {
        return;
    };
    assert_eq!(
        sent[2]["path"], "/v1/conversations/652f1a3b4c5d6e7f8a9b0c1d",
        "a message that already is a wire scalar has no field to read: it is the segment. \
         Got: {sent:#?}"
    );
    assert_eq!(
        sent[2]["query"], "",
        "and no key is left over for a query. Got: {sent:#?}"
    );
}

// -------------------------------------------------------------------------------------------
// Group 3: the `ws_rpc` client against an in-memory socket fed captured frames.
// -------------------------------------------------------------------------------------------

/// What the driver's own `Report` printed, or `None` where no runtime was reachable.
fn driven_ws() -> Option<serde_json::Value> {
    let written = ran(
        "swift",
        RUNTIME_VAR,
        "swift",
        "main.swift",
        &client_module(WS_DRIVER),
    )?;
    Some(serde_json::from_str(written.trim()).unwrap())
}

#[test]
fn an_inbound_ping_is_answered_with_one_pong() {
    let Some(written) = driven_ws() else {
        return;
    };
    assert_eq!(written["pong"], true, "got: {written:#?}");
}

#[test]
fn a_silent_socket_closes_between_the_heartbeat_interval_and_its_timeout() {
    let Some(written) = driven_ws() else {
        return;
    };
    let elapsed = written["closeElapsedMs"].as_i64().unwrap();
    assert!(
        (250..=400).contains(&elapsed),
        "a 150 ms interval plus a 100 ms timeout should close the socket around 250 ms in, got \
         {elapsed} ms. Full output: {written:#?}"
    );
}

#[test]
fn a_reply_that_fails_the_declared_decode_answers_a_failed_validation_fault() {
    let Some(written) = driven_ws() else {
        return;
    };
    assert_eq!(
        written["failedValidationKind"], "failed-validation",
        "got: {written:#?}"
    );
}

#[test]
fn closing_with_a_request_pending_answers_a_transport_failure_fault() {
    let Some(written) = driven_ws() else {
        return;
    };
    assert_eq!(
        written["transportFailureKind"], "transport-failure",
        "got: {written:#?}"
    );
}

#[test]
fn a_frame_for_another_service_is_dropped_and_the_genuine_reply_still_resolves() {
    let Some(written) = driven_ws() else {
        return;
    };
    assert_eq!(written["foreignResolvedOk"], true, "got: {written:#?}");
}

fn thumbnail_module() -> String {
    [
        "import Foundation".to_owned(),
        thumbnail_error_swift::swift_definition(),
        thumbnail_client_service_schema::thumbnail_client_service_fault_fields_swift::swift_definition(),
        thumbnail_client_service_schema::thumbnail_client_service_fault_kind_swift::swift_definition(),
        ThumbnailClientServiceSchema::swift_http_client(),
        THUMBNAIL_DRIVER.to_owned(),
    ]
    .join("\n\n")
}

#[test]
fn the_client_decodes_the_declared_errors_own_header_and_omits_a_none_header_out_element() {
    let Some(wrote) = ran(
        "swift",
        RUNTIME_VAR,
        "swift",
        "thumbnail.swift",
        &thumbnail_module(),
    ) else {
        return;
    };
    let results: serde_json::Value = serde_json::from_str(wrote.trim()).unwrap();
    assert_eq!(results["missing"]["kind"], "operation", "got: {results:#?}");
    assert_eq!(
        results["missing"]["errorCode"], "not-found",
        "got: {results:#?}"
    );
    assert_eq!(
        results["missing"]["errorHeaderOut"], "archived",
        "the declared error's own `error_header_out` element must decode. got: {results:#?}"
    );
    assert_eq!(results["gone"]["kind"], "operation", "got: {results:#?}");
    assert!(
        results["gone"]["errorHeaderOut"].is_null(),
        "an absent `error_header_out` header must decode as `nil`. got: {results:#?}"
    );
    assert_eq!(results["anon"]["kind"], "ok", "got: {results:#?}");
    assert!(
        results["anon"]["headerOut"].is_null(),
        "an absent `header_out` header must decode as `nil`. got: {results:#?}"
    );
}

fn echo_module() -> String {
    [
        "import Foundation".to_owned(),
        echo_range_response_swift::swift_definition(),
        echo_range_error_swift::swift_definition(),
        echo_client_service_schema::echo_client_service_fault_fields_swift::swift_definition(),
        echo_client_service_schema::echo_client_service_fault_kind_swift::swift_definition(),
        EchoClientServiceSchema::swift_http_client(),
        ECHO_DRIVER.to_owned(),
    ]
    .join("\n\n")
}

#[test]
fn a_header_in_value_with_a_line_feed_is_refused_before_the_transport_is_ever_reached() {
    let Some(wrote) = ran("swift", RUNTIME_VAR, "swift", "echo.swift", &echo_module()) else {
        return;
    };
    let results: serde_json::Value = serde_json::from_str(wrote.trim()).unwrap();
    assert_eq!(
        results["sendCalled"], false,
        "an illegal `header_in` value must refuse before the transport is ever reached. \
         got: {results:#?}"
    );
    assert_eq!(
        results["faultKind"], "failed-validation",
        "got: {results:#?}"
    );
}

fn stamp_module() -> String {
    [
        "import Foundation".to_owned(),
        stamp_receipt_swift::swift_definition(),
        stamp_error_swift::swift_definition(),
        stamp_client_service_fault_fields_swift::swift_definition(),
        stamp_client_service_fault_kind_swift::swift_definition(),
        StampClientServiceSchema::swift_http_client(),
        STAMP_HEADER_WIDTH_DRIVER.to_owned(),
    ]
    .join("\n\n")
}

#[test]
fn a_numeric_header_out_element_reads_present_absent_and_a_value_that_will_not_parse() {
    let Some(wrote) = ran(
        "swift",
        RUNTIME_VAR,
        "swift",
        "stamp.swift",
        &stamp_module(),
    ) else {
        return;
    };
    let results: serde_json::Value = serde_json::from_str(wrote.trim()).unwrap();
    assert_eq!(results["present"]["age"], 8_u32, "got: {results:#?}");
    assert_eq!(results["absent"]["ageIsNil"], true, "got: {results:#?}");
    assert_eq!(
        results["malformed"]["faultKind"], "undeserializable-payload",
        "a present header that will not parse as its declared type must fault rather than \
         default to 0. got: {results:#?}"
    );
}
