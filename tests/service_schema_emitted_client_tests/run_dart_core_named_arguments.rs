//! Arguments named after the core types Dart reads inside a method, called through the emitted
//! Dart HTTP and WebSocket clients: passed by the analyzer, each request read by the Rust
//! dispatcher.

use super::run_dart_validate::analyzed;
use super::runtime::ran;
use super::tally_http_rest_transport;
use super::tests::{
    TallyBackEnd, TallyClientServiceSchema, tally_client_service_schema, tally_error_dart,
    tally_reply_dart,
};
use core::future::Future;
use core::pin::pin;
use core::task::{Context as PollContext, Poll, Waker};

/// Records each request and answers `200`, then makes the two calls.
const DRIVER: &str = "
class _TallyRecorder implements TallyClientServiceHttpTransport {
  final List<Map<String, dynamic>> sent = <Map<String, dynamic>>[];

  @override
  Future<({int status, List<(String, String)> headers, List<int> body, Stream<List<int>> bodyStream})> send(
    ({String method, String path, String query, List<(String, String)> headers, List<int> body, List<(String, dynamic)> parts}) request,
  ) async {
    sent.add(<String, dynamic>{
      'method': request.method,
      'path': request.path,
      'query': request.query,
      'headers': [for (final (name, value) in request.headers) [name, value]],
      'body': utf8.decode(request.body),
    });
    return (
      status: 200,
      headers: <(String, String)>[],
      body: utf8.encode(jsonEncode(<String, dynamic>{'total': ''})),
      bodyStream: const Stream<List<int>>.empty(),
    );
  }
}

void main() async {
  final recorder = _TallyRecorder();
  final client = TallyClientServiceHttpClient(recorder);
  await client.count('l', 3, 1.5, true);
  await client.find(FindRequest(int: 7, label: 'f'));
  print(jsonEncode(recorder.sent));
}
";

/// Answers every fault `499`, under a header naming its kind.
struct RefusedAs499;

impl tally_http_rest_transport::FaultHandler for RefusedAs499 {
    fn on_fault(
        &self,
        fault: &tally_client_service_schema::ServiceFault,
    ) -> tally_http_rest_transport::OutgoingResponse {
        tally_http_rest_transport::OutgoingResponse::new(
            499,
            vec![("x-fault-kind".to_owned(), format!("{}", fault.kind()))],
            b"refused".to_vec(),
        )
    }
}

fn module() -> String {
    [
        "import 'dart:async';".to_owned(),
        "import 'dart:convert';".to_owned(),
        tally_reply_dart::dart_definition(),
        tally_error_dart::dart_definition(),
        TallyClientServiceSchema::dart_definition(),
        TallyClientServiceSchema::dart_http_client(),
        TallyClientServiceSchema::dart_ws_client(),
        DRIVER.to_owned(),
    ]
    .join("\n\n")
}

/// The dispatcher never suspends, so one poll answers it.
fn poll_once<Answering>(answering: Answering) -> Option<Answering::Output>
where
    Answering: Future,
{
    let mut pinned = pin!(answering);
    match pinned
        .as_mut()
        .poll(&mut PollContext::from_waker(Waker::noop()))
    {
        Poll::Ready(answer) => Some(answer),
        Poll::Pending => None,
    }
}

/// What the Rust handler names when it is handed each request the driver recorded, in order.
fn read_by_rust(printed: &str) -> Vec<String> {
    let sent: Vec<serde_json::Value> = serde_json::from_str(printed.trim()).unwrap();
    sent.iter()
        .map(|request| {
            let text = |key: &str| request[key].as_str().unwrap().to_owned();
            let headers = request["headers"]
                .as_array()
                .unwrap()
                .iter()
                .map(|pair| {
                    (
                        pair[0].as_str().unwrap().to_owned(),
                        pair[1].as_str().unwrap().to_owned(),
                    )
                })
                .collect();
            let incoming = tally_http_rest_transport::IncomingRequest::new(
                text("method"),
                text("path"),
                text("query"),
                headers,
                text("body").into_bytes(),
            );
            let response = poll_once(tally_http_rest_transport::dispatch(
                &TallyBackEnd,
                &(),
                &incoming,
                &tally_http_rest_transport::DefaultFaultHandler,
            ))
            .unwrap();
            let answered: serde_json::Value = serde_json::from_slice(response.body()).unwrap();
            format!("{} {answered}", response.status())
        })
        .collect()
}

#[test]
fn the_dart_clients_have_no_issue_over_arguments_named_after_core_types() {
    analyzed("dart-core-named-arguments-analyze", &module());
}

#[test]
fn the_dart_client_sends_arguments_named_after_core_types_as_the_rust_client_does() {
    let Some(printed) = ran(
        "dart-core-named-arguments",
        "TIXSCHEMA_DART",
        "dart",
        "client.dart",
        &module(),
    ) else {
        return;
    };
    assert_eq!(
        read_by_rust(&printed),
        [r#"200 {"total":"l|3|1.5|true"}"#, r#"200 {"total":"7|f"}"#,],
        "{printed}"
    );
}

#[test]
fn the_route_table_names_each_argument_as_the_path_writes_it() {
    let table: Vec<_> = tally_http_rest_transport::ROUTES
        .iter()
        .map(|route| (route.method(), route.path(), route.operation()))
        .collect();
    assert_eq!(
        table,
        [
            ("POST", "/tallies", "count"),
            ("POST", "/tallies/{int}", "find")
        ]
    );
    assert!(
        tally_http_rest_transport::ROUTES
            .iter()
            .all(|route| route.ok_status() == 200 && route.error_statuses() == [422_u16])
    );
}

#[test]
fn a_request_without_the_header_an_argument_named_after_a_core_type_is_bound_to_is_refused() {
    let incoming = tally_http_rest_transport::IncomingRequest::new(
        "POST".to_owned(),
        "/tallies".to_owned(),
        String::new(),
        vec![("x-int".to_owned(), "3".to_owned())],
        br#""l""#.to_vec(),
    );
    assert_eq!(incoming.headers().len(), 1);
    assert_eq!(incoming.query(), "");
    let response = poll_once(tally_http_rest_transport::dispatch(
        &TallyBackEnd,
        &(),
        &incoming,
        &RefusedAs499,
    ))
    .unwrap();
    assert_eq!(response.status(), 499);
    assert_eq!(response.body(), b"refused");
    assert_eq!(response.headers().len(), 1, "{:?}", response.headers());
}
