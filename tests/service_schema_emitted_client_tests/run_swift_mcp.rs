//! The emitted Swift `mcp` client run under a Swift toolchain: it reads the outcomes the design
//! lists from the answers the Rust dispatcher gives it, writes the requests the Rust dispatcher
//! reads, and numbers them in its own state.

#![cfg(feature = "swift")]

use super::mcp_mobile_rows::{
    CLIENT_LINES, assert_client_request, client_calls, client_replies, swift_text,
};
use super::mcp_mobile_service::{
    archive_document_request_swift, balance_error_swift, balance_response_swift, document_id_swift,
    get_version_request_swift, rename_document_request_swift, touch_document_request_swift,
    version_error_swift, version_response_swift,
};
use super::runtime::ran;
use super::{MobileDocumentServiceSchema, mobile_document_service_schema};
use serde_json::{Value, json};
use std::sync::OnceLock;

const RUNTIME_VAR: &str = "TIXSCHEMA_SWIFT";

const DRIVER: &str = r#"
func described(_ failure: MobileDocumentServiceGetVersionFailure) -> String {
  switch failure {
  case .declared: return "declared not-found"
  case .fault(let fault): return "fault \(fault.kind.rawValue)"
  }
}

final class Canned: MobileDocumentServiceMcpTransport, @unchecked Sendable {
  struct Down: Error {}

  private let lock = NSLock()
  private var recorded: [MobileDocumentServiceMcpMessage] = []
  private let replies: [Int: String?]

  init(replies: [Int: String?]) { self.replies = replies }

  var sent: [MobileDocumentServiceMcpMessage] {
    lock.lock()
    defer { lock.unlock() }
    return recorded
  }

  func send(_ message: MobileDocumentServiceMcpMessage) async throws -> String {
    lock.lock()
    recorded.append(message)
    lock.unlock()
    let object = try JSONSerialization.jsonObject(with: Data(message.text.utf8)) as! [String: Any]
    let id = object["id"] as! Int
    guard let reply = replies[id] ?? nil else { throw Down() }
    return reply
  }
}

let input = try JSONSerialization.jsonObject(with: Data(INPUT.utf8)) as! [String: Any]
var replies: [Int: String?] = [:]
for (key, value) in input["replies"] as! [String: Any] {
  replies[Int(key)!] = value as? String
}
let canned = Canned(replies: replies)
let client = MobileDocumentServiceMcpClient(transport: canned)
var lines: [String] = []

switch await client.getVersion(GetVersionRequest(documentId: "doc-1")) {
case .success(let version): lines.append("ok version=\(version.version)")
case .failure(let failure): lines.append(described(failure))
}
for id in ["missing", "", "panic", "doc-1", "doc-1", "doc-1"] {
  switch await client.getVersion(GetVersionRequest(documentId: id)) {
  case .success(let version): lines.append("ok version=\(version.version)")
  case .failure(let failure): lines.append(described(failure))
  }
}
switch await client.lookup(DocumentId(value: "doc-1")) {
case .success(let version): lines.append("ok version=\(version.version)")
case .failure(let failure):
  switch failure {
  case .declared: lines.append("declared not-found")
  case .fault(let fault): lines.append("fault \(fault.kind.rawValue)")
  }
}
switch await client.getTagged("doc-1", known_tag: "\"v6\"") {
case .success(let (version, etag)): lines.append("ok version=\(version.version) etag=\(etag)")
case .failure(let failure):
  switch failure {
  case .declared: lines.append("declared not-found")
  case .fault(let fault): lines.append("fault \(fault.kind.rawValue)")
  }
}
switch await client.getTagged("missing", known_tag: nil) {
case .success: lines.append("ok")
case .failure(let failure):
  switch failure {
  case .declared(let (_, reason)): lines.append("declared not-found reason=\(reason ?? "nil")")
  case .fault(let fault): lines.append("fault \(fault.kind.rawValue)")
  }
}
switch await client.renameDocument(RenameDocumentRequest(documentId: "doc-1", title: "New")) {
case .success: lines.append("ok unit")
case .failure(let failure):
  switch failure {
  case .declared: lines.append("declared")
  case .fault(let fault): lines.append("fault \(fault.kind.rawValue)")
  }
}
switch await client.archiveDocument(ArchiveDocumentRequest(documentId: "doc-1", reason: "stale"), exchange: "reviewer-7") {
case .success: lines.append("ok unit")
case .failure(let failure):
  switch failure {
  case .declared: lines.append("declared")
  case .fault(let fault): lines.append("fault \(fault.kind.rawValue)")
  }
}
try await client.touchDocument(TouchDocumentRequest(documentId: "doc-1", reader: "reviewer-7"))
lines.append("ok sent")
switch await client.tagDocument("t", arguments: "a", message: "m", outcome: "o", members: "s") {
case .success(let version): lines.append("ok version=\(version.version)")
case .failure(let failure):
  switch failure {
  case .declared: lines.append("declared")
  case .fault(let fault): lines.append("fault \(fault.kind.rawValue)")
  }
}
switch await client.labelDocument("l", sent: "s", next_id: "n", mcp_source: "m", carried: "c") {
case .success(let version): lines.append("ok version=\(version.version)")
case .failure(let failure):
  switch failure {
  case .declared: lines.append("declared")
  case .fault(let fault): lines.append("fault \(fault.kind.rawValue)")
  }
}
for _ in 0..<2 {
  do {
    try await client.touchDocument(TouchDocumentRequest(documentId: "doc-1", reader: "reviewer-7"))
    lines.append("ok sent")
  } catch let refusal as MobileDocumentServiceRefusal {
    lines.append("refused \(refusal.fault.kind.rawValue)")
  }
}

