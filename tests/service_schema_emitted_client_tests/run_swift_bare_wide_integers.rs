//! The emitted Swift `http_rest` client, compiled with warnings as errors, handed a bare `i64` as
//! a message, a header and a list, and answered a reply, a header beside it and a declared error
//! of one: each call prints the request the transport was handed and what the client answered.
//! The `ws_rpc` client compiles beside it.

#![cfg(all(feature = "swift", feature = "serde"))]

use core::fmt::Write as _;

use super::bare_wide_integer_rows::{BARE_EXCHANGES, bare_printed};
use super::run_node_wide_integers::tallying_bare_service_schema::{
    tallying_bare_service_fault_fields_swift, tallying_bare_service_fault_kind_swift,
};
use super::run_node_wide_integers::{TallyingBareServiceSchema, tally_error_swift};
use super::runtime::judged;

/// A transport that prints each request it is handed and answers the next response of
/// `responses`, and what prints each answer.
const RECORDER: &str = r#"
actor Recorder: TallyingBareServiceHttpTransport {
  private let responses: [TallyingBareServiceHttpResponse]
  private var at = 0

  init(_ responses: [TallyingBareServiceHttpResponse]) {
    self.responses = responses
  }

  func send(_ request: TallyingBareServiceHttpRequest) async throws -> TallyingBareServiceHttpResponse {
    let headed = request.headers.map { "\($0.0)=\($0.1)" }.joined(separator: ",")
    print("\(request.method) \(request.path) \(headed) \(String(decoding: request.body, as: UTF8.self))")
    let answered = responses[at]
    at += 1
    return answered
  }
}

func shownShift(_ result: Result<(Int64, Int64?), TallyingBareServiceShiftFailure>) -> String {
  switch result {
  case .success(let value): return "ok \(value.0) \(value.1.map { "\($0)" } ?? "null")"
  case .failure(.declared(let error)): return "declared \(error)"
  case .failure(.fault(let fault)): return "fault \(fault.kind.rawValue)"
  }
}

func shownTotal(_ result: Result<Int64, TallyingBareServiceTotalFailure>) -> String {
  switch result {
  case .success(let value): return "ok \(value)"
  case .failure(.declared(let error)): return "declared \(error)"
  case .failure(.fault(let fault)): return "fault \(fault.kind.rawValue)"
  }
}
"#;

/// Each call hands its transport the request the Rust client sends of it, and reads each response
/// the Rust dispatcher answers back into the integers.
#[test]
fn a_bare_i64_travels_as_a_number_through_the_swift_client() {
    let mut responses = String::new();
    for exchange in &BARE_EXCHANGES {
        let headers: Vec<String> = exchange
            .headers
            .iter()
            .map(|(name, value)| format!("(\"{name}\", \"{value}\")"))
            .collect();
        let _ = write!(
            responses,
            "TallyingBareServiceHttpResponse(status: {}, headers: [{}], body: Data(#\"{}\"#.utf8)), ",
            exchange.status,
            headers.join(", "),
            exchange.body
        );
    }
    let main = format!(
        "let client = TallyingBareServiceHttpClient(transport: Recorder([{responses}]))\n\
         print(shownShift(await client.shift(9223372036854775806, offset: 1)))\n\
         print(shownShift(await client.shift(4611686018427387903, offset: nil)))\n\
         print(shownTotal(await client.total([9223372036854775807, 1])))\n\
         try await client.record(Int64.min)\n\
         print(\"sent\")\n\
         print(shownTotal(await client.total([1])))\n"
    );
    let source = [
        "import Foundation".to_owned(),
        tally_error_swift::swift_definition(),
        tallying_bare_service_fault_fields_swift::swift_definition(),
        tallying_bare_service_fault_kind_swift::swift_definition(),
        TallyingBareServiceSchema::swift_http_client(),
        TallyingBareServiceSchema::swift_ws_client(),
        RECORDER.to_owned(),
        main,
    ]
    .join("\n\n");
    let Some(run) = judged(
        "swift-bare-wide-integers",
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
        "`swift -warnings-as-errors main.swift` failed.\n--- stdout ---\n{}\n--- stderr ---\n{}\n--- source ---\n{source}",
        run.stdout, run.stderr
    );
    assert_eq!(
        run.stdout,
        bare_printed(),
        "the Swift client is not the Rust one"
    );
}
