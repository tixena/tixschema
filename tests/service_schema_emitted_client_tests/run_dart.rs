//! The emitted Dart client run by the Dart VM, with a real message object.
//!
//! The Dart twin of [`super::run_node`], asking the same three questions. Dart erases nothing, so
//! the module carries the real generated classes: the client calls the `toJson` the `dart` backend
//! wrote, not a stand-in.

#![cfg(feature = "dart")]

use super::runtime::ran;
use super::tests::swift_codec_fixture::{codec_unit_field_dart, codec_unit_payload_dart};
use super::tests::{
    ConversationClientServiceSchema, EchoClientServiceSchema, ShelfClientServiceSchema,
    StampClientServiceSchema, ThumbnailClientServiceSchema, conversation_id_dart,
    echo_range_error_dart, echo_range_response_dart, shelf_dart, shelf_error_dart,
    stamp_error_dart, stamp_receipt_dart, thumbnail_error_dart, window_error_dart,
    window_page_dart,
};

/// Names the runtime to run, for a machine that has one somewhere other than `PATH`.
const RUNTIME_VAR: &str = "TIXSCHEMA_DART";

/// The unit-field group's own driver: constructs a `CodecUnitField`, writes it to JSON, decodes it
/// back, and re-encodes — proving the round trip stays `{}` both ways.
const UNIT_FIELD_DRIVER: &str = "
void main() {
  final value = CodecUnitField(label: 'marker', payload: CodecUnitPayload());
  final encoded = jsonEncode(value.toJson());
  final decoded = CodecUnitField.fromJson(jsonDecode(encoded) as Map<String, dynamic>);
  print(jsonEncode(decoded.toJson()));
}
";

/// Records the request it is handed and answers each operation's own declared status.
const DRIVER: &str = "
class _Recorder implements ConversationClientServiceHttpTransport {
  final List<Map<String, String>> sent = <Map<String, String>>[];

  @override
  Future<({int status, List<(String, String)> headers, List<int> body, Stream<List<int>> bodyStream})> send(
    ({String method, String path, String query, List<(String, String)> headers, List<int> body, List<(String, dynamic)> parts}) request,
  ) async {
    sent.add(<String, String>{
      'method': request.method,
      'path': request.path,
      'query': request.query,
    });
    if (request.method == 'DELETE') {
      return (status: 204, headers: <(String, String)>[], body: <int>[], bodyStream: const Stream<List<int>>.empty());
    }
    return (
      status: 200,
      headers: <(String, String)>[],
      body: utf8.encode(jsonEncode(<String, dynamic>{'items': <String>[]})),
      bodyStream: const Stream<List<int>>.empty(),
    );
  }
}

void main() async {
  final recorder = _Recorder();
  final client = ConversationClientServiceHttpClient(recorder);
  final outcome = await client.window(WindowRequest(
    conversation_id: '652f1a3b4c5d6e7f8a9b0c1d',
    limit: 10,
  ));
  await client.window(WindowRequest(conversation_id: '652f1a3b4c5d6e7f8a9b0c1d'));
  await client.purgeConversation(ConversationId('652f1a3b4c5d6e7f8a9b0c1d'));
  final ok = outcome is ConversationClientServiceWindowResultOk;
  print(jsonEncode(<String, dynamic>{
    'sent': recorder.sent,
    'ok': ok,
    'items': ok ? (outcome as ConversationClientServiceWindowResultOk).value.items : null,
  }));
}
";

/// A stub `ThumbnailClientServiceHttpTransport` answering by path alone, driving the emitted
/// client's own declared-error and `header_out` decode - the Dart twin of the Node client test
/// on the same fixture.
const THUMBNAIL_DRIVER: &str = "
class _ThumbnailRecorder implements ThumbnailClientServiceHttpTransport {
  @override
  Future<({int status, List<(String, String)> headers, List<int> body, Stream<List<int>> bodyStream})> send(
    ({String method, String path, String query, List<(String, String)> headers, List<int> body, List<(String, dynamic)> parts}) request,
  ) async {
    if (request.path == '/thumbnails/missing') {
      return (status: 404, headers: [('x-thumbnail-reason', 'archived')], body: utf8.encode(jsonEncode({'errorCode': 'not-found'})), bodyStream: const Stream<List<int>>.empty());
    }
    if (request.path == '/thumbnails/gone') {
      return (status: 404, headers: <(String, String)>[], body: utf8.encode(jsonEncode({'errorCode': 'not-found'})), bodyStream: const Stream<List<int>>.empty());
    }
    return (status: 200, headers: [('content-type', 'image/png')], body: utf8.encode('PNGDATA'), bodyStream: const Stream<List<int>>.empty());
  }
}

