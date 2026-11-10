//! The emitted Dart `mcp` server and client run by the Dart VM on `GuideService`: the server's
//! answer to each prompt request, completion, HTTP request and scope query is the one the Rust
//! dispatcher gives, and the client reads the answers back into the text, the messages and the
//! declared error.

#![cfg(feature = "dart")]

use super::GuideServiceSchema;
use super::mcp_guide_rows::{
    assert_client, assert_matches, client_input, guide_error_dart, guide_version_dart, input,
};
use super::run_dart_validate::analyzed;
use super::runtime::ran;
use crate::mcp_streams::dart_respond;
use serde_json::Value;

const RUNTIME_VAR: &str = "TIXSCHEMA_DART";

const SERVER_DRIVER: &str = "
final reached = <Object?>[];

final handlers = GuideServiceHandlers<void>(
  configureOrganization: (ctx, req) async {
    reached.add(<Object?>['configure-organization']);
    return 'Apply the entity types in this order.';
  },
  getGuideVersion: (ctx, req) async {
    reached.add(<Object?>['get-guide-version']);
    return GuideVersion(version: 3);
  },
  greet: (ctx, req) async {
    reached.add(<Object?>['greet', req]);
    if (req == 'boom') throw 'the handler came apart';
    if (req == 'missing') throw const GuideErrorNotFound();
    return 'Say hello to $req.';
  },
  reviewDocumentType: (ctx, req) async {
    reached.add(<Object?>['review-document-type', req.document_type, req.focus]);
    return <McpPromptMessage>[
      McpPromptMessage(
        role: McpRole.user,
        content: McpContentText('Review the document type ${req.document_type} against this guide.'),
      ),
      const McpPromptMessage(
        role: McpRole.user,
        content: McpContentResource(
          uri: 'guide://naming',
          mimeType: 'text/markdown',
          contents: McpResourceContentsText('# Naming'),
        ),
      ),
      McpPromptMessage(
        role: McpRole.assistant,
        content: McpContentImage(Uint8List.fromList([0x89, 0x50, 0x4e, 0x47]), 'image/png'),
      ),
      const McpPromptMessage(
        role: McpRole.assistant,
        content: McpContentResourceLink(
          uri: 'guide://naming',
          name: 'naming',
          title: 'Naming',
          mimeType: 'text/markdown',
        ),
      ),
    ];
  },
  suggestDocumentTypes: (ctx, req) async {
    final known = req.chosen.entries.map((entry) => <Object?>[entry.key, entry.value]).toList()
      ..sort((left, right) => (left[0] as String).compareTo(right[0] as String));
    reached.add(<Object?>['suggest-document-types', req.typed, known]);
    return ['ID card', 'Invoice', 'Passport']
        .where((name) => name.toLowerCase().startsWith(req.typed.toLowerCase()))
        .toList();
  },
);

void main() async {
  final input = jsonDecode(inputText) as Map<String, dynamic>;
  final dispatch = createGuideServiceMcpDispatcher<void>(handlers);
  final inFlight = GuideServiceMcpInFlight();
  final http = createGuideServiceMcpHttpDispatcher<void>(
    handlers,
    const GuideServiceMcpHttpSettings(allowedOrigins: []),
    (request, call) async => (ctx: null, denied: null, serverName: null),
  );
  final answers = <String, dynamic>{};
  final calls = <String, dynamic>{};
  for (final entry in (input['requests'] as Map<String, dynamic>).entries) {
    reached.clear();
    answers[entry.key] = await dispatch(entry.value as String, (call) {}, (message) async {}, inFlight);
    calls[entry.key] = List<Object?>.of(reached);
  }
  final answered = <String, dynamic>{};
  for (final row in (input['http'] as List).cast<Map<String, dynamic>>()) {
    final response = await respondGuideService(http, GuideServiceMcpHttpRequest(
      method: 'POST',
      path: '/mcp',
      headers: [for (final pair in row['headers'] as List) ((pair as List)[0] as String, pair[1] as String)],
      body: utf8.encode(row['body'] as String),
    ));
    answered[row['name'] as String] = {'status': response.status, 'body': utf8.decode(response.body)};
  }
  final scopes = [
    for (final query in input['queries'] as List)
      guideServiceMcpScopes((query as List)[0] as String, query[1] as String?),
  ];
  print(jsonEncode({'answers': answers, 'reached': calls, 'http': answered, 'scopes': scopes}));
}
";

