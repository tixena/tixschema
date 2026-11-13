//! The emitted Swift `mcp` client run under a Swift toolchain on `JobService`: it reads the handle
//! and the state of a task as the ordinary successes the Rust dispatcher answers, and writes the
//! `tools/call` requests the Rust dispatcher reads.

#![cfg(feature = "swift")]

use super::JobServiceSchema;
use super::job_service_schema;
use super::mcp_mobile_rows::swift_text;
use super::mcp_task_rows::{
    answer_export_request_swift, assert_client, client_input, export_report_swift,
    export_request_swift, job_error_swift,
};
use super::runtime::ran;
use serde_json::Value;
use std::sync::OnceLock;

const RUNTIME_VAR: &str = "TIXSCHEMA_SWIFT";

const DRIVER: &str = r#"
final class Canned: JobServiceMcpTransport, @unchecked Sendable {
  private let lock = NSLock()
  private var recorded: [String] = []
  private var queue: [String]

  init(queue: [String]) { self.queue = queue }

  var sent: [String] {
    lock.lock()
    defer { lock.unlock() }
    return recorded
  }

  func send(_ message: JobServiceMcpMessage) async throws -> String {
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

func handle(_ task: McpTask) -> [String: Any] {
  var held = json(task) as! [String: Any]
  held.removeValue(forKey: "status")
  return held
}

func state(_ value: McpTaskState<ExportReport>) -> [String: Any] {
  var held = handle(value.task)
  switch value {
  case .working: held["status"] = "working"
  case .inputRequired(_, let requests):
    held["status"] = "input_required"
    held["inputRequests"] = json(requests)
  case .completed(_, let result):
    held["status"] = "completed"
    held["result"] = json(result)
  case .failed(_, let error):
    held["status"] = "failed"
    held["error"] = json(error)
  case .cancelled: held["status"] = "cancelled"
  }
  return held
}

func started(_ result: Result<McpTask, JobServiceStartExportFailure>) -> [String: Any] {
  switch result {
  case .success(let task): return ["ok": handle(task)]
  case .failure(.declared(let error)): return ["declared": json(error)]
  case .failure(.fault(let fault)): return ["fault": fault.kind.rawValue]
  }
}

func statused(_ result: Result<McpTaskState<ExportReport>, JobServiceExportStatusFailure>) -> [String: Any] {
  switch result {
  case .success(let value): return ["ok": state(value)]
  case .failure(.declared(let error)): return ["declared": json(error)]
  case .failure(.fault(let fault)): return ["fault": fault.kind.rawValue]
  }
}

func cancelled(_ result: Result<Void, JobServiceCancelExportFailure>) -> [String: Any] {
  switch result {
  case .success: return ["ok": NSNull()]
  case .failure(.declared(let error)): return ["declared": json(error)]
  case .failure(.fault(let fault)): return ["fault": fault.kind.rawValue]
  }
}

func answered(_ result: Result<Void, JobServiceAnswerExportFailure>) -> [String: Any] {
  switch result {
  case .success: return ["ok": NSNull()]
  case .failure(.declared(let error)): return ["declared": json(error)]
  case .failure(.fault(let fault)): return ["fault": fault.kind.rawValue]
  }
}

let input = try JSONSerialization.jsonObject(with: Data(INPUT.utf8)) as! [String: Any]
let calls = input["calls"] as! [[String: Any]]
let canned = Canned(queue: calls.map { $0["answer"] as! String })
let client = JobServiceMcpClient(transport: canned)
var outcomes: [String: Any] = [:]
for call in calls {
  let label = call["label"] as! String
  switch label {
  case "started": outcomes[label] = started(await client.startExport(try ExportRequestDraft(folder: "reports").validate()))
  case "working": outcomes[label] = statused(await client.exportStatus("7f3a"))
  case "completed": outcomes[label] = statused(await client.exportStatus("done"))
  case "waiting": outcomes[label] = statused(await client.exportStatus("needs"))
  case "failed": outcomes[label] = statused(await client.exportStatus("broken"))
  case "cancelled": outcomes[label] = statused(await client.exportStatus("stopped"))
  case "cancel": outcomes[label] = cancelled(await client.cancelExport("7f3a"))
  case "answer": outcomes[label] = answered(await client.answerExport(AnswerExportRequest(taskId: "needs", key: "overwrite", action: "accept", content: "{}")))
  case "declared": outcomes[label] = statused(await client.exportStatus("gone"))
  case "panicked": outcomes[label] = statused(await client.exportStatus("boom"))
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
        job_error_swift::swift_definition(),
        export_request_swift::swift_definition(),
        export_report_swift::swift_definition(),
        answer_export_request_swift::swift_definition(),
        job_service_schema::job_service_fault_fields_swift::swift_definition(),
        job_service_schema::job_service_fault_kind_swift::swift_definition(),
        JobServiceSchema::swift_http_client(),
        JobServiceSchema::swift_mcp_content(),
        JobServiceSchema::swift_mcp_client(),
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
                "swift-mcp-tasks",
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
fn the_swift_client_reads_the_handle_and_the_state_the_rust_dispatcher_answers() {
    let Some(report) = driven() else {
        return;
    };
    assert_client(report);
}