Map<String, dynamic> _describe(ThumbnailClientServiceGetThumbnailResult result) {
  return switch (result) {
    ThumbnailClientServiceGetThumbnailResultOk(:final value) => {
        'kind': 'ok',
        'headerOut': value.$3,
      },
    ThumbnailClientServiceGetThumbnailResultOperation(:final error) => {
        'kind': 'operation',
        'errorCode': error.$1.toJson(),
        'errorHeaderOut': error.$2,
      },
    ThumbnailClientServiceGetThumbnailResultFault(:final fault) => {
        'kind': 'fault',
        'detail': fault.detail,
      },
  };
}

void main() async {
  final client = ThumbnailClientServiceHttpClient(_ThumbnailRecorder());
  final missing = await client.getThumbnail('missing');
  final gone = await client.getThumbnail('gone');
  final anon = await client.getThumbnail('anon');
  print(jsonEncode(<String, dynamic>{
    'missing': _describe(missing),
    'gone': _describe(gone),
    'anon': _describe(anon),
  }));
}
";

/// A stub `StampClientServiceHttpTransport` answering `x-age` by the sent `label` — malformed,
/// valid, and absent — driving the emitted client's own `tryParse`-and-fault header read.
const STAMP_DRIVER: &str = "
class _StampRecorder implements StampClientServiceHttpTransport {
  @override
  Future<({int status, List<(String, String)> headers, List<int> body, Stream<List<int>> bodyStream})> send(
    ({String method, String path, String query, List<(String, String)> headers, List<int> body, List<(String, dynamic)> parts}) request,
  ) async {
    final label = jsonDecode(utf8.decode(request.body)) as String;
    final ageHeader = switch (label) {
      'malformed' => [('etag', 'e1'), ('x-age', 'soon')],
      'valid' => [('etag', 'e1'), ('x-age', '8')],
      _ => [('etag', 'e1')],
    };
    return (
      status: 200,
      headers: ageHeader,
      body: utf8.encode(jsonEncode(<String, dynamic>{'label': label, 'tenant': 'acme'})),
      bodyStream: const Stream<List<int>>.empty(),
    );
  }
}

Map<String, dynamic> _describeStamp(StampClientServiceStampResult result) {
  return switch (result) {
    StampClientServiceStampResultOk(:final value) => {'kind': 'ok', 'age': value.$3},
    StampClientServiceStampResultOperation() => {'kind': 'operation'},
    StampClientServiceStampResultFault(:final fault) => {
        'kind': 'fault',
        'detail': fault.detail,
      },
  };
}

void main() async {
  final client = StampClientServiceHttpClient(_StampRecorder());
  final malformed = await client.stamp('malformed', 'acme', null);
  final valid = await client.stamp('valid', 'acme', null);
  final absent = await client.stamp('absent', 'acme', null);
  print(jsonEncode(<String, dynamic>{
    'malformed': _describeStamp(malformed),
    'valid': _describeStamp(valid),
    'absent': _describeStamp(absent),
  }));
}
";

/// A stub `EchoClientServiceHttpTransport` recording whether `send` was ever called, driving the
/// emitted client's own `header_in` legality check on a value carrying a line feed.
const ECHO_DRIVER: &str = "
class _EchoRecorder implements EchoClientServiceHttpTransport {
  bool sendCalled = false;

  @override
  Future<({int status, List<(String, String)> headers, List<int> body, Stream<List<int>> bodyStream})> send(
    ({String method, String path, String query, List<(String, String)> headers, List<int> body, List<(String, dynamic)> parts}) request,
  ) async {
    sendCalled = true;
    return (status: 200, headers: <(String, String)>[], body: utf8.encode(jsonEncode(<String, dynamic>{'received': 'unreachable'})), bodyStream: const Stream<List<int>>.empty());
  }
}

void main() async {
  final recorder = _EchoRecorder();
  final client = EchoClientServiceHttpClient(recorder);
  final refused = await client.echoRange('doc-1', 'bytes=0-10\\nX-Injected: yes');
  final fault = refused is EchoClientServiceEchoRangeResultFault ? (refused as EchoClientServiceEchoRangeResultFault).fault.kind.wireValue : null;
  print(jsonEncode(<String, dynamic>{'sendCalled': recorder.sendCalled, 'faultKind': fault}));
}
";

