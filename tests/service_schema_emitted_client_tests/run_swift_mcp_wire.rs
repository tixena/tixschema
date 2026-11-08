//! The emitted Swift `http_rest` and `ws_rpc` clients of `ShelfWireService`, run under a Swift
//! toolchain: an operation answering content blocks or prompt messages is read in the block form
//! the Rust dispatchers write.

#![cfg(feature = "swift")]

use super::ShelfWireServiceSchema;
use super::mcp_mobile_rows::swift_text;
use super::mcp_wire_rust::{BLOCKS, MESSAGES, PAGE, over_http, over_ws};
use super::mcp_wire_service::{shelf_wire_error_swift, shelf_wire_request_swift};
use super::runtime::ran;
use super::shelf_wire_service_schema;
use serde_json::{Value, json};

const RUNTIME_VAR: &str = "TIXSCHEMA_SWIFT";

const DRIVER: &str = r#"
struct Canned: ShelfWireServiceHttpTransport {
  func send(_ request: ShelfWireServiceHttpRequest) async throws -> ShelfWireServiceHttpResponse {
    ShelfWireServiceHttpResponse(status: 200, headers: [], body: Data(RUST_HTTP[request.path]!.utf8))
  }
}

final class Relay: ShelfWireServiceWsSocket, @unchecked Sendable {
  var onMessage: (@Sendable (String) -> Void)?
  var onClose: (@Sendable () -> Void)?

  func send(_ text: String) {
    let sent = try! JSONSerialization.jsonObject(with: Data(text.utf8)) as! [String: Any]
    var reply = try! JSONSerialization.jsonObject(with: Data(RUST_REPLIES[sent["operation"] as! String]!.utf8)) as! [String: Any]
    reply["id"] = sent["id"]
    let data = try! JSONSerialization.data(withJSONObject: reply)
    onMessage?(String(decoding: data, as: UTF8.self))
  }

  func close() {}
}

func json<Value: Encodable>(_ value: Value) -> Any {
  let data = try! JSONEncoder().encode(value)
  return try! JSONSerialization.jsonObject(with: data, options: [.fragmentsAllowed])
}

func blocks(_ result: Result<[McpContent], ShelfWireServiceDescribeShelfFailure>) -> Any {
  switch result {
  case .success(let value): return json(value)
  case .failure: return "other"
  }
}

func page(_ result: Result<ShelfWireServiceMcpResourcePage, ShelfWireServiceListShelvesFailure>) -> Any {
  switch result {
  case .success(let value): return json(value)
  case .failure: return "other"
  }
}

func messages(_ result: Result<[McpPromptMessage], ShelfWireServiceReviewShelfFailure>) -> Any {
  switch result {
  case .success(let value): return json(value)
  case .failure: return "other"
  }
}

let overHttp = ShelfWireServiceHttpClient(transport: Canned())
let httpDescribed = blocks(await overHttp.describeShelf(ShelfWireRequest(shelf: "history")))
let httpReviewed = messages(await overHttp.reviewShelf("history"))
let httpListed = page(await overHttp.listShelves(nil))

let overWs = ShelfWireServiceWsTransport(socket: Relay(), options: .init(heartbeat: nil))
let wsDescribed = blocks(await overWs.describeShelf(ShelfWireRequest(shelf: "history")))
let wsReviewed = messages(await overWs.reviewShelf("history"))
let wsListed = page(await overWs.listShelves(nil))

let report: [String: Any] = ["http": [httpDescribed, httpReviewed, httpListed], "ws": [wsDescribed, wsReviewed, wsListed]]
let data = try JSONSerialization.data(withJSONObject: report)
print(String(decoding: data, as: UTF8.self))
"#;

fn emitted() -> String {
    [
        "import Foundation".to_owned(),
        shelf_wire_error_swift::swift_definition(),
        shelf_wire_request_swift::swift_definition(),
        shelf_wire_service_schema::shelf_wire_service_fault_fields_swift::swift_definition(),
        shelf_wire_service_schema::shelf_wire_service_fault_kind_swift::swift_definition(),
        ShelfWireServiceSchema::swift_http_client(),
        ShelfWireServiceSchema::swift_ws_client(),
        ShelfWireServiceSchema::swift_mcp_content(),
    ]
    .join("\n\n")
}

fn declared() -> String {
    let history = json!({ "shelf": "history" });
    let (_, describe_reply) = over_ws("describe-shelf", &history);
    let (_, review_reply) = over_ws("review-shelf", &json!("history"));
    let (_, list_reply) = over_ws("list-shelves", &Value::Null);
    let (_, list_body) = over_http("/list-shelves", &Value::Null);
    let (_, describe_body) = over_http("/describe-shelf", &history);
    let (_, review_body) = over_http("/review-shelf", &json!("history"));
    let text = |held: &str| format!("\"{}\"", swift_text(held));
    format!(
        "let RUST_HTTP: [String: String] = [\"/describe-shelf\": {}, \"/review-shelf\": {}, \"/list-shelves\": {}]\n\
         let RUST_REPLIES: [String: String] = [\"describe-shelf\": {}, \"review-shelf\": {}, \"list-shelves\": {}]",
        text(&describe_body),
        text(&review_body),
        text(&list_body),
        text(&describe_reply),
        text(&review_reply),
        text(&list_reply),
    )
}

#[test]
fn the_swift_clients_read_the_block_form_the_rust_dispatchers_write() {
    let module = format!("{}\n\n{}\n{DRIVER}", emitted(), declared());
    let Some(wrote) = ran(
        "swift-mcp-wire",
        RUNTIME_VAR,
        "swift",
        "main.swift",
        &module,
    ) else {
        return;
    };
    let report: Value = serde_json::from_str(wrote.trim()).unwrap();
    let blocks: Value = serde_json::from_str(BLOCKS).unwrap();
    let messages: Value = serde_json::from_str(MESSAGES).unwrap();
    let page: Value = serde_json::from_str(PAGE).unwrap();
    for transport in ["http", "ws"] {
        assert_eq!(report[transport][0], blocks, "{transport}");
        assert_eq!(report[transport][1], messages, "{transport}");
        assert_eq!(report[transport][2], page, "{transport}");
    }
}
