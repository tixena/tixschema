//! The emitted Dart `mcp` server and client run by the Dart VM on `AskingService`: the server's
//! answer to each request of the form, link and content flows is the one the Rust dispatcher
//! gives, and the client reads the answers back into the pair, the list and the declared arm.

#![cfg(feature = "dart")]

use super::mcp_asking_rows::{
    asking_adjacent_error_dart, asking_confirmation_dart, asking_error_dart,
    asking_kind_error_dart, asking_link_error_dart, asking_linked_dart, asking_miss_dart,
    asking_priority_dart, asking_renamed_error_dart, asking_report_request_dart,
    asking_shipping_dart, assert_client, assert_matches, client_input, input,
};
use super::run_dart_validate::analyzed;
use super::runtime::ran;
use super::{AskingServiceSchema, PreviewServiceSchema};
use serde_json::Value;

const RUNTIME_VAR: &str = "TIXSCHEMA_DART";

const SERVER_DRIVER: &str = "
final reached = <Object?>[];

final handlers = AskingServiceHandlers<void>(
  buildReport: (ctx, req) async {
    reached.add(<Object?>['build-report', req.title]);
    return <McpContent>[
      McpContentText(req.title),
      McpContentImage(Uint8List.fromList([0x89, 0x50, 0x4e, 0x47]), 'image/png'),
      McpContentAudio(Uint8List.fromList([1, 2, 3, 4, 5]), 'audio/mpeg'),
      const McpContentResourceLink(
        uri: 'workspace://documents/doc-1',
        name: 'id-card.pdf',
        title: 'Id card',
        mimeType: 'application/pdf',
      ),
      const McpContentResource(
        uri: 'workspace://documents/doc-2',
        mimeType: 'text/plain',
        contents: McpResourceContentsText('passport'),
      ),
      McpContentResource(
        uri: 'workspace://documents/doc-3',
        contents: McpResourceContentsBlob(Uint8List.fromList([9, 8, 7])),
      ),
    ];
  },
  connectRenamed: (ctx, req) async {
    reached.add(<Object?>['connect-renamed', req]);
    throw const AskingRenamedErrorNeedsConnection(
      message: 'Connect your Google Drive so this server can import the file.',
      url: 'https://mcp.example.com/connect/drive?request=7f3a',
    );
  },
  connectAdjacent: (ctx, req) async {
    reached.add(<Object?>['connect-adjacent', req]);
    throw const AskingAdjacentErrorNeedsConnection(
      message: 'Connect your Google Drive so this server can import the file.',
      url: 'https://mcp.example.com/connect/drive?request=7f3a',
    );
  },
  deleteAdjacent: (ctx, req) async {
    reached.add(<Object?>['delete-adjacent', req.document_id, req.confirmation?.toJson()]);
    if (req.document_id == 'missing') throw const AskingAdjacentErrorNotFound();
    final confirmation = req.confirmation;
    if (confirmation == null) throw const AskingAdjacentErrorNeedsConfirmation(message: 'Delete this document for good?');
    if (!confirmation.confirm) throw const AskingAdjacentErrorRefused();
  },
  connectByKind: (ctx, req) async {
    reached.add(<Object?>['connect-by-kind', req]);
    throw const AskingKindErrorNeedsConnection(
      message: 'Connect your Google Drive so this server can import the file.',
      url: 'https://mcp.example.com/connect/drive?request=7f3a',
    );
  },
  deleteDocument: (ctx, req) async {
    reached.add(<Object?>['delete-document', req.document_id, req.confirmation?.toJson()]);
    if (req.document_id == 'missing') throw const AskingErrorNotFound();
    final confirmation = req.confirmation;
    if (confirmation == null) throw const AskingErrorNeedsConfirmation(message: 'Delete this document for good?');
    if (!confirmation.confirm) throw const AskingErrorRefused();
  },
  importFromDrive: (ctx, req) async {
    reached.add(<Object?>['import-from-drive', req]);
    if (req == 'connected') return AskingLinkedDraft(document_id: 'doc-$req').validate();
    throw const AskingLinkErrorNeedsConnection(
      message: 'Connect your Google Drive so this server can import the file.',
      url: 'https://mcp.example.com/connect/drive?request=7f3a',
    );
  },
  shipDocument: (ctx, req) async {
    reached.add(<Object?>['ship-document', req.document_id, req.shipping?.toJson()]);
    if (req.shipping == null) throw const AskingErrorNeedsConfirmation(message: 'How should it ship?');
  },
);