/// A stub `ShelfClientServiceHttpTransport` answering by path: a list of strings, a list of
/// classes, an integer, and the plain-enum error when `deep` is set, recording every request body.
const SHELF_DRIVER: &str = "
class _ShelfRecorder implements ShelfClientServiceHttpTransport {
  final List<dynamic> bodies = <dynamic>[];

  @override
  Future<({int status, List<(String, String)> headers, List<int> body, Stream<List<int>> bodyStream})> send(
    ({String method, String path, String query, List<(String, String)> headers, List<int> body, List<(String, dynamic)> parts}) request,
  ) async {
    final sent = jsonDecode(utf8.decode(request.body));
    bodies.add(sent);
    if (request.path == '/shelve') {
      return (status: 204, headers: <(String, String)>[], body: <int>[], bodyStream: const Stream<List<int>>.empty());
    }
    if (request.path == '/tally' && (sent as Map<String, dynamic>)['deep'] == true) {
      return (status: 404, headers: <(String, String)>[], body: utf8.encode(jsonEncode('Missing')), bodyStream: const Stream<List<int>>.empty());
    }
    final Object answer = switch (request.path) {
      '/titles' => <String>['a', 'b'],
      '/stacks' => <Map<String, dynamic>>[<String, dynamic>{'title': 't'}],
      _ => 7,
    };
    return (status: 200, headers: <(String, String)>[], body: utf8.encode(jsonEncode(answer)), bodyStream: const Stream<List<int>>.empty());
  }
}

void main() async {
  final recorder = _ShelfRecorder();
  final client = ShelfClientServiceHttpClient(recorder);
  final shelved = await client.shelve('shelf-1');
  final titles = await client.titles(TitlesRequest(prefix: 'a', limit: 2));
  final stacks = await client.stacks(StacksRequest(prefix: 't', limit: 1));
  final tally = await client.tally(TallyRequest(prefix: 'x', deep: false));
  final refused = await client.tally(TallyRequest(prefix: 'x', deep: true));
  print(jsonEncode(<String, dynamic>{
    'shelveBody': recorder.bodies.first,
    'shelved': shelved is ShelfClientServiceShelveResultOk,
    'titles': (titles as ShelfClientServiceTitlesResultOk).value,
    'stacks': (stacks as ShelfClientServiceStacksResultOk).value.map((shelf) => shelf.toJson()).toList(),
    'tally': (tally as ShelfClientServiceTallyResultOk).value,
    'refused': (refused as ShelfClientServiceTallyResultOperation).error.toJson(),
  }));
}
";

/// The generated classes the client calls, the client, and the driver.
fn module() -> String {
    [
        "import 'dart:convert';".to_owned(),
        conversation_id_dart::dart_definition(),
        window_page_dart::dart_definition(),
        window_error_dart::dart_definition(),
        ConversationClientServiceSchema::dart_definition(),
        ConversationClientServiceSchema::dart_http_client(),
        DRIVER.to_owned(),
    ]
    .join("\n\n")
}

/// What the driver wrote: the requests it recorded, and the outcome of its first `window` call —
/// `None` where no runtime was reachable.
fn driven() -> Option<serde_json::Value> {
    let wrote = ran("dart", RUNTIME_VAR, "dart", "client.dart", &module())?;
    Some(serde_json::from_str(wrote.trim()).unwrap())
}

/// The requests the driver recorded, or `None` where no runtime was reachable.
fn sent() -> Option<Vec<serde_json::Value>> {
    driven().map(|written| written["sent"].as_array().unwrap().clone())
}