let sent = canned.sent
let report: [String: Any] = [
  "lines": lines,
  "sent": sent.map { $0.text },
  "names": sent.map { $0.name ?? "" },
  "methods": sent.map { $0.method },
  "versions": sent.map { $0.protocolVersion },
]
let data = try JSONSerialization.data(withJSONObject: report)
print(String(decoding: data, as: UTF8.self))
"#;

fn emitted() -> String {
    [
        "import Foundation".to_owned(),
        version_response_swift::swift_definition(),
        version_error_swift::swift_definition(),
        get_version_request_swift::swift_definition(),
        document_id_swift::swift_definition(),
        balance_response_swift::swift_definition(),
        balance_error_swift::swift_definition(),
        rename_document_request_swift::swift_definition(),
        touch_document_request_swift::swift_definition(),
        archive_document_request_swift::swift_definition(),
        mobile_document_service_schema::mobile_document_service_fault_fields_swift::swift_definition(),
        mobile_document_service_schema::mobile_document_service_fault_kind_swift::swift_definition(),
        MobileDocumentServiceSchema::swift_http_client(),
        MobileDocumentServiceSchema::swift_mcp_client(),
    ]
    .join("\n\n")
}

fn input() -> Value {
    let replies: serde_json::Map<String, Value> = client_replies()
        .into_iter()
        .map(|(number, answered)| {
            (
                number.to_string(),
                answered.map_or(Value::Null, Value::String),
            )
        })
        .collect();
    json!({ "replies": replies })
}

/// The report of the one Swift run; `None` where no Swift toolchain was reachable.
fn driven() -> Option<&'static Value> {
    static REPORT: OnceLock<Option<Value>> = OnceLock::new();
    REPORT
        .get_or_init(|| {
            let module = format!(
                "{}\n\nlet INPUT = \"{}\"\n{DRIVER}",
                emitted(),
                swift_text(&input().to_string())
            );
            let wrote = ran("swift-mcp", RUNTIME_VAR, "swift", "main.swift", &module)?;
            Some(serde_json::from_str(wrote.trim()).unwrap())
        })
        .as_ref()
}

#[test]
fn the_swift_client_reads_the_outcomes_the_design_lists_and_writes_the_requests_rust_reads() {
    let Some(report) = driven() else {
        return;
    };
    let lines: Vec<&str> = report["lines"]
        .as_array()
        .unwrap()
        .iter()
        .map(|line| line.as_str().unwrap())
        .collect();
    assert_eq!(lines, CLIENT_LINES);
    let sent = report["sent"].as_array().unwrap();
    assert_eq!(sent.len(), client_calls().len());
    for (at, text) in sent.iter().enumerate() {
        let number = u64::try_from(at).unwrap() + 1;
        assert_client_request(number, text.as_str().unwrap());
        assert_eq!(report["methods"][at], "tools/call");
        assert_eq!(report["versions"][at], "2026-07-28");
        assert_eq!(report["names"][at], client_calls()[at].tool);
    }
}
