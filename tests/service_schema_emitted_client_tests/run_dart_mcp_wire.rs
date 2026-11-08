//! The emitted Dart `http_rest` and `ws_rpc` clients, and the `ws_rpc` attachment, of
//! `ShelfWireService`, run by the Dart VM: an operation answering content blocks or prompt
//! messages is read and written in the block form the Rust dispatchers write.

#![cfg(feature = "dart")]

use super::ShelfWireServiceSchema;
use super::mcp_wire_rust::{BLOCKS, MESSAGES, PAGE, over_http, over_ws};
use super::mcp_wire_service::{shelf_wire_error_dart, shelf_wire_request_dart};
use super::run_dart_validate::analyzed;
use super::runtime::ran;
use serde_json::{Value, json};

const RUNTIME_VAR: &str = "TIXSCHEMA_DART";

const DRIVER: &str = "
class Canned implements ShelfWireServiceHttpTransport {
  @override
  Future<({int status, List<(String, String)> headers, List<int> body, Stream<List<int>> bodyStream})> send(
    ({String method, String path, String query, List<(String, String)> headers, List<int> body, List<(String, dynamic)> parts}) request,
  ) async {
    final answer = (rustHttp[request.path] as Map<String, dynamic>);
    return (
      status: answer['status'] as int,
      headers: <(String, String)>[],
      body: (answer['body'] as List).cast<int>(),
      bodyStream: const Stream<List<int>>.empty(),
    );
  }
}

Object? blocksOf(ShelfWireServiceDescribeShelfResult result) => switch (result) {
      ShelfWireServiceDescribeShelfResultOk(:final value) => <String, dynamic>{
          'ok': [for (final block in value) block.toJson()],
        },
      _ => <String, dynamic>{'other': true},
    };

Object? pageOf(ShelfWireServiceListShelvesResult result) => switch (result) {
      ShelfWireServiceListShelvesResultOk(:final value) => <String, dynamic>{'ok': value.toJson()},
      _ => <String, dynamic>{'other': true},
    };

Object? messagesOf(ShelfWireServiceReviewShelfResult result) => switch (result) {
      ShelfWireServiceReviewShelfResultOk(:final value) => <String, dynamic>{
          'ok': [for (final message in value) message.toJson()],
        },
      _ => <String, dynamic>{'other': true},
    };

void main() async {
  final overHttp = ShelfWireServiceHttpClient(Canned());
  final httpDescribed = blocksOf(await overHttp.describeShelf(ShelfWireRequest(shelf: 'history')));
  final httpReviewed = messagesOf(await overHttp.reviewShelf('history'));
  final httpListed = pageOf(await overHttp.listShelves(null));

  final sentRaw = <dynamic>[];
  final outbound = StreamController<dynamic>();
  final inbound = StreamController<dynamic>();
  outbound.stream.listen((frame) {
    sentRaw.add(frame);
    final sent = jsonDecode(frame as String) as Map<String, dynamic>;
    final reply = jsonDecode(rustReplies[sent['operation']] as String) as Map<String, dynamic>;
    reply['id'] = sent['id'];
    inbound.add(jsonEncode(reply));
  });
  final transport = ShelfWireServiceWsTransport(
    sink: outbound.sink,
    stream: inbound.stream,
    heartbeat: ShelfWireServiceWsHeartbeat.off(),
  );
  final overWs = ShelfWireServiceWsClient(transport);
  final wsDescribed = blocksOf(await overWs.describeShelf(ShelfWireRequest(shelf: 'history')));
  final wsReviewed = messagesOf(await overWs.reviewShelf('history'));
  final wsListed = pageOf(await overWs.listShelves(null));
  transport.close();

  final served = StreamController<Map<String, dynamic>>();
  final answers = <Map<String, dynamic>>[];
  final done = Completer<void>();
  final detach = attachShelfWireServiceWsDispatcher<void>(
    (inbound: served.stream, send: (answer) {
      answers.add(answer);
      if (answers.length == frames.length) done.complete();
    }),
    null,
    ShelfWireServiceHandlers<void>(
      describeShelf: (ctx, req) async => <McpContent>[
        McpContentText('Eleven books.'),
        McpContentImage(Uint8List.fromList([0x89, 0x50, 0x4e, 0x47]), 'image/png'),
        McpContentAudio(Uint8List.fromList([0xff, 0xfb, 0x90]), 'audio/mpeg'),
        const McpContentResourceLink(uri: 'shelf://history', name: 'history', title: 'History', mimeType: 'text/markdown'),
        const McpContentResource(
          uri: 'shelf://history/index',
          mimeType: 'text/markdown',
          contents: McpResourceContentsText('# History'),
        ),
        McpContentResource(uri: 'shelf://history/cover', contents: McpResourceContentsBlob(Uint8List.fromList([9, 8, 7]))),
      ],
      listShelves: (ctx, req) async => ShelfWireServiceMcpResourcePage(
        resources: [ShelfWireServiceMcpResource(uri: 'shelf://history', name: 'history', mimeType: 'text/markdown', size: 11)],
        nextCursor: 'page-2',
      ),
      reviewShelf: (ctx, req) async => <McpPromptMessage>[
        const McpPromptMessage(role: McpRole.user, content: McpContentText('Review the history shelf.')),
        const McpPromptMessage(role: McpRole.assistant, content: McpContentText('Which books?')),
      ],
    ),
    onFault: (fault) {},
  );
  for (final frame in frames) {
    served.add(jsonDecode(frame) as Map<String, dynamic>);
  }
  await done.future;
  detach();

  print(jsonEncode(<String, dynamic>{
    'http': [httpDescribed, httpReviewed, httpListed],
    'ws': [wsDescribed, wsReviewed, wsListed],
    'answers': answers,
  }));
}
";