#[test]
fn dart_definition_publishes_the_result_pair_and_no_pair_for_the_one_way_operation() {
    let written = ConversationClientServiceSchema::dart_definition();
    assert!(
        written.contains(
            "sealed class ConversationClientServiceWindowResult {\n  \
             const ConversationClientServiceWindowResult();\n}"
        ),
        "got: {written}"
    );
    assert!(
        written.contains(
            "final class ConversationClientServiceWindowResultOk extends \
             ConversationClientServiceWindowResult {\n  \
             const ConversationClientServiceWindowResultOk(this.value);\n  \
             final WindowPage value;\n}"
        ),
        "got: {written}"
    );
    assert!(
        written.contains(
            "final class ConversationClientServiceWindowResultOperation extends \
             ConversationClientServiceWindowResult {\n  \
             const ConversationClientServiceWindowResultOperation(this.error);\n  \
             final WindowError error;\n}"
        ),
        "got: {written}"
    );
    assert!(
        written.contains(
            "final class ConversationClientServiceWindowResultFault extends \
             ConversationClientServiceWindowResult {\n  \
             const ConversationClientServiceWindowResultFault(this.fault);\n  \
             final ConversationClientServiceFaultFields fault;\n}"
        ),
        "got: {written}"
    );
    assert!(
        !written.contains("PurgeConversationResult"),
        "purge_conversation is one-way and declared no reply to join into a pair. Got: {written}"
    );
}

#[test]
fn window_answers_the_ok_result_pair_carrying_the_recorded_items() {
    let Some(written) = driven() else {
        return;
    };
    assert_eq!(
        written["ok"], true,
        "the recorder answers `window`'s own declared `ok_status`, so the client's `Future` \
         resolves to the pair's `Ok` member rather than `Operation` or `Fault`. Got: {written:#?}"
    );
    assert_eq!(
        written["items"],
        serde_json::json!([]),
        "the `Ok` member carries the page the recorder's own canned body decoded into. \
         Got: {written:#?}"
    );
}

#[test]
fn a_lone_placeholder_sends_the_field_it_names_and_never_the_rendered_message() {
    let Some(sent) = sent() else {
        return;
    };
    assert_eq!(
        sent[0]["path"], "/v1/conversations/652f1a3b4c5d6e7f8a9b0c1d/window",
        "the placeholder is filled by the field it names. Got: {sent:#?}"
    );
    assert!(
        !sent[0]["path"].as_str().unwrap().contains("%7B"),
        "rendering the whole message puts its own map spelling in the segment. Got: {sent:#?}"
    );
}

#[test]
fn a_field_the_path_does_not_bind_reaches_the_query_string() {
    let Some(sent) = sent() else {
        return;
    };
    assert_eq!(
        sent[0]["query"], "limit=10",
        "`limit` is bound to no placeholder, so the query string is the only place left for it. \
         Got: {sent:#?}"
    );
    assert_eq!(
        sent[1]["query"], "",
        "the same operation with `limit` absent sends no key for it rather than `limit=null`. \
         Got: {sent:#?}"
    );
}

#[test]
fn a_scalar_message_is_still_the_whole_segment() {
    let Some(sent) = sent() else {
        return;
    };
    assert_eq!(
        sent[2]["path"], "/v1/conversations/652f1a3b4c5d6e7f8a9b0c1d",
        "a message that already is a wire scalar has no field to read: it is the segment. \
         Got: {sent:#?}"
    );
    assert_eq!(
        sent[2]["query"], "",
        "and no key is left over for a query. Got: {sent:#?}"
    );
}

// -------------------------------------------------------------------------------------------
// A unit-struct field's own round trip: construct it, write it to JSON, decode it back, and
// re-encode — `{}` both ways, through the pair's own generated `fromJson`/`toJson`.
// -------------------------------------------------------------------------------------------

fn unit_field_module() -> String {
    [
        "import 'dart:convert';".to_owned(),
        codec_unit_payload_dart::dart_definition(),
        codec_unit_field_dart::dart_definition(),
        UNIT_FIELD_DRIVER.to_owned(),
    ]
    .join("\n\n")
}

#[test]
fn a_unit_struct_field_round_trips_as_an_empty_object() {
    let Some(written) = ran(
        "dart",
        RUNTIME_VAR,
        "dart",
        "unit_field.dart",
        &unit_field_module(),
    ) else {
        return;
    };
    let value: serde_json::Value = serde_json::from_str(written.trim()).unwrap();
    assert_eq!(
        value,
        serde_json::json!({"label": "marker", "payload": {}}),
        "got: {value:#?}"
    );
}

fn thumbnail_module() -> String {
    [
        "import 'dart:convert';".to_owned(),
        thumbnail_error_dart::dart_definition(),
        ThumbnailClientServiceSchema::dart_definition(),
        ThumbnailClientServiceSchema::dart_http_client(),
        THUMBNAIL_DRIVER.to_owned(),
    ]
    .join("\n\n")
}

