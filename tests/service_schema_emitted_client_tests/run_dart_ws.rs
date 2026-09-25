//! The emitted `ws_rpc` Dart client run by the Dart VM, over an in-memory stream pair.
//!
//! The `ws_rpc` twin of [`super::run_dart`]: no socket library is reachable here either, so the
//! driver hands the transport a `StreamController` pair standing in for `WebSocketChannel`'s own
//! sink and stream, and answers a captured `request` frame by hand. Stands down exactly as
//! [`super::run_dart`] does where no Dart runtime is reachable.

#![cfg(feature = "dart")]

use super::runtime::ran;
use super::tests::{
    ConversationClientServiceSchema, conversation_id_dart, window_error_dart, window_page_dart,
};

/// Names the runtime to run, for a machine that has one somewhere other than `PATH`.
const RUNTIME_VAR: &str = "TIXSCHEMA_DART";

/// Drives one `window` call per outcome over a hand-fed reply frame, and a fourth left waiting
/// when the transport closes. `heartbeat` is turned off so no `Timer.periodic` is left scheduled
/// once the driver is done with it.
const DRIVER: &str = "
void main() async {
  final sentRaw = <dynamic>[];
  final outbound = StreamController<dynamic>();
  outbound.stream.listen(sentRaw.add);
  final inbound = StreamController<dynamic>();
  final transport = ConversationClientServiceWsTransport(
    sink: outbound.sink,
    stream: inbound.stream,
    heartbeat: ConversationClientServiceWsHeartbeat.off(),
  );
  final client = ConversationClientServiceWsClient(transport);

  Future<String> lastSentId() async {
    await Future<void>.delayed(Duration.zero);
    final frame = jsonDecode(sentRaw.last as String) as Map<String, dynamic>;
    return frame['id'] as String;
  }

  final okFuture = client.window(WindowRequest(conversation_id: '652f1a3b4c5d6e7f8a9b0c1d'));
  final okId = await lastSentId();
  inbound.add(jsonEncode(<String, dynamic>{
    'kind': 'reply',
    'id': okId,
    'service': 'ConversationClientService',
    'ok': true,
    'value': <String, dynamic>{'items': <String>['a']},
  }));
  final okResult = await okFuture;

  final operationFuture =
      client.window(WindowRequest(conversation_id: '652f1a3b4c5d6e7f8a9b0c1d'));
  final operationId = await lastSentId();
  inbound.add(jsonEncode(<String, dynamic>{
    'kind': 'reply',
    'id': operationId,
    'service': 'ConversationClientService',
    'ok': false,
    'error': <String, dynamic>{'errorCode': 'not-found'},
  }));
  final operationResult = await operationFuture;

  final faultFuture = client.window(WindowRequest(conversation_id: '652f1a3b4c5d6e7f8a9b0c1d'));
  final faultId = await lastSentId();
  inbound.add(jsonEncode(<String, dynamic>{
    'kind': 'reply',
    'id': faultId,
    'service': 'ConversationClientService',
    'ok': false,
    'error': <String, dynamic>{
      'isServiceFault': true,
      'fault': <String, dynamic>{
        'detail': 'boom',
        'kind': 'handler-panic',
        'operation': 'window',
      },
    },
  }));
  final faultResult = await faultFuture;

  final invalidFuture = client.window(WindowRequest(conversation_id: '652f1a3b4c5d6e7f8a9b0c1d'));
  final invalidId = await lastSentId();
  inbound.add(jsonEncode(<String, dynamic>{
    'kind': 'reply',
    'id': invalidId,
    'service': 'ConversationClientService',
    'ok': true,
    'value': 7,
  }));
  final invalidResult = await invalidFuture;

  final closedFuture = client.window(WindowRequest(conversation_id: '652f1a3b4c5d6e7f8a9b0c1d'));
  await lastSentId();
  transport.close();
  final closedResult = await closedFuture;

  print(jsonEncode(<String, dynamic>{
    'ok': okResult is ConversationClientServiceWindowResultOk,
    'okItems': (okResult as ConversationClientServiceWindowResultOk).value.items,
    'operation': operationResult is ConversationClientServiceWindowResultOperation,
    'fault': faultResult is ConversationClientServiceWindowResultFault,
    'invalid': invalidResult is ConversationClientServiceWindowResultFault,
    'closed': closedResult is ConversationClientServiceWindowResultFault,
  }));
}
";

/// The generated classes the client calls, the client, and the driver.
fn module() -> String {
    [
        "import 'dart:async';".to_owned(),
        "import 'dart:convert';".to_owned(),
        conversation_id_dart::dart_definition(),
        window_page_dart::dart_definition(),
        window_error_dart::dart_definition(),
        ConversationClientServiceSchema::dart_definition(),
        ConversationClientServiceSchema::dart_ws_client(),
        DRIVER.to_owned(),
    ]
    .join("\n\n")
}

/// What the driver printed, or `None` where no runtime was reachable.
fn driven() -> Option<serde_json::Value> {
    let wrote = ran("dart", RUNTIME_VAR, "dart", "client.dart", &module())?;
    Some(serde_json::from_str(wrote.trim()).unwrap())
}

#[test]
fn a_reply_frame_with_ok_true_answers_the_ok_member_carrying_the_decoded_value() {
    let Some(written) = driven() else {
        return;
    };
    assert_eq!(written["ok"], true, "got: {written:#?}");
    assert_eq!(
        written["okItems"],
        serde_json::json!(["a"]),
        "got: {written:#?}"
    );
}

#[test]
fn a_reply_frame_with_a_declared_error_answers_the_operation_member() {
    let Some(written) = driven() else {
        return;
    };
    assert_eq!(written["operation"], true, "got: {written:#?}");
}

#[test]
fn a_reply_frame_behind_is_service_fault_answers_the_fault_member() {
    let Some(written) = driven() else {
        return;
    };
    assert_eq!(written["fault"], true, "got: {written:#?}");
}

#[test]
fn a_reply_that_fails_the_declared_decode_answers_the_fault_member() {
    let Some(written) = driven() else {
        return;
    };
    assert_eq!(
        written["invalid"], true,
        "`ok: true` with a value that will not become `WindowPage` is still a fault. \
         Got: {written:#?}"
    );
}

#[test]
fn a_request_still_waiting_when_the_transport_closes_answers_the_fault_member() {
    let Some(written) = driven() else {
        return;
    };
    assert_eq!(
        written["closed"], true,
        "the transport settles a pending request with a value, not an error, once it closes, and \
         the client turns that into the same fault member. Got: {written:#?}"
    );
}
