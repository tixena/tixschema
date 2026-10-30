//! A reply typed with a plain enum that names a check, handed to the emitted Dart clients of each
//! transport by a canned transport: a reply the check refuses is the fault each client answers
//! for a reply it cannot read, with the check's sentence, and one it keeps is the value. Each
//! payload is held beside what the serde read says of it.

#![cfg(feature = "dart")]

use super::plain_enum_check_rows::{
    AppraisalClientServiceSchema, Verdict, appraisal_error_dart, verdict_dart,
};
use super::run_dart_validate::{analyzed, module};
use super::runtime::ran;

const RUNTIME_VAR: &str = "TIXSCHEMA_DART";

const CHECKS: &str = "
List<String> affirmed(Verdict value) =>
    value == Verdict.overturned ? const ['a verdict is upheld'] : const [];

Map<String, dynamic> described(AppraisalClientServiceAppraiseResult result) {
  return switch (result) {
    AppraisalClientServiceAppraiseResultOk(:final value) => {'kind': 'ok', 'value': value.toJson()},
    AppraisalClientServiceAppraiseResultOperation() => {'kind': 'operation'},
    AppraisalClientServiceAppraiseResultFault(:final fault) => {
        'kind': 'fault',
        'faultKind': fault.kind.name,
        'detail': fault.detail,
      },
  };
}
";

const HTTP_DRIVER: &str = "
class _Court implements AppraisalClientServiceHttpTransport {
  _Court(this.reply);
  final String reply;

  @override
  Future<({int status, List<(String, String)> headers, List<int> body, Stream<List<int>> bodyStream})> send(
    ({String method, String path, String query, List<(String, String)> headers, List<int> body, List<(String, dynamic)> parts}) request,
  ) async {
    return (
      status: 200,
      headers: <(String, String)>[],
      body: utf8.encode(jsonEncode(reply)),
      bodyStream: const Stream<List<int>>.empty(),
    );
  }
}

void main() async {
  print(jsonEncode(<String, dynamic>{
    'kept': described(await AppraisalClientServiceHttpClient(_Court('Upheld')).appraise('c1')),
    'broken': described(await AppraisalClientServiceHttpClient(_Court('Overturned')).appraise('c1')),
  }));
}
";

const WS_DRIVER: &str = "
void main() async {
  final sentRaw = <dynamic>[];
  final outbound = StreamController<dynamic>();
  outbound.stream.listen(sentRaw.add);
  final inbound = StreamController<dynamic>();
  final transport = AppraisalClientServiceWsTransport(
    sink: outbound.sink,
    stream: inbound.stream,
    heartbeat: AppraisalClientServiceWsHeartbeat.off(),
  );
  final client = AppraisalClientServiceWsClient(transport);

  Future<Map<String, dynamic>> answered(String reply) async {
    final appraised = client.appraise('c1');
    await Future<void>.delayed(Duration.zero);
    final frame = jsonDecode(sentRaw.last as String) as Map<String, dynamic>;
    inbound.add(jsonEncode(<String, dynamic>{
      'kind': 'reply',
      'id': frame['id'],
      'service': 'AppraisalClientService',
      'ok': true,
      'value': reply,
    }));
    return described(await appraised);
  }

  final kept = await answered('Upheld');
  final broken = await answered('Overturned');
  transport.close();
  print(jsonEncode(<String, dynamic>{'kept': kept, 'broken': broken}));
}
";

fn library(imports: &[String], client: String, entry: &str) -> String {
    module(
        &[
            verdict_dart::dart_definition(),
            appraisal_error_dart::dart_definition(),
            AppraisalClientServiceSchema::dart_definition(),
            client,
        ],
        &[CHECKS],
        entry,
    )
    .replacen(
        "import 'dart:convert';",
        &format!("import 'dart:convert';\n{}", imports.join("\n")),
        1,
    )
}

fn answers(kind: &str) -> serde_json::Value {
    serde_json::json!({
        "kept": {"kind": "ok", "value": "Upheld"},
        "broken": {
            "kind": "fault",
            "faultKind": kind,
            "detail": "FormatException: a verdict is upheld",
        },
    })
}

fn held_to_serde() {
    assert_eq!(
        serde_json::from_str::<Verdict>(r#""Overturned""#)
            .unwrap_err()
            .to_string(),
        "a verdict is upheld"
    );
    serde_json::from_str::<Verdict>(r#""Upheld""#).unwrap();
}

#[test]
fn the_dart_http_client_refuses_a_reply_typed_with_a_plain_enum_that_names_a_check() {
    held_to_serde();
    let source = library(
        &[],
        AppraisalClientServiceSchema::dart_http_client(),
        HTTP_DRIVER,
    );
    analyzed("court-http-analyze", &source);
    let Some(wrote) = ran("dart", RUNTIME_VAR, "dart", "court_http.dart", &source) else {
        return;
    };
    let written: serde_json::Value = serde_json::from_str(wrote.trim()).unwrap();
    assert_eq!(written, answers("undeserializablePayload"));
}

#[test]
fn the_dart_ws_client_refuses_a_reply_typed_with_a_plain_enum_that_names_a_check() {
    held_to_serde();
    let source = library(
        &["import 'dart:async';".to_owned()],
        AppraisalClientServiceSchema::dart_ws_client(),
        WS_DRIVER,
    );
    analyzed("court-ws-analyze", &source);
    let Some(wrote) = ran("dart", RUNTIME_VAR, "dart", "court_ws.dart", &source) else {
        return;
    };
    let written: serde_json::Value = serde_json::from_str(wrote.trim()).unwrap();
    assert_eq!(written, answers("failedValidation"));
}