#[test]
fn the_client_decodes_the_declared_errors_own_header_and_omits_a_none_header_out_element() {
    let Some(wrote) = ran(
        "dart",
        RUNTIME_VAR,
        "dart",
        "thumbnail.dart",
        &thumbnail_module(),
    ) else {
        return;
    };
    let results: serde_json::Value = serde_json::from_str(wrote.trim()).unwrap();
    assert_eq!(
        results["missing"],
        serde_json::json!({"kind": "operation", "errorCode": {"errorCode": "not-found"}, "errorHeaderOut": "archived"}),
        "the declared error's own head and its `error_header_out` element must both decode. \
         got: {results:#?}"
    );
    assert_eq!(
        results["gone"],
        serde_json::json!({"kind": "operation", "errorCode": {"errorCode": "not-found"}, "errorHeaderOut": null}),
        "an absent `error_header_out` header must decode as `null` rather than a string. \
         got: {results:#?}"
    );
    assert_eq!(
        results["anon"],
        serde_json::json!({"kind": "ok", "headerOut": null}),
        "an absent `header_out` header must decode as `null` rather than a string. \
         got: {results:#?}"
    );
}

fn stamp_module() -> String {
    [
        "import 'dart:convert';".to_owned(),
        stamp_receipt_dart::dart_definition(),
        stamp_error_dart::dart_definition(),
        StampClientServiceSchema::dart_definition(),
        StampClientServiceSchema::dart_http_client(),
        STAMP_DRIVER.to_owned(),
    ]
    .join("\n\n")
}

#[test]
fn a_present_header_that_will_not_parse_answers_the_fault_member_not_an_exception() {
    let Some(wrote) = ran("dart", RUNTIME_VAR, "dart", "stamp.dart", &stamp_module()) else {
        return;
    };
    let results: serde_json::Value = serde_json::from_str(wrote.trim()).unwrap();
    assert_eq!(
        results["malformed"]["kind"],
        serde_json::json!("fault"),
        "`x-age: soon` answers the fault member rather than throwing. got: {results:#?}"
    );
    assert_eq!(
        results["malformed"]["detail"],
        serde_json::json!("a response header did not match its declared type"),
        "got: {results:#?}"
    );
    assert_eq!(
        results["valid"],
        serde_json::json!({"kind": "ok", "age": 8_u32}),
        "`x-age: 8` still decodes. got: {results:#?}"
    );
    assert_eq!(
        results["absent"],
        serde_json::json!({"kind": "ok", "age": null}),
        "a missing optional `x-age` reads `null`. got: {results:#?}"
    );
}

fn echo_module() -> String {
    [
        "import 'dart:convert';".to_owned(),
        echo_range_response_dart::dart_definition(),
        echo_range_error_dart::dart_definition(),
        EchoClientServiceSchema::dart_definition(),
        EchoClientServiceSchema::dart_http_client(),
        ECHO_DRIVER.to_owned(),
    ]
    .join("\n\n")
}

#[test]
fn a_header_in_value_with_a_line_feed_is_refused_before_the_transport_is_ever_reached() {
    let Some(wrote) = ran("dart", RUNTIME_VAR, "dart", "echo.dart", &echo_module()) else {
        return;
    };
    let results: serde_json::Value = serde_json::from_str(wrote.trim()).unwrap();
    assert_eq!(
        results["sendCalled"], false,
        "an illegal `header_in` value must refuse before the transport is ever reached. \
         got: {results:#?}"
    );
    assert_eq!(
        results["faultKind"], "failed-validation",
        "got: {results:#?}"
    );
}

fn shelf_module() -> String {
    [
        "import 'dart:convert';".to_owned(),
        shelf_dart::dart_definition(),
        shelf_error_dart::dart_definition(),
        ShelfClientServiceSchema::dart_definition(),
        ShelfClientServiceSchema::dart_http_client(),
        SHELF_DRIVER.to_owned(),
    ]
    .join("\n\n")
}

#[test]
fn a_primitive_message_and_primitive_and_list_successes_cross_the_http_client() {
    let Some(wrote) = ran("dart", RUNTIME_VAR, "dart", "shelf.dart", &shelf_module()) else {
        return;
    };
    let results: serde_json::Value = serde_json::from_str(wrote.trim()).unwrap();
    assert_eq!(
        results,
        serde_json::json!({
            "shelveBody": "shelf-1",
            "shelved": true,
            "titles": ["a", "b"],
            "stacks": [{"title": "t"}],
            "tally": 7_i64,
            "refused": "Missing",
        })
    );
}
