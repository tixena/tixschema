//! The emitted Swift `http_rest` client, compiled with warnings as errors, handed each value of
//! [`WIRE_FORMS`](super::wire_forms_models::WIRE_FORMS) in a path, a query, a header and a part:
//! each call prints the request the transport was handed, or `refused` where the client refused
//! the value before the transport was reached, and the Rust dispatcher reads each request.

#![cfg(all(feature = "swift", feature = "chrono"))]

use super::run_swift_wire_forms::swift_value;
use super::runtime::judged;
use super::text_position_rows::text_placing_client_service_schema::{
    text_placing_client_service_fault_fields_swift, text_placing_client_service_fault_kind_swift,
};
use super::text_position_rows::{
    Call, Position, TextPlacingClientServiceSchema, calls, held_to_the_table, messages_swift,
    spelled, text_placing_error_swift,
};
use super::wire_forms_models::conveyed_slot_swift;

/// A transport that records the request it is handed and answers the empty text, and the one
/// line each call prints: that request, or `refused` where none reached the transport.
const RECORDER: &str = r#"
struct Sent: Codable {
  let method: String
  let path: String
  let query: String
  let headers: [[String]]
  let body: String
  let parts: [[String]]
}

actor Recorder: TextPlacingClientServiceHttpTransport {
  private(set) var recorded: Sent?

  func clear() {
    recorded = nil
  }

  func send(_ request: TextPlacingClientServiceHttpRequest) async throws -> TextPlacingClientServiceHttpResponse {
    recorded = Sent(
      method: request.method,
      path: request.path,
      query: request.query,
      headers: request.headers.map { [$0.0, $0.1] },
      body: String(decoding: request.body, as: UTF8.self),
      parts: request.parts.map { [$0.0, "\($0.1)"] }
    )
    return TextPlacingClientServiceHttpResponse(status: 200, headers: [], body: Data("\"\"".utf8))
  }
}

func printed(_ recorder: Recorder) async {
  if let sent = await recorder.recorded {
    print(String(decoding: try! JSONEncoder().encode(sent), as: UTF8.self))
  } else {
    print("\"refused\"")
  }
}

let recorder = Recorder()
let client = TextPlacingClientServiceHttpClient(transport: recorder)
"#;

/// The statements that make `call` through the client and print what it sent.
fn statement(call: &Call) -> String {
    let arguments = spelled(call, &swift_value, "nil");
    let labelled = call
        .arguments()
        .iter()
        .zip(&arguments)
        .map(|((argument, _, _), value)| format!("{argument}: {value}"))
        .collect::<Vec<_>>()
        .join(", ");
    let operation = call.operation();
    let made = if call.position == Position::Header {
        format!("client.{operation}(\"n\", {labelled})")
    } else {
        format!("client.{operation}({}({labelled}))", call.message())
    };
    format!("await recorder.clear()\n_ = await {made}\nawait printed(recorder)")
}

/// Each value the table's Swift column hands the client is written in a path, a query, a header
/// and a part in the form the table states, and refused where the table states it.
#[test]
fn each_value_is_written_as_text_by_the_swift_client_in_the_form_the_dispatcher_reads() {
    let made: Vec<String> = calls(|row| row.swift).iter().map(statement).collect();
    let source = [
        "import Foundation".to_owned(),
        conveyed_slot_swift::swift_definition(),
        text_placing_error_swift::swift_definition(),
        messages_swift(),
        text_placing_client_service_fault_fields_swift::swift_definition(),
        text_placing_client_service_fault_kind_swift::swift_definition(),
        TextPlacingClientServiceSchema::swift_http_client(),
        RECORDER.to_owned(),
        made.join("\n"),
    ]
    .join("\n\n");
    let Some(run) = judged(
        "swift-text-positions",
        "TIXSCHEMA_SWIFT",
        "swift",
        &["-warnings-as-errors"],
        "main.swift",
        &source,
    ) else {
        return;
    };
    assert!(
        run.succeeded,
        "`swift -warnings-as-errors main.swift` failed.\n--- stdout ---\n{}\n--- stderr ---\n{}",
        run.stdout, run.stderr
    );
    held_to_the_table(|row| row.swift, &run.stdout);
}
