//! A reply typed with a plain enum that names a check, handed to the emitted Swift `http_rest`
//! client by a canned transport, compiled with warnings as errors: a reply the check refuses is
//! the fault the client answers for a body it cannot read, with the check's sentence, and one it
//! keeps is the value. Each payload is held beside what the serde read says of it. The
//! `ws_rpc` client's driver waits on a real clock, so it stays with the slow runs.

#![cfg(feature = "swift")]

use super::plain_enum_check_rows::appraisal_client_service_schema::{
    appraisal_client_service_fault_fields_swift, appraisal_client_service_fault_kind_swift,
};
use super::plain_enum_check_rows::{
    AppraisalClientServiceSchema, Verdict, appraisal_error_swift, verdict_swift,
};
use super::runtime::judged;

const RUNTIME_VAR: &str = "TIXSCHEMA_SWIFT";

const DRIVER: &str = r#"
func affirmed(_ value: Verdict) -> [String] {
  if case .overturned = value { return ["a verdict is upheld"] }
  return []
}

struct Answering: AppraisalClientServiceHttpTransport {
  let body: String
  func send(_ request: AppraisalClientServiceHttpRequest) async throws -> AppraisalClientServiceHttpResponse {
    AppraisalClientServiceHttpResponse(status: 200, headers: [], body: Data(body.utf8))
  }
}

func described(_ name: String, _ answer: Result<Verdict, AppraisalClientServiceAppraiseFailure>) {
  switch answer {
  case .success(let verdict):
    print("\(name): read \(verdict.rawValue)")
  case .failure(.declared):
    print("\(name): the declared error")
  case .failure(.fault(let fault)):
    print("\(name): the fault \(fault.kind.rawValue)")
    print("    \(fault.detail)")
  }
}

for (name, body) in [("kept", "\"Upheld\""), ("broken", "\"Overturned\"")] {
  let client = AppraisalClientServiceHttpClient(transport: Answering(body: body))
  described(name, await client.appraise("c1"))
}
"#;

#[test]
fn the_swift_http_client_refuses_a_reply_typed_with_a_plain_enum_that_names_a_check() {
    assert_eq!(
        serde_json::from_str::<Verdict>(r#""Overturned""#)
            .unwrap_err()
            .to_string(),
        "a verdict is upheld"
    );
    serde_json::from_str::<Verdict>(r#""Upheld""#).unwrap();
    let source = [
        "import Foundation".to_owned(),
        verdict_swift::swift_definition(),
        appraisal_error_swift::swift_definition(),
        appraisal_client_service_fault_fields_swift::swift_definition(),
        appraisal_client_service_fault_kind_swift::swift_definition(),
        AppraisalClientServiceSchema::swift_http_client(),
        DRIVER.to_owned(),
    ]
    .join("\n\n");
    let Some(run) = judged(
        "swift-plain-enum-replies",
        RUNTIME_VAR,
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
    let lines: Vec<&str> = run.stdout.lines().collect();
    assert_eq!(lines.len(), 3, "got: {}", run.stdout);
    assert_eq!(lines[0], "kept: read Upheld");
    assert_eq!(lines[1], "broken: the fault undeserializable-payload");
    assert!(
        lines[2].contains("a verdict is upheld"),
        "got: {}",
        run.stdout
    );
}