const CLIENT_DRIVER: &str = "
final seen = <Object?>[];
final queue = <String>[];

class Canned implements GuideServiceMcpTransport {
  @override
  Future<String> send(({String text, String protocolVersion, String method, String? name}) message) async {
    seen.add(jsonDecode(message.text));
    return queue.removeAt(0);
  }
}

Object? text(GuideServiceGreetResult result) => switch (result) {
      GuideServiceGreetResultOk(:final value) => <String, dynamic>{'ok': value},
      GuideServiceGreetResultOperation(:final error) => <String, dynamic>{'declared': error.toJson()},
      GuideServiceGreetResultFault(:final fault) => <String, dynamic>{'fault': fault.kind.wireValue},
    };

Object? configured(GuideServiceConfigureOrganizationResult result) => switch (result) {
      GuideServiceConfigureOrganizationResultOk(:final value) => <String, dynamic>{'ok': value},
      GuideServiceConfigureOrganizationResultOperation(:final error) => <String, dynamic>{'declared': error.toJson()},
      GuideServiceConfigureOrganizationResultFault(:final fault) => <String, dynamic>{'fault': fault.kind.wireValue},
    };

Object? reviewed(GuideServiceReviewDocumentTypeResult result) => switch (result) {
      GuideServiceReviewDocumentTypeResultOk(:final value) => <String, dynamic>{
          'ok': [for (final message in value) message.toJson()],
        },
      GuideServiceReviewDocumentTypeResultOperation(:final error) => <String, dynamic>{'declared': error.toJson()},
      GuideServiceReviewDocumentTypeResultFault(:final fault) => <String, dynamic>{'fault': fault.kind.wireValue},
    };

void main() async {
  final input = jsonDecode(inputText) as Map<String, dynamic>;
  final client = GuideServiceMcpClient(Canned());
  final calls = <String, Future<Object?> Function()>{
    'string': () async => configured(await client.configureOrganization(const ConfigureOrganizationRequest())),
    'named': () async => text(await client.greet('Ana')),
    'messages': () async => reviewed(await client.reviewDocumentType(const ReviewDocumentTypeRequest(document_type: 'ID card'))),
    'declared': () async => text(await client.greet('missing')),
    'panicked': () async => text(await client.greet('boom')),
    'unknown': () async => text(await client.greet('nobody')),
    'unreadable': () async => reviewed(await client.reviewDocumentType(
        const ReviewDocumentTypeRequest(document_type: 'ID card', focus: 'names'))),
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
        guide_error_dart::dart_definition(),
        guide_version_dart::dart_definition(),
        GuideServiceSchema::dart_definition(),
        GuideServiceSchema::dart_mcp_content(),
        GuideServiceSchema::dart_ws_client(),
        GuideServiceSchema::dart_mcp_service(),
        GuideServiceSchema::dart_mcp_client(),
        dart_respond("GuideService"),
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
    let Some(report) = driven("mcp_guide_server", SERVER_DRIVER, &input()) else {
        return;
    };
    assert_matches(&report);
}

#[test]
fn the_dart_client_reads_the_text_the_messages_and_the_error_the_rust_dispatcher_answers() {
    let Some(report) = driven("mcp_guide_client", CLIENT_DRIVER, &client_input()) else {
        return;
    };
    assert_client(&report);
}

#[test]
fn the_dart_text_of_the_guide_service_passes_the_analyzer() {
    analyzed("mcp-guide-server-analyze", &module(SERVER_DRIVER, &input()));
    analyzed(
        "mcp-guide-client-analyze",
        &module(CLIENT_DRIVER, &client_input()),
    );
}
