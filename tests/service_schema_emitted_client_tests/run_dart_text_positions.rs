//! The emitted Dart `http_rest` client, run under the Dart SDK at `TZ=UTC`, handed each value of
//! [`WIRE_FORMS`](super::wire_forms_models::WIRE_FORMS) in a path, a query, a header and a part:
//! each call prints the request the transport was handed, or `refused` where the client refused
//! the value before the transport was reached, and the Rust dispatcher reads each request.

#![cfg(all(feature = "dart", feature = "chrono"))]

use core::fmt::Write as _;

use super::run_dart_wire_forms::{dart_at_utc, dart_value};
use super::text_position_rows::{
    Call, Position, TextPlacingClientServiceSchema, calls, held_to_the_table, spelled,
    text_placing_error_dart,
};
use super::wire_forms_models::conveyed_slot_dart;

/// A transport that records the request it is handed and answers the empty text, and the one
/// line each call prints: that request, or `refused` where none reached the transport.
const RECORDER: &str = r#"
class _Recorder implements TextPlacingClientServiceHttpTransport {
  Map<String, dynamic>? recorded;

  @override
  Future<({int status, List<(String, String)> headers, List<int> body, Stream<List<int>> bodyStream})> send(
    ({String method, String path, String query, List<(String, String)> headers, List<int> body, List<(String, dynamic)> parts}) request,
  ) async {
    recorded = <String, dynamic>{
      'method': request.method,
      'path': request.path,
      'query': request.query,
      'headers': [for (final (name, value) in request.headers) [name, value]],
      'body': utf8.decode(request.body),
      'parts': [for (final (name, value) in request.parts) [name, value]],
    };
    return (
      status: 200,
      headers: <(String, String)>[],
      body: utf8.encode('""'),
      bodyStream: const Stream<List<int>>.empty(),
    );
  }
}

Future<void> call(_Recorder recorder, Future<Object?> Function() made) async {
  recorder.recorded = null;
  try {
    await made();
  } catch (_) {}
  print(jsonEncode(recorder.recorded ?? 'refused'));
}
"#;

/// The statement that makes `call` through the client.
fn statement(call: &Call) -> String {
    let arguments = spelled(call, &dart_value, "null");
    let operation = call.operation();
    let made = if call.position == Position::Header {
        format!("client.{operation}('n', {})", arguments.join(", "))
    } else {
        let members = call
            .arguments()
            .iter()
            .zip(&arguments)
            .map(|((argument, _, _), value)| format!("{argument}: {value}"))
            .collect::<Vec<_>>()
            .join(", ");
        format!("client.{operation}({}({members}))", call.message())
    };
    format!("  await call(recorder, () => {made});")
}

/// Each value the table's Dart column hands the client is written in a path, a query, a header
/// and a part in the form the table states, and refused where the table states it.
#[test]
fn each_value_is_written_as_text_by_the_dart_client_in_the_form_the_dispatcher_reads() {
    let mut main = String::from(
        "void main() async {\n  final recorder = _Recorder();\n  \
         final client = TextPlacingClientServiceHttpClient(recorder);\n",
    );
    for call in calls(|row| row.dart) {
        let _ = writeln!(main, "{}", statement(&call));
    }
    main.push_str("}\n");
    let source = [
        "import 'dart:async';".to_owned(),
        "import 'dart:convert';".to_owned(),
        conveyed_slot_dart::dart_definition(),
        text_placing_error_dart::dart_definition(),
        TextPlacingClientServiceSchema::dart_definition(),
        TextPlacingClientServiceSchema::dart_http_client(),
        RECORDER.to_owned(),
        main,
    ]
    .join("\n\n");
    let Some((ran, printed)) = dart_at_utc("text-positions", &source) else {
        return;
    };
    assert!(ran, "{printed}");
    held_to_the_table(|row| row.dart, &printed);
}