final previewing = PreviewServiceHandlers<void>(
  documentPreview: (ctx, req) async {
    reached.add(<Object?>['document-preview', req]);
    switch (req) {
      case 'doc-1':
        return (Uint8List.fromList([0x89, 0x50, 0x4e, 0x47]), 'image/png');
      case 'voice':
        return (Uint8List.fromList([1, 2, 3, 4, 5]), 'audio/mpeg');
      case 'plain':
        return (Uint8List.fromList([37, 80, 68, 70, 45]), 'application/pdf');
    }
    throw const AskingMissNotFound();
  },
);

void main() async {
  final input = jsonDecode(inputText) as Map<String, dynamic>;
  final asking = createAskingServiceMcpDispatcher<void>(handlers);
  final previews = createPreviewServiceMcpDispatcher<void>(previewing);
  final inFlight = AskingServiceMcpInFlight();
  final previewFlight = PreviewServiceMcpInFlight();
  final previewed = (input['previews'] as List).cast<String>();
  final answers = <String, dynamic>{};
  final calls = <String, dynamic>{};
  for (final entry in (input['requests'] as Map<String, dynamic>).entries) {
    reached.clear();
    answers[entry.key] = previewed.contains(entry.key)
        ? await previews(entry.value as String, (call) {}, (message) async {}, previewFlight)
        : await asking(entry.value as String, (call) {}, (message) async {}, inFlight);
    calls[entry.key] = List<Object?>.of(reached);
  }
  print(jsonEncode({'answers': answers, 'reached': calls}));
}
";

const CLIENT_DRIVER: &str = "
final seen = <Object?>[];
final queue = <String>[];

class Canned implements AskingServiceMcpTransport, PreviewServiceMcpTransport {
  @override
  Future<String> send(({String text, String protocolVersion, String method, String? name}) message) async {
    seen.add(jsonDecode(message.text));
    return queue.removeAt(0);
  }
}

Object? deleted(AskingServiceDeleteDocumentResult result) => switch (result) {
      AskingServiceDeleteDocumentResultOk() => <String, dynamic>{'ok': null},
      AskingServiceDeleteDocumentResultOperation(:final error) => <String, dynamic>{'declared': error.toJson()},
      AskingServiceDeleteDocumentResultFault(:final fault) => <String, dynamic>{'fault': fault.kind.wireValue},
    };

Object? previewed(PreviewServiceDocumentPreviewResult result) => switch (result) {
      PreviewServiceDocumentPreviewResultOk(:final value) => <String, dynamic>{
          'ok': <String, dynamic>{'bytes': value.$1.toList(), 'contentType': value.$2},
        },
      PreviewServiceDocumentPreviewResultOperation(:final error) => <String, dynamic>{'declared': error.toJson()},
      PreviewServiceDocumentPreviewResultFault(:final fault) => <String, dynamic>{'fault': fault.kind.wireValue},
    };

Object? reported(AskingServiceBuildReportResult result) => switch (result) {
      AskingServiceBuildReportResultOk(:final value) => <String, dynamic>{
          'ok': [for (final block in value) block.toJson()],
        },
      AskingServiceBuildReportResultOperation(:final error) => <String, dynamic>{'declared': error.toJson()},
      AskingServiceBuildReportResultFault(:final fault) => <String, dynamic>{'fault': fault.kind.wireValue},
    };

Object? imported(AskingServiceImportFromDriveResult result) => switch (result) {
      AskingServiceImportFromDriveResultOk(:final value) => <String, dynamic>{'ok': value.toJson()},
      AskingServiceImportFromDriveResultOperation(:final error) => <String, dynamic>{'declared': error.toJson()},
      AskingServiceImportFromDriveResultFault(:final fault) => <String, dynamic>{'fault': fault.kind.wireValue},
    };

