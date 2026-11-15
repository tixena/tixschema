//! The `ws_rpc` client of `HoistService` run by a real Swift toolchain: a reply holding a value
//! declared above the service, below it or in another crate is refused with one issue per broken
//! rule, each at its own field, as Rust's own read lists them. Swift has no `ws_rpc` attachment.

#![cfg(feature = "swift")]

use super::refusal_order_service::{
    BROKEN_LOAD, HoistServiceSchema, OPERATIONS, above_item_swift, above_load_swift,
    below_item_swift, below_load_swift, below_renamed_swift, broken_issues, hoist_error_swift,
    hoist_service_schema, shipment_mcp_task_swift,
};
use super::runtime::ran;
use refusal_order_apart::{apart_item_swift, apart_load_swift, apart_renamed_swift};
use serde_json::{Value, json};

const RUNTIME_VAR: &str = "TIXSCHEMA_SWIFT";

/// Hands the client of each operation a reply holding a broken value.
const DRIVER: &str = r#"
final class RelaySocket: HoistServiceWsSocket, @unchecked Sendable {
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

  func close() { onClose?() }

  func deliver(_ text: String) { onMessage?(text) }
}

struct FrameProbe: Decodable { let id: String? }

struct IssueReport: Codable {
  let detail: String
  let field: String?
}

struct FaultReport: Codable {
  let kind: String
  let issues: [IssueReport]
}

func requestId(from sent: String?) -> String? {
  guard let sent, let data = sent.data(using: .utf8),
        let frame = try? JSONDecoder().decode(FrameProbe.self, from: data)
  else { return nil }
  return frame.id
}

func reply(_ id: String) -> String {
  "{\"kind\":\"reply\",\"id\":\"\(id)\",\"service\":\"HoistService\",\"ok\":true,\"value\":\(broken)}"
}

func main() async {
  var results: [String: FaultReport] = [:]

  let aboveSocket = RelaySocket()
  let aboveTransport = HoistServiceWsTransport(socket: aboveSocket, options: .init(heartbeat: nil))
  async let above = aboveTransport.above(try! AboveLoad.of(items: [], label: "a", weight: 1))
  try? await Task.sleep(nanoseconds: 30_000_000)
  aboveSocket.deliver(reply(requestId(from: aboveSocket.sent.last)!))
  if case .failure(.fault(let fault)) = await above {
    results["above"] = FaultReport(kind: fault.kind.rawValue, issues: (fault.issues ?? []).map { IssueReport(detail: $0.detail, field: $0.field) })
  }

  let apartSocket = RelaySocket()
  let apartTransport = HoistServiceWsTransport(socket: apartSocket, options: .init(heartbeat: nil))
  async let apart = apartTransport.apart(try! ApartLoad.of(items: [], label: "a", weight: 1))
  try? await Task.sleep(nanoseconds: 30_000_000)
  apartSocket.deliver(reply(requestId(from: apartSocket.sent.last)!))
  if case .failure(.fault(let fault)) = await apart {
    results["apart"] = FaultReport(kind: fault.kind.rawValue, issues: (fault.issues ?? []).map { IssueReport(detail: $0.detail, field: $0.field) })
  }

  let belowSocket = RelaySocket()
  let belowTransport = HoistServiceWsTransport(socket: belowSocket, options: .init(heartbeat: nil))
  async let below = belowTransport.below(try! BelowLoad.of(items: [], label: "a", weight: 1))
  try? await Task.sleep(nanoseconds: 30_000_000)
  belowSocket.deliver(reply(requestId(from: belowSocket.sent.last)!))
  if case .failure(.fault(let fault)) = await below {
    results["below"] = FaultReport(kind: fault.kind.rawValue, issues: (fault.issues ?? []).map { IssueReport(detail: $0.detail, field: $0.field) })
  }

  let movedSocket = RelaySocket()
  let movedTransport = HoistServiceWsTransport(socket: movedSocket, options: .init(heartbeat: nil))
  async let moved = movedTransport.moved(try! PublishedBelow.of(items: [], label: "a", weight: 1))
  try? await Task.sleep(nanoseconds: 30_000_000)
  movedSocket.deliver(reply(requestId(from: movedSocket.sent.last)!))
  if case .failure(.fault(let fault)) = await moved {
    results["moved"] = FaultReport(kind: fault.kind.rawValue, issues: (fault.issues ?? []).map { IssueReport(detail: $0.detail, field: $0.field) })
  }
  let shiftedSocket = RelaySocket()
  let shiftedTransport = HoistServiceWsTransport(socket: shiftedSocket, options: .init(heartbeat: nil))
  async let shifted = shiftedTransport.shifted(try! PublishedApart.of(items: [], label: "a", weight: 1))
  try? await Task.sleep(nanoseconds: 30_000_000)
  shiftedSocket.deliver(reply(requestId(from: shiftedSocket.sent.last)!))
  if case .failure(.fault(let fault)) = await shifted {
    results["shifted"] = FaultReport(kind: fault.kind.rawValue, issues: (fault.issues ?? []).map { IssueReport(detail: $0.detail, field: $0.field) })
  }
  let taskedSocket = RelaySocket()
  let taskedTransport = HoistServiceWsTransport(socket: taskedSocket, options: .init(heartbeat: nil))
  async let tasked = taskedTransport.tasked(try! ShipmentMcpTask.of(items: [], label: "a", weight: 1))
  try? await Task.sleep(nanoseconds: 30_000_000)
  taskedSocket.deliver(reply(requestId(from: taskedSocket.sent.last)!))
  if case .failure(.fault(let fault)) = await tasked {
    results["tasked"] = FaultReport(kind: fault.kind.rawValue, issues: (fault.issues ?? []).map { IssueReport(detail: $0.detail, field: $0.field) })
  }

  print(String(data: try! JSONEncoder().encode(results), encoding: .utf8)!)
}
await main()
"#;

/// Every definition, the data the driver reads, and the driver.
fn module() -> String {
    [
        "import Foundation".to_owned(),
        above_item_swift::swift_definition(),
        above_load_swift::swift_definition(),
        apart_item_swift::swift_definition(),
        apart_load_swift::swift_definition(),
        below_item_swift::swift_definition(),
        below_load_swift::swift_definition(),
        below_renamed_swift::swift_definition(),
        apart_renamed_swift::swift_definition(),
        shipment_mcp_task_swift::swift_definition(),
        hoist_error_swift::swift_definition(),
        hoist_service_schema::hoist_service_fault_fields_swift::swift_definition(),
        hoist_service_schema::hoist_service_fault_kind_swift::swift_definition(),
        HoistServiceSchema::swift_http_client(),
        HoistServiceSchema::swift_ws_client(),
        format!("let broken = #\"{BROKEN_LOAD}\"#"),
        DRIVER.to_owned(),
    ]
    .join("\n\n")
}

#[test]
fn a_value_declared_above_below_or_in_another_crate_is_listed_issue_by_issue() {
    let Some(wrote) = ran(
        "swift-refusal-order",
        RUNTIME_VAR,
        "swift",
        "main.swift",
        &module(),
    ) else {
        return;
    };
    let told: Value = serde_json::from_str(wrote.trim()).unwrap();
    let expected: Vec<Value> = broken_issues()
        .into_iter()
        .map(|(field, detail)| json!({ "detail": detail, "field": field }))
        .collect();
    for operation in OPERATIONS {
        assert_eq!(told[operation]["kind"], "failed-validation", "{told}");
        assert_eq!(
            told[operation]["issues"],
            Value::Array(expected.clone()),
            "{operation}: {told}"
        );
    }
}
