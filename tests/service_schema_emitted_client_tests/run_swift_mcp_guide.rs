//! The emitted Swift `mcp` client run under a Swift toolchain on `GuideService`: it reads the
//! text, the messages and the declared error the Rust dispatcher answers, and writes the
//! `prompts/get` requests the Rust dispatcher reads.

#![cfg(feature = "swift")]

use super::GuideServiceSchema;
use super::guide_service_schema;
use super::mcp_guide_rows::{
    assert_client, client_input, configure_organization_request_swift,
    get_guide_version_request_swift, guide_error_swift, guide_version_swift,
    review_document_type_request_swift, suggest_document_types_request_swift,
};
use super::mcp_mobile_rows::swift_text;
use super::runtime::ran;
use serde_json::Value;
use std::sync::OnceLock;

const RUNTIME_VAR: &str = "TIXSCHEMA_SWIFT";

const DRIVER: &str = r#"
final class Canned: GuideServiceMcpTransport, @unchecked Sendable {
  private let lock = NSLock()
  private var recorded: [String] = []
  private var queue: [String]

  init(queue: [String]) { self.queue = queue }

  var sent: [String] {
    lock.lock()
    defer { lock.unlock() }
    return recorded
  }

  func send(_ message: GuideServiceMcpMessage) async throws -> String {
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

func greeted(_ result: Result<String, GuideServiceGreetFailure>) -> [String: Any] {
  switch result {
  case .success(let text): return ["ok": text]
  case .failure(.declared(let error)): return ["declared": json(error)]
  case .failure(.fault(let fault)): return ["fault": fault.kind.rawValue]
  }
}

func configured(_ result: Result<String, GuideServiceConfigureOrganizationFailure>) -> [String: Any] {
  switch result {
  case .success(let text): return ["ok": text]
  case .failure(.declared(let error)): return ["declared": json(error)]
  case .failure(.fault(let fault)): return ["fault": fault.kind.rawValue]
  }
}

func reviewed(_ result: Result<[McpPromptMessage], GuideServiceReviewDocumentTypeFailure>) -> [String: Any] {
  switch result {
  case .success(let messages): return ["ok": messages.map { json($0) }]
  case .failure(.declared(let error)): return ["declared": json(error)]
  case .failure(.fault(let fault)): return ["fault": fault.kind.rawValue]
  }
}

let input = try JSONSerialization.jsonObject(with: Data(INPUT.utf8)) as! [String: Any]
let calls = input["calls"] as! [[String: Any]]
let canned = Canned(queue: calls.map { $0["answer"] as! String })
let client = GuideServiceMcpClient(transport: canned)
var outcomes: [String: Any] = [:]
for call in calls {
  let label = call["label"] as! String
  switch label {
  case "string": outcomes[label] = configured(await client.configureOrganization(ConfigureOrganizationRequest()))
  case "named": outcomes[label] = greeted(await client.greet("Ana"))
  case "messages": outcomes[label] = reviewed(await client.reviewDocumentType(ReviewDocumentTypeRequest(documentType: "ID card", focus: nil)))
  case "declared": outcomes[label] = greeted(await client.greet("missing"))
  case "panicked": outcomes[label] = greeted(await client.greet("boom"))
  case "unknown": outcomes[label] = greeted(await client.greet("nobody"))
  case "unreadable": outcomes[label] = reviewed(await client.reviewDocumentType(ReviewDocumentTypeRequest(documentType: "ID card", focus: "names")))
  default: fatalError("no call is made for \(label)")
  }
}

let report: [String: Any] = [
  "outcomes": outcomes,
  "seen": canned.sent.map { try! JSONSerialization.jsonObject(with: Data($0.utf8)) },
]
let data = try JSONSerialization.data(withJSONObject: report)
print(String(decoding: data, as: UTF8.self))
"#;

fn emitted() -> String {
    [
        "import Foundation".to_owned(),
        guide_error_swift::swift_definition(),
        guide_version_swift::swift_definition(),
        configure_organization_request_swift::swift_definition(),
        get_guide_version_request_swift::swift_definition(),
        review_document_type_request_swift::swift_definition(),
        suggest_document_types_request_swift::swift_definition(),
        guide_service_schema::guide_service_fault_fields_swift::swift_definition(),
        guide_service_schema::guide_service_fault_kind_swift::swift_definition(),
        GuideServiceSchema::swift_http_client(),
        GuideServiceSchema::swift_mcp_content(),
        GuideServiceSchema::swift_mcp_client(),
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
                "swift-mcp-guide",
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
fn the_swift_client_reads_the_text_the_messages_and_the_error_the_rust_dispatcher_answers() {
    let Some(report) = driven() else {
        return;
    };
    assert_client(report);
}
