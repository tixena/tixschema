//! The emitted Dart `http_rest` client, run under the Dart SDK, handed a bare `i64` as a message,
//! a header and a list, and answered a reply, a header beside it and a declared error of one:
//! each call prints the request the transport was handed and what the client answered. The
//! analyzer passes the `http_rest` and the `ws_rpc` client.

#![cfg(all(feature = "dart", feature = "serde"))]

use core::fmt::Write as _;

use super::bare_digit_rows::{BARE_EXCHANGES, bare_printed};
use super::run_dart_validate::{dart, module};
use super::run_node_digits::{TallyingBareServiceSchema, tally_error_dart};

/// A transport that prints each request it is handed and answers the next response of
/// `responses`, and what prints each answer.
const RECORDER: &str = "
class _Recorder implements TallyingBareServiceHttpTransport {
  _Recorder(this.responses);
  final List<(int, List<(String, String)>, String)> responses;
  var at = 0;

  @override
  Future<({int status, List<(String, String)> headers, List<int> body, Stream<List<int>> bodyStream})> send(
    ({String method, String path, String query, List<(String, String)> headers, List<int> body, List<(String, dynamic)> parts}) request,
  ) async {
    final headed = [for (final (name, value) in request.headers) '$name=$value'].join(',');
    print('${request.method} ${request.path} $headed ${utf8.decode(request.body)}');
    final (status, headers, body) = responses[at];
    at += 1;
    return (status: status, headers: headers, body: utf8.encode(body), bodyStream: const Stream<List<int>>.empty());
  }
}

String shownShift(TallyingBareServiceShiftResult result) => switch (result) {
  TallyingBareServiceShiftResultOk(:final value) => 'ok ${value.$1} ${value.$2}',
  TallyingBareServiceShiftResultOperation(:final error) => 'declared ${jsonEncode(error.toJson())}',
  TallyingBareServiceShiftResultFault(:final fault) => 'fault ${fault.kind.wireValue}',
};

String shownTotal(TallyingBareServiceTotalResult result) => switch (result) {
  TallyingBareServiceTotalResultOk(:final value) => 'ok $value',
  TallyingBareServiceTotalResultOperation(:final error) => 'declared $error',
  TallyingBareServiceTotalResultFault(:final fault) => 'fault ${fault.kind.wireValue}',
};
";

/// Each call hands its transport the request the Rust client sends of it, and reads each response
/// the Rust dispatcher answers back into the integers.
#[test]
fn a_bare_i64_travels_as_its_digits_through_the_dart_client() {
    let mut responses = String::new();
    for exchange in &BARE_EXCHANGES {
        let headers: Vec<String> = exchange
            .headers
            .iter()
            .map(|(name, value)| format!("('{name}', '{value}')"))
            .collect();
        let _ = write!(
            responses,
            "({}, <(String, String)>[{}], r'{}'), ",
            exchange.status,
            headers.join(", "),
            exchange.body
        );
    }
    let main = format!(
        "void main() async {{\n  \
         final client = TallyingBareServiceHttpClient(_Recorder([{responses}]));\n  \
         print(shownShift(await client.shift(9223372036854775806, 1)));\n  \
         print(shownShift(await client.shift(4611686018427387903, null)));\n  \
         print(shownTotal(await client.total([9223372036854775807, 1])));\n  \
         await client.record(-9223372036854775807 - 1);\n  \
         print('sent');\n  \
         print(shownTotal(await client.total([1])));\n\
         }}\n"
    );
    let source = module(
        &[
            "import 'dart:async';".to_owned(),
            tally_error_dart::dart_definition(),
            TallyingBareServiceSchema::dart_definition(),
            TallyingBareServiceSchema::dart_http_client(),
            TallyingBareServiceSchema::dart_ws_client(),
        ],
        &[RECORDER],
        &main,
    );
    let Some((ran, printed)) = dart("bare-digits", &[], &source) else {
        return;
    };
    assert!(ran, "{printed}");
    assert_eq!(
        printed,
        bare_printed(),
        "the Dart client is not the Rust one"
    );
    let Some((clean, said)) = dart("bare-digits-analyze", &["analyze"], &source) else {
        return;
    };
    assert!(clean && said.contains("No issues found!"), "{said}");
}
