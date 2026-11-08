//! The emitted Swift `mcp` client run under a Swift toolchain on `AskingService`: it reads the
//! blocks and the arm the Rust dispatcher answers, and writes the requests the Rust dispatcher
//! reads, the preview service's pair among them.

#![cfg(feature = "swift")]

use super::mcp_asking_rows::{
    asking_confirmation_swift, asking_error_swift, asking_kind_error_swift,
    asking_link_error_swift, asking_linked_swift, asking_miss_swift, asking_priority_swift,
    asking_renamed_error_swift, asking_report_request_swift, asking_shipping_swift, assert_client,
    client_input, delete_document_request_swift, ship_document_request_swift,
};
use super::mcp_mobile_rows::swift_text;
use super::runtime::ran;
use super::{AskingServiceSchema, PreviewServiceSchema};
use super::{asking_service_schema, preview_service_schema};
use serde_json::{Value, json};
use std::sync::OnceLock;

const RUNTIME_VAR: &str = "TIXSCHEMA_SWIFT";

const DRIVER: &str = r#"
final class Canned: AskingServiceMcpTransport, PreviewServiceMcpTransport, @unchecked Sendable {
  private let lock = NSLock()
  private var recorded: [String] = []
  private var queue: [String]

  init(queue: [String]) { self.queue = queue }

  var sent: [String] {
    lock.lock()
    defer { lock.unlock() }
    return recorded
  }

  func send(_ message: AskingServiceMcpMessage) async throws -> String {
    lock.lock()
    defer { lock.unlock() }
    recorded.append(message.text)
    return queue.removeFirst()
  }

  func send(_ message: PreviewServiceMcpMessage) async throws -> String {
    lock.lock()
    defer { lock.unlock() }
    recorded.append(message.text)
    return queue.removeFirst()
  }
}

func json<Value: Encodable>(_ value: Value) -> Any {
  let data = try! JSONEncoder().encode(value)
  return try! JSONSerialization.jsonObject(with: data, options: [.fragmentsAllowed])
}

func deleted(_ result: Result<Void, AskingServiceDeleteDocumentFailure>) -> [String: Any] {
  switch result {
  case .success: return ["ok": NSNull()]
  case .failure(.declared(let error)): return ["declared": json(error)]
  case .failure(.fault(let fault)): return ["fault": fault.kind.rawValue]
  }
}

func previewed(_ result: Result<([UInt8], String), PreviewServiceDocumentPreviewFailure>) -> [String: Any] {
  switch result {
  case .success(let pair): return ["ok": ["bytes": pair.0.map { Int($0) }, "contentType": pair.1] as [String: Any]]
  case .failure(.declared(let error)): return ["declared": json(error)]
  case .failure(.fault(let fault)): return ["fault": fault.kind.rawValue]
  }
}

func reported(_ result: Result<[McpContent], AskingServiceBuildReportFailure>) -> [String: Any] {
  switch result {
  case .success(let blocks): return ["ok": blocks.map { json($0) }]
  case .failure(.declared(let error)): return ["declared": json(error)]
  case .failure(.fault(let fault)): return ["fault": fault.kind.rawValue]
  }
}

func imported(_ result: Result<AskingLinked, AskingServiceImportFromDriveFailure>) -> [String: Any] {
  switch result {
  case .success(let linked): return ["ok": json(linked)]
  case .failure(.declared(let error)): return ["declared": json(error)]
  case .failure(.fault(let fault)): return ["fault": fault.kind.rawValue]
  }
}

let input = try JSONSerialization.jsonObject(with: Data(INPUT.utf8)) as! [String: Any]
let calls = input["calls"] as! [[String: Any]]
let canned = Canned(queue: calls.map { $0["answer"] as! String })
let client = AskingServiceMcpClient(transport: canned)
let previewer = PreviewServiceMcpClient(transport: canned)
var outcomes: [String: Any] = [:]
for call in calls {
  let label = call["label"] as! String
  switch label {
  case "declared": outcomes[label] = deleted(await client.deleteDocument(DeleteDocumentRequest(documentId: "doc-1", confirmation: nil)))
  case "confirmed": outcomes[label] = deleted(await client.deleteDocument(DeleteDocumentRequest(documentId: "doc-1", confirmation: AskingConfirmation(confirm: true))))
  case "image": outcomes[label] = previewed(await previewer.documentPreview("doc-1"))
  case "audio": outcomes[label] = previewed(await previewer.documentPreview("voice"))
  case "plain": outcomes[label] = previewed(await previewer.documentPreview("plain"))
  case "missing": outcomes[label] = previewed(await previewer.documentPreview("nothing"))
  case "report": outcomes[label] = reported(await client.buildReport(AskingReportRequest(title: "Q3")))
  case "link": outcomes[label] = imported(await client.importFromDrive("1A2b3C"))
  case "linked": outcomes[label] = imported(await client.importFromDrive("connected"))
  default: fatalError("no call is made for \(label)")
  }
}