fn module(driver: &str, given: &str) -> String {
    [
        "import 'dart:async';".to_owned(),
        "import 'dart:convert';".to_owned(),
        "import 'dart:typed_data';".to_owned(),
        shelf_wire_error_dart::dart_definition(),
        shelf_wire_request_dart::dart_definition(),
        ShelfWireServiceSchema::dart_definition(),
        ShelfWireServiceSchema::dart_mcp_content(),
        ShelfWireServiceSchema::dart_http_client(),
        ShelfWireServiceSchema::dart_ws_client(),
        given.to_owned(),
        driver.to_owned(),
    ]
    .join("\n\n")
}

/// The answers the Rust dispatchers write, declared for the driver, and the replies the attachment
/// must write.
fn given() -> (String, Vec<Value>) {
    let history = json!({ "shelf": "history" });
    let http = |path: &str, payload: &Value| {
        let (status, body) = over_http(path, payload);
        json!({ "status": status, "body": body.into_bytes() })
    };
    let (describe_frame, describe_reply) = over_ws("describe-shelf", &history);
    let (review_frame, review_reply) = over_ws("review-shelf", &json!("history"));
    let (list_frame, list_reply) = over_ws("list-shelves", &Value::Null);
    let rust_http = json!({
        "/describe-shelf": http("/describe-shelf", &history),
        "/review-shelf": http("/review-shelf", &json!("history")),
        "/list-shelves": http("/list-shelves", &Value::Null),
    });
    let replies = json!({ "describe-shelf": describe_reply, "list-shelves": list_reply, "review-shelf": review_reply });
    let frames = json!([describe_frame, review_frame, list_frame]);
    (
        format!(
            "final rustHttp = jsonDecode(r'''{rust_http}''') as Map<String, dynamic>;\nfinal rustReplies = jsonDecode(r'''{replies}''') as Map<String, dynamic>;\nfinal frames = (jsonDecode(r'''{frames}''') as List).cast<String>();"
        ),
        vec![
            serde_json::from_str(&describe_reply).unwrap(),
            serde_json::from_str(&review_reply).unwrap(),
            serde_json::from_str(&list_reply).unwrap(),
        ],
    )
}

#[test]
fn the_dart_clients_read_and_the_attachment_writes_the_block_form_the_rust_dispatchers_write() {
    let (declared, replies) = given();
    let Some(wrote) = ran(
        "dart",
        RUNTIME_VAR,
        "dart",
        "mcp_wire.dart",
        &module(DRIVER, &declared),
    ) else {
        return;
    };
    let report: Value = serde_json::from_str(wrote.trim()).unwrap();
    let blocks: Value = serde_json::from_str(BLOCKS).unwrap();
    let messages: Value = serde_json::from_str(MESSAGES).unwrap();
    let page: Value = serde_json::from_str(PAGE).unwrap();
    for transport in ["http", "ws"] {
        assert_eq!(report[transport][0], json!({ "ok": blocks }), "{transport}");
        assert_eq!(
            report[transport][1],
            json!({ "ok": messages }),
            "{transport}"
        );
        assert_eq!(report[transport][2], json!({ "ok": page }), "{transport}");
    }
    let answers = report["answers"].as_array().unwrap();
    assert_eq!(answers.len(), replies.len());
    for (got, wanted) in answers.iter().zip(&replies) {
        let mut expected = wanted.clone();
        expected["id"] = got["id"].clone();
        assert_eq!(got, &expected);
    }
}

#[test]
fn the_dart_text_of_the_shelf_service_passes_the_analyzer() {
    let (declared, _) = given();
    analyzed("mcp-wire-analyze", &module(DRIVER, &declared));
}