void main() async {
  final input = jsonDecode(inputText) as Map<String, dynamic>;
  final canned = Canned();
  final client = AskingServiceMcpClient(canned);
  final previewer = PreviewServiceMcpClient(canned);
  final calls = <String, Future<Object?> Function()>{
    'declared': () async => deleted(await client.deleteDocument(const DeleteDocumentRequest(document_id: 'doc-1'))),
    'confirmed': () async => deleted(await client.deleteDocument(
        DeleteDocumentRequest(document_id: 'doc-1', confirmation: AskingConfirmationDraft(confirm: true).validate()))),
    'image': () async => previewed(await previewer.documentPreview('doc-1')),
    'audio': () async => previewed(await previewer.documentPreview('voice')),
    'plain': () async => previewed(await previewer.documentPreview('plain')),
    'missing': () async => previewed(await previewer.documentPreview('nothing')),
    'report': () async => reported(await client.buildReport(AskingReportRequestDraft(title: 'Q3').validate())),
    'link': () async => imported(await client.importFromDrive('1A2b3C')),
    'linked': () async => imported(await client.importFromDrive('connected')),
  };
  final outcomes = <String, Object?>{};
  for (final call in (input['calls'] as List).cast<Map<String, dynamic>>()) {
    queue.add(call['answer'] as String);
    final label = call['label'] as String;
    outcomes[label] = await calls[label]!();
  }
  print(jsonEncode({'outcomes': outcomes, 'seen': seen}));
}
";

/// The imports, the models, the service and its server and client, and `driver`.
fn module(driver: &str, given: &Value) -> String {
    [
        "import 'dart:async';".to_owned(),
        "import 'dart:convert';".to_owned(),
        "import 'dart:typed_data';".to_owned(),
        asking_priority_dart::dart_definition(),
        asking_shipping_dart::dart_definition(),
        asking_confirmation_dart::dart_definition(),
        asking_adjacent_error_dart::dart_definition(),
        asking_error_dart::dart_definition(),
        asking_link_error_dart::dart_definition(),
        asking_renamed_error_dart::dart_definition(),
        asking_kind_error_dart::dart_definition(),
        asking_miss_dart::dart_definition(),
        asking_report_request_dart::dart_definition(),
        asking_linked_dart::dart_definition(),
        AskingServiceSchema::dart_definition(),
        AskingServiceSchema::dart_mcp_content(),
        AskingServiceSchema::dart_ws_client(),
        AskingServiceSchema::dart_mcp_service(),
        AskingServiceSchema::dart_mcp_client(),
        PreviewServiceSchema::dart_definition(),
        PreviewServiceSchema::dart_ws_client(),
        PreviewServiceSchema::dart_mcp_service(),
        PreviewServiceSchema::dart_mcp_client(),
        format!("const inputText = r'''{given}''';"),
        driver.to_owned(),
    ]
    .join("\n\n")
}

fn driven(named: &str, driver: &str, given: &Value) -> Option<Value> {
    let wrote = ran(
        "dart",
        RUNTIME_VAR,
        "dart",
        &format!("{named}.dart"),
        &module(driver, given),
    )?;
    Some(serde_json::from_str(wrote.trim()).unwrap())
}

#[test]
fn the_dart_server_answers_the_bytes_the_rust_dispatcher_answers() {
    let Some(report) = driven("mcp_asking_server", SERVER_DRIVER, &input()) else {
        return;
    };
    assert_matches(&report);
}

#[test]
fn the_dart_client_reads_the_blocks_and_the_arm_the_rust_dispatcher_answers() {
    let Some(report) = driven("mcp_asking_client", CLIENT_DRIVER, &client_input()) else {
        return;
    };
    assert_client(&report);
}

#[test]
fn the_dart_text_of_the_asking_service_passes_the_analyzer() {
    analyzed(
        "mcp-asking-server-analyze",
        &module(SERVER_DRIVER, &input()),
    );
    analyzed(
        "mcp-asking-client-analyze",
        &module(CLIENT_DRIVER, &client_input()),
    );
}