struct ReplyingHttp: PreviewServiceHttpTransport {
  func send(_ request: PreviewServiceHttpRequest) async throws -> PreviewServiceHttpResponse {
    PreviewServiceHttpResponse(status: 200, headers: [], body: Data("[[37,80],\"application/pdf\"]".utf8))
  }
}

final class ReplyingSocket: PreviewServiceWsSocket, @unchecked Sendable {
  var onMessage: (@Sendable (String) -> Void)?
  var onClose: (@Sendable () -> Void)?

  func send(_ text: String) {
    let id = (try! JSONSerialization.jsonObject(with: Data(text.utf8)) as! [String: Any])["id"] as! String
    onMessage?("{\"kind\":\"reply\",\"id\":\"\(id)\",\"service\":\"PreviewService\",\"ok\":true,\"value\":[[37,80],\"application/pdf\"]}")
  }

  func close() { onClose?() }
}

let overHttp = previewed(await PreviewServiceHttpClient(transport: ReplyingHttp()).documentPreview("plain"))
let overSocket = previewed(await PreviewServiceWsTransport(socket: ReplyingSocket(), options: .init(heartbeat: nil)).documentPreview("plain"))

let report: [String: Any] = [
  "http": overHttp,
  "socket": overSocket,
  "outcomes": outcomes,
  "seen": canned.sent.map { try! JSONSerialization.jsonObject(with: Data($0.utf8)) },
]
let data = try JSONSerialization.data(withJSONObject: report)
print(String(decoding: data, as: UTF8.self))
"#;

fn emitted() -> String {
    [
        "import Foundation".to_owned(),
        asking_priority_swift::swift_definition(),
        asking_shipping_swift::swift_definition(),
        asking_confirmation_swift::swift_definition(),
        asking_error_swift::swift_definition(),
        asking_link_error_swift::swift_definition(),
        asking_renamed_error_swift::swift_definition(),
        asking_kind_error_swift::swift_definition(),
        asking_miss_swift::swift_definition(),
        asking_report_request_swift::swift_definition(),
        asking_linked_swift::swift_definition(),
        delete_document_request_swift::swift_definition(),
        ship_document_request_swift::swift_definition(),
        asking_service_schema::asking_service_fault_fields_swift::swift_definition(),
        asking_service_schema::asking_service_fault_kind_swift::swift_definition(),
        AskingServiceSchema::swift_http_client(),
        AskingServiceSchema::swift_mcp_content(),
        AskingServiceSchema::swift_mcp_client(),
        preview_service_schema::preview_service_fault_fields_swift::swift_definition(),
        preview_service_schema::preview_service_fault_kind_swift::swift_definition(),
        PreviewServiceSchema::swift_http_client(),
        PreviewServiceSchema::swift_ws_client(),
        PreviewServiceSchema::swift_mcp_client(),
    ]
    .join("\n\n")
}

/// The report of the one Swift run; `None` where no Swift toolchain was reachable.
fn driven() -> Option<&'static Value> {
    static REPORT: OnceLock<Option<Value>> = OnceLock::new();
    REPORT
        .get_or_init(|| {
            let module = format!(
                "{}\n\nlet INPUT = \"{}\"\n{DRIVER}",
                emitted(),
                swift_text(&client_input().to_string())
            );
            let wrote = ran(
                "swift-mcp-asking",
                RUNTIME_VAR,
                "swift",
                "main.swift",
                &module,
            )?;
            Some(serde_json::from_str(wrote.trim()).unwrap())
        })
        .as_ref()
}

#[test]
fn the_swift_client_reads_the_blocks_and_the_arm_the_rust_dispatcher_answers() {
    let Some(report) = driven() else {
        return;
    };
    assert_client(report);
    let pair = json!({ "ok": { "bytes": [37_u8, 80_u8], "contentType": "application/pdf" } });
    assert_eq!(report["http"], pair);
    assert_eq!(report["socket"], pair);
}
