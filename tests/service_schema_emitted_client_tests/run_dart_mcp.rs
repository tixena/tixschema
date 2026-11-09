//! The Dart `mcp` server and client run by the Dart VM: the server answers the messages of the
//! design, every answer validated against the official 2026-07-28 schema, and the client reads the
//! outcomes a server can answer with.

#![cfg(feature = "dart")]

use super::WriterServiceSchema;
use super::mcp_auth_rows::{
    AUTHORIZATION_SERVER, AuthRow, METADATA_URL, ORIGIN, RESOURCE, SCOPE_QUERIES, SCOPES_SUPPORTED,
    assert_organization_answer, auth_rows,
};
use super::mcp_dart_document as document_mcp;
use super::mcp_dart_services::{
    Archive, AuditService, AuditServiceSchema, DartBillingService as _, DartBillingServiceSchema,
    DartDocumentService, DartDocumentServiceSchema, McpDocumentId, McpInvoiceRequest,
    McpVersionError, McpVersionRequest, TaggingService as _, TaggingServiceSchema,
    mcp_document_id_dart, mcp_draft_batch_dart, mcp_draft_dart, mcp_format_dart,
    mcp_invoice_list_dart, mcp_invoice_request_dart, mcp_reference_dart, mcp_version_error_dart,
    mcp_version_request_dart, mcp_version_response_dart,
};
use super::mcp_writer_rows::{
    assert_matches, input as writer_input, writer_refusal_dart, writer_version_dart,
    writer_version_request_dart,
};
use super::run_dart_validate::{analyzed, dart};
use super::run_node_mcp::cases_json;
use super::runtime::{ran, ran_with_stderr};
use super::tests::poll_once;
use crate::cases;
use crate::mcp_streams::dart_respond;
use core::future::Future;
use serde_json::{Value, json};

const RUNTIME_VAR: &str = "TIXSCHEMA_DART";

/// The official 2026-07-28 schema of the Model Context Protocol.
const MCP_SCHEMA: &str = include_str!("../mcp_declaration_tests/mcp-schema-2026-07-28.json");

const VERSION: &str = env!("CARGO_PKG_VERSION");

/// The definitions that describe a whole error response; every other error definition describes
/// the `error` member alone.
const WHOLE_ERRORS: [&str; 2] = ["HeaderMismatchError", "UnsupportedProtocolVersionError"];

/// The server the driver builds: the document service, and beside it the billing service with a
/// resource, a template and a listing a hand-written stand-in answers.
const SERVER_DRIVER: &str = r#"
const meta = <String, dynamic>{
  'io.modelcontextprotocol/protocolVersion': '2026-07-28',
  'io.modelcontextprotocol/clientCapabilities': <String, dynamic>{},
};

String request(Object? id, String method, [Map<String, dynamic> params = const <String, dynamic>{}]) =>
    jsonEncode(<String, dynamic>{
      'jsonrpc': '2.0',
      'id': id,
      'method': method,
      'params': <String, dynamic>{...params, '_meta': meta},
    });

const faulted = /*FAULT_CASES*/;

void main() async {
  final touched = <String>[];
  final tags = <String?>[];
  final documents = DartDocumentServiceHandlers<void>(
    checkDrafts: (ctx, req) async => McpVersionResponse(version: req.drafts.length),
    checkMany: (ctx, req) async => McpVersionResponse(version: 7),
    checkMaybe: (ctx, req) async => McpVersionResponse(version: 7),
    checkNamed: (ctx, req) async => McpVersionResponse(version: 7),
    getTagged: (ctx, req, known_tag) async {
      tags.add(known_tag);
      return (McpVersionResponse(version: 7), '"v7"');
    },
    getVersion: (ctx, req) async {
      if (req.document_id == 'missing') throw const McpVersionErrorNotFound();
      if (req.document_id == 'panic') throw 'the handler came apart';
      return McpVersionResponse(version: 7);
    },
    lookup: (ctx, req) async {
      if (req.value == 'panic') throw 'the handler came apart';
      return McpVersionResponse(version: req.value.length);
    },
    renameDocument: (ctx, req) async {},
    touchDocument: (ctx, req) async {
      touched.add('${req.document_id}:${req.reader}');
    },
  );
  final billing = DartBillingServiceHandlers<void>(
    listInvoices: (ctx, req) async => McpInvoiceList(invoices: <String>['inv-1', req.organization]),
  );
  final documentRoute = createDartDocumentServiceMcpRoute<void>(documents);
  final billingRoute = createDartBillingServiceMcpRoute<void>(billing);
  final guide = (
    tools: dartBillingServiceMcpCatalog.tools,
    resources: <String>['{"mimeType":"text/markdown","name":"configuration-guide","uri":"guide://configure-organization"}'],
    resourceTemplates: <String>['{"name":"documents","uriTemplate":"workspace://documents/{document_id}"}'],
    prompts: <String>[],
    skills: <String>[],
    capabilities: <String>['{"resources":{},"tools":{}}'],
  );
  final endpoint = DartDocumentServiceMcpEndpoint(
    {...dartDocumentServiceMcpNames, ...dartBillingServiceMcpNames},
    [dartDocumentServiceMcpCatalog, guide],
  );
  Future<DartDocumentServiceMcpAnswer?> route(int part, String method, Map<String, dynamic> params) async {
    if (method == 'resources/list') {
      final cursor = params['cursor'];
      if (part == 0 && cursor == null) {
        return (result: <String, dynamic>{'resources': [<String, dynamic>{'name': 'id-card.pdf', 'uri': 'workspace://documents/doc-1'}], 'nextCursor': 'page-2'}, error: null);
      }
      if (part == 0) {
        return (result: <String, dynamic>{'resources': [<String, dynamic>{'name': 'passport.pdf', 'uri': 'workspace://documents/doc-2'}]}, error: null);
      }
      return (result: <String, dynamic>{'resources': [<String, dynamic>{'name': 'inv-1', 'uri': 'billing://invoices/inv-1'}]}, error: null);
    }
    if (part == 0) return documentRoute(null, method, params);
    final answered = await billingRoute(null, method, params);
    if (answered != null) return answered;
    if (method == 'resources/read') {
      return (
        result: <String, dynamic>{
          'cacheScope': 'private',
          'contents': [<String, dynamic>{'mimeType': 'application/json', 'text': '{"total":120}', 'uri': 'billing://invoices/inv-1'}],
          'ttlMs': 0,
        },
        error: null,
      );
    }
    return null;
  }

  final host = DartDocumentServiceMcpHost(writer: (message) async {}, inFlight: DartDocumentServiceMcpInFlight());
  final answers = <String, String?>{};
  Future<void> ask(String name, String text) async {
    answers[name] = await dartDocumentServiceMcpServe<void>(text, 'Workspace', null, endpoint, (call) {}, host,
        (ctx, part, method, params) => route(part, method, params));
  }

  await ask('discover', request('discover-1', 'server/discover'));
  await ask('tools_list', request(1, 'tools/list'));
  await ask('success', request(2, 'tools/call', {'name': 'get-version', 'arguments': {'document_id': 'doc-1'}}));
  await ask('declared', request(4, 'tools/call', {'name': 'get-version', 'arguments': {'document_id': 'missing'}}));
  await ask('unread', request(5, 'tools/call', {'name': 'get-version', 'arguments': {'document_id': 5}}));
  await ask('refused', request(6, 'tools/call', {'name': 'get-version', 'arguments': {'document_id': ''}}));
  await ask('second_service', request(7, 'tools/call', {'name': 'list-invoices', 'arguments': {'organization': 'acme'}}));
  await ask('unknown_tool', request(8, 'tools/call', {'name': 'delete-everything', 'arguments': {}}));
  await ask('notification', jsonEncode({'jsonrpc': '2.0', 'method': 'notifications/initialized'}));
  await ask('old_version', jsonEncode({
    'jsonrpc': '2.0',
    'id': 9,
    'method': 'tools/list',
    'params': {'_meta': {...meta, 'io.modelcontextprotocol/protocolVersion': '2025-11-25'}},
  }));
  await ask('unknown_method', request(10, 'tools/unheard'));
  await ask('resource', request(11, 'resources/read', {'uri': 'billing://invoices/inv-1'}));
  await ask('templates', request(12, 'resources/templates/list'));
  await ask('panic', request(13, 'tools/call', {'name': 'get-version', 'arguments': {'document_id': 'panic'}}));
  await ask('one_way', request(14, 'tools/call', {'name': 'touch-document', 'arguments': {'documentId': 'doc-1', 'reader': 'reviewer-7'}}));
  await ask('wrapped', request(15, 'tools/call', {'name': 'lookup-document', 'arguments': {'value': 'doc-12'}}));
  await ask('headers', request(16, 'tools/call', {'name': 'get-tagged', 'arguments': {'value': 'doc-1', 'if-none-match': '"v6"'}}));
  await ask('no_header', request(17, 'tools/call', {'name': 'get-tagged', 'arguments': {'value': 'doc-1'}}));
  await ask('bad_header', request(18, 'tools/call', {'name': 'get-tagged', 'arguments': {'value': 'doc-1', 'if-none-match': 5}}));
  await ask('unit', request(19, 'tools/call', {'name': 'rename-document', 'arguments': {'documentId': 'doc-1', 'title': 'Plan'}}));
  await ask('no_meta', jsonEncode({'jsonrpc': '2.0', 'id': 20, 'method': 'tools/list', 'params': {}}));
  await ask('not_json', '{');
  await ask('resources_first', request(21, 'resources/list'));
  await ask('resources_second', request(22, 'resources/list', {'cursor': '0:page-2'}));
  await ask('resources_last', request(23, 'resources/list', {'cursor': '1:'}));
  await ask('resources_invalid', request(24, 'resources/list', {'cursor': '7:'}));
  await ask('prompts', request(25, 'prompts/list'));
  await ask('unknown_variant', request(40, 'tools/call', {'name': 'get-version', 'arguments': {'document_id': 'doc-1', 'format': 'rtf'}}));
  for (var at = 0; at < faulted.length; at++) {
    final held = faulted[at] as Map<String, dynamic>;
    await ask('fault_$at', request(100 + at, 'tools/call', {'name': held['tool'], 'arguments': held['arguments']}));
  }
  await ask('no_arguments', request(26, 'tools/call', {'name': 'get-version'}));

  final alone = createDartDocumentServiceMcpDispatcher<void>(documents);
  answers['alone'] = await alone(request(27, 'tools/list'), (call) {}, host.writer, host.inFlight);

  print(jsonEncode({'answers': answers, 'touched': touched, 'tags': tags}));
}
"#;

/// The HTTP rules around the server, one request per rule.
const HTTP_DRIVER: &str = r#"
const meta = <String, dynamic>{
  'io.modelcontextprotocol/protocolVersion': '2026-07-28',
  'io.modelcontextprotocol/clientCapabilities': <String, dynamic>{},
};

DartDocumentServiceMcpHttpRequest post(
  Map<String, dynamic> body, {
  Map<String, String> headers = const <String, String>{},
  String method = 'POST',
}) =>
    DartDocumentServiceMcpHttpRequest(
      method: method,
      path: '/mcp',
      headers: [for (final entry in headers.entries) (entry.key, entry.value)],
      body: utf8.encode(jsonEncode(body)),
    );

Map<String, dynamic> call(Object? id, String name) => {
      'jsonrpc': '2.0',
      'id': id,
      'method': 'tools/call',
      'params': {'name': name, 'arguments': {'document_id': 'doc-1'}, '_meta': meta},
    };

const proper = <String, String>{
  'MCP-Protocol-Version': '2026-07-28',
  'Mcp-Method': 'tools/call',
  'Mcp-Name': 'get-version',
};

void main() async {
  final documents = DartDocumentServiceHandlers<void>(
    checkDrafts: (ctx, req) async => McpVersionResponse(version: req.drafts.length),
    checkMany: (ctx, req) async => McpVersionResponse(version: 7),
    checkMaybe: (ctx, req) async => McpVersionResponse(version: 7),
    checkNamed: (ctx, req) async => McpVersionResponse(version: 7),
    getTagged: (ctx, req, known_tag) async => (McpVersionResponse(version: 7), '"v7"'),
    getVersion: (ctx, req) async => McpVersionResponse(version: 7),
    lookup: (ctx, req) async => McpVersionResponse(version: 7),
    renameDocument: (ctx, req) async {},
    touchDocument: (ctx, req) async {},
  );
  final serve = createDartDocumentServiceMcpHttpDispatcher<void>(
    documents,
    const DartDocumentServiceMcpHttpSettings(allowedOrigins: ['https://app.example.com']),
    (request, call) async => (ctx: null, denied: null, serverName: null),
  );
  final answers = <String, Map<String, dynamic>>{};
  Future<void> ask(String name, DartDocumentServiceMcpHttpRequest request) async {
    final response = await respondDartDocumentService(serve, request);
    answers[name] = {
      'status': response.status,
      'headers': [for (final (key, value) in response.headers) '$key: $value'],
      'body': utf8.decode(response.body),
    };
  }

  await ask('success', post(call(1, 'get-version'), headers: proper));
  await ask('lowercase_names', post(call(2, 'get-version'), headers: {
    'mcp-protocol-version': '2026-07-28',
    'MCP-METHOD': 'tools/call',
    'mcp-name': 'get-version',
  }));
  await ask('base64_name', post(call(3, 'get-version'), headers: {
    ...proper,
    'Mcp-Name': '=?base64?${base64.encode(utf8.encode('get-version'))}?=',
  }));
  await ask('unknown_tool', post(call(4, 'delete-everything'), headers: {...proper, 'Mcp-Name': 'delete-everything'}));
  await ask('get', post(call(5, 'get-version'), headers: proper, method: 'GET'));
  await ask('foreign_origin', post(call(6, 'get-version'), headers: {...proper, 'Origin': 'https://evil.example.com'}));
  await ask('allowed_origin', post(call(7, 'get-version'), headers: {...proper, 'Origin': 'https://app.example.com'}));
  await ask('unreadable', DartDocumentServiceMcpHttpRequest(method: 'POST', path: '/mcp', headers: const [], body: utf8.encode('{')));
  await ask('notification', post({'jsonrpc': '2.0', 'method': 'notifications/initialized'}, headers: proper));
  await ask('no_version_header', post(call(8, 'get-version'), headers: {'Mcp-Method': 'tools/call', 'Mcp-Name': 'get-version'}));
  await ask('method_differs', post(call(9, 'get-version'), headers: {...proper, 'Mcp-Method': 'tools/list'}));
  await ask('name_missing', post(call(10, 'get-version'), headers: {
    'MCP-Protocol-Version': '2026-07-28',
    'Mcp-Method': 'tools/call',
  }));
  await ask('name_differs', post(call(11, 'get-version'), headers: {...proper, 'Mcp-Name': 'lookup-document'}));
  await ask('no_meta', post({'jsonrpc': '2.0', 'id': 12, 'method': 'tools/list', 'params': {}}, headers: {
    'MCP-Protocol-Version': '2026-07-28',
    'Mcp-Method': 'tools/list',
  }));
  await ask('old_version', post({
    'jsonrpc': '2.0',
    'id': 13,
    'method': 'tools/list',
    'params': {'_meta': {...meta, 'io.modelcontextprotocol/protocolVersion': '2025-11-25'}},
  }, headers: {'MCP-Protocol-Version': '2025-11-25', 'Mcp-Method': 'tools/list'}));
  await ask('unknown_method', post({
    'jsonrpc': '2.0',
    'id': 14,
    'method': 'tools/unheard',
    'params': {'_meta': meta},
  }, headers: {'MCP-Protocol-Version': '2026-07-28', 'Mcp-Method': 'tools/unheard'}));
  print(jsonEncode(answers));
}
"#;

/// Authorization: every request of `auth_rows()` through the server with the `admit` of its scheme,
/// and every scope query through `dartDocumentServiceMcpScopes`.
const AUTH_DRIVER: &str = r#"
void main() async {
  final input = jsonDecode(inputText) as Map<String, dynamic>;
  var reached = 0;
  final documents = DartDocumentServiceHandlers<void>(
    checkDrafts: (ctx, req) async => McpVersionResponse(version: req.drafts.length),
    checkMany: (ctx, req) async => McpVersionResponse(version: 7),
    checkMaybe: (ctx, req) async => McpVersionResponse(version: 7),
    checkNamed: (ctx, req) async => McpVersionResponse(version: 7),
    getTagged: (ctx, req, known_tag) async {
      reached += 1;
      return (McpVersionResponse(version: 7), '"v7"');
    },
    getVersion: (ctx, req) async {
      reached += 1;
      return McpVersionResponse(version: req.document_id.length);
    },
    lookup: (ctx, req) async {
      reached += 1;
      return McpVersionResponse(version: 7);
    },
    renameDocument: (ctx, req) async {
      reached += 1;
    },
    touchDocument: (ctx, req) async {
      reached += 1;
    },
  );
  const tokens = <String, String>{'read-token': 'documents:read', 'write-token': 'documents:read documents:write'};
  const noCtx = (ctx: null, denied: null, serverName: null);
  final admits = <String, DartDocumentServiceMcpAdmit<void>>{
    'oauth': (request, call) async {
      final needed = dartDocumentServiceMcpScopes(request.header('mcp-method') ?? '', request.mcpName);
      final token = request.bearerToken;
      if (token == null) {
        return (ctx: null, denied: DartDocumentServiceMcpNoValidCredentials(scope: needed.isEmpty ? null : needed.join(' ')), serverName: null);
      }
      if (token == 'forbidden-token') return (ctx: null, denied: const DartDocumentServiceMcpForbidden(), serverName: null);
      final held = tokens[token];
      if (held == null) return (ctx: null, denied: const DartDocumentServiceMcpNoValidCredentials(), serverName: null);
      if (needed.any((scope) => !held.split(' ').contains(scope))) {
        return (ctx: null, denied: DartDocumentServiceMcpInsufficientScope(scope: needed.join(' ')), serverName: null);
      }
      return noCtx;
    },
    'keys': (request, call) async {
      final key = request.header('x-api-key');
      final secret = request.header('x-api-secret');
      if (key == null || secret == null) return (ctx: null, denied: const DartDocumentServiceMcpNoValidCredentials(), serverName: null);
      if (key == 'admin-key' && secret == 'admin-secret') return noCtx;
      return (ctx: null, denied: const DartDocumentServiceMcpForbidden(), serverName: null);
    },
    'organization': (request, call) async {
      final organization = request.header('x-organization');
      if (organization == null) return noCtx;
      if (organization == 'banned') return (ctx: null, denied: const DartDocumentServiceMcpForbidden(), serverName: null);
      return (ctx: null, denied: null, serverName: '$organization-documents');
    },
  };
  final oauth = input['oauth'] as Map<String, dynamic>;
  final settings = <String, DartDocumentServiceMcpHttpSettings>{
    'oauth': DartDocumentServiceMcpHttpSettings(
      allowedOrigins: [input['origin'] as String],
      oauth: DartDocumentServiceMcpOAuth(
        authorizationServers: (oauth['authorizationServers'] as List).cast<String>(),
        resource: oauth['resource'] as String,
        resourceMetadataUrl: oauth['resourceMetadataUrl'] as String,
        scopesSupported: (oauth['scopesSupported'] as List).cast<String>(),
      ),
    ),
    'keys': DartDocumentServiceMcpHttpSettings(allowedOrigins: [input['origin'] as String]),
    'organization': DartDocumentServiceMcpHttpSettings(allowedOrigins: [input['origin'] as String]),
  };
  final answers = <Map<String, dynamic>>[];
  for (final row in input['rows'] as List) {
    final given = row as Map<String, dynamic>;
    final scheme = given['scheme'] as String;
    final serve = createDartDocumentServiceMcpHttpDispatcher<void>(documents, settings[scheme]!, admits[scheme]!);
    final before = reached;
    final response = await respondDartDocumentService(serve, DartDocumentServiceMcpHttpRequest(
      method: given['method'] as String,
      path: given['path'] as String,
      headers: [for (final pair in given['headers'] as List) ((pair as List)[0] as String, pair[1] as String)],
      body: utf8.encode(given['body'] as String),
    ));
    answers.add({
      'status': response.status,
      'headers': [for (final (key, value) in response.headers) [key, value]],
      'body': utf8.decode(response.body),
      'reached': reached - before,
    });
  }
  final scopes = [
    for (final query in input['queries'] as List)
      dartDocumentServiceMcpScopes((query as List)[0] as String, query[1] as String?),
  ];
  print(jsonEncode({'answers': answers, 'scopes': scopes}));
}
"#;

/// A client over a seam that answers canned text, one answer per call.
const CLIENT_DRIVER: &str = r#"
class Canned implements DartDocumentServiceMcpTransport {
  Canned(this.answers);
  final List<Object> answers;
  final sent = <({String text, String protocolVersion, String method, String? name})>[];

  @override
  Future<String> send(({String text, String protocolVersion, String method, String? name}) message) async {
    sent.add(message);
    final answer = answers.removeAt(0);
    if (answer is String) return answer;
    throw 'the line went down';
  }
}

String result(Object? id, Map<String, dynamic> members) =>
    jsonEncode({'jsonrpc': '2.0', 'id': id, 'result': {'resultType': 'complete', ...members}});

String toolText(Object? value) => jsonEncode({
      'content': [{'type': 'text', 'text': jsonEncode(value)}],
      'isError': true,
    });

String fault(String kind) => jsonEncode({
      'isServiceFault': true,
      'fault': {'detail': 'it broke', 'kind': kind, 'operation': 'get-version'},
    });

String summary(DartDocumentServiceGetVersionResult outcome) => switch (outcome) {
      DartDocumentServiceGetVersionResultOk(:final value) => 'ok version=${value.version}',
      DartDocumentServiceGetVersionResultOperation(:final error) => 'declared ${jsonEncode(error.toJson())}',
      DartDocumentServiceGetVersionResultFault(:final fault) => 'fault ${fault.kind.wireValue}',
    };

void main() async {
  final lines = <String>[];
  final canned = Canned([
    result(1, {'content': [{'type': 'text', 'text': '{"version":7}'}], 'structuredContent': {'version': 7}, 'isError': false}),
    result(2, {...jsonDecode(toolText({'errorCode': 'not-found'})) as Map<String, dynamic>}),
    result(3, {...jsonDecode(toolText(jsonDecode(fault('failed-validation')))) as Map<String, dynamic>}),
    jsonEncode({'jsonrpc': '2.0', 'id': 4, 'error': {'code': -32603, 'message': 'get-version: it broke', 'data': jsonDecode(fault('handler-panic'))}}),
    jsonEncode({'jsonrpc': '2.0', 'id': 5, 'error': {'code': -32601, 'message': 'Method not found'}}),
    'this is no JSON',
    Object(),
    result(8, {'content': [], 'structuredContent': {'version': 'seven'}, 'isError': false}),
  ]);
  final client = DartDocumentServiceMcpClient(canned);
  for (var call = 0; call < 8; call += 1) {
    lines.add(summary(await client.getVersion(McpVersionRequest(document_id: 'doc-1'))));
  }

  final more = Canned([
    result(1, {'content': [{'type': 'text', 'text': '{"version":12}'}], 'structuredContent': {'version': 12}, 'isError': false}),
    result(2, {'content': [{'type': 'text', 'text': '{}'}], 'structuredContent': {'value': {'version': 7}, 'etag': '"v7"'}, 'isError': false}),
    result(3, {'content': [], 'isError': false}),
    result(4, {'content': [], 'isError': false}),
    result(5, toolText(jsonDecode(fault('failed-validation'))).isEmpty ? {} : jsonDecode(toolText(jsonDecode(fault('failed-validation')))) as Map<String, dynamic>),
    Object(),
  ]);
  final other = DartDocumentServiceMcpClient(more);
  final lookup = await other.lookup(McpDocumentId('doc-12'));
  final tagged = await other.getTagged('doc-1', '"v6"');
  final renamed = await other.renameDocument(RenameDocumentRequest(document_id: 'doc-1', title: 'Plan'));
  await other.touchDocument(TouchDocumentRequest(document_id: 'doc-1', reader: 'reviewer-7'));
  var refusal = '';
  try {
    await other.touchDocument(TouchDocumentRequest(document_id: 'doc-1', reader: 'reviewer-7'));
  } on DartDocumentServiceMcpRefusal catch (refused) {
    refusal = refused.fault.kind.wireValue;
  }
  var down = '';
  try {
    await other.touchDocument(TouchDocumentRequest(document_id: 'doc-1', reader: 'reviewer-7'));
  } on DartDocumentServiceMcpRefusal catch (refused) {
    down = refused.fault.kind.wireValue;
  }

  print(jsonEncode({
    'lines': lines,
    'sent': [
      for (final message in [...canned.sent.take(1), ...more.sent])
        {'text': message.text, 'protocolVersion': message.protocolVersion, 'method': message.method, 'name': message.name},
    ],
    'lookup': (lookup as DartDocumentServiceLookupResultOk).value.version,
    'tagged': (() {
      final ok = (tagged as DartDocumentServiceGetTaggedResultOk).value;
      return [ok.$1.version, ok.$2];
    })(),
    'renamed': renamed is DartDocumentServiceRenameDocumentResultOk,
    'refusal': refusal,
    'down': down,
  }));
}
"#;

/// A client wired to the server of the same service, with a header argument named after each local
/// the emitted code writes around it.
const LOOPBACK_DRIVER: &str = "
class Loopback implements TaggingServiceMcpTransport {
  Loopback(this.serve);
  final Future<String?> Function(String text) serve;

  @override
  Future<String> send(({String text, String protocolVersion, String method, String? name}) message) async =>
      (await serve(message.text))!;
}

void main() async {
  final seen = <String>[];
  final handlers = TaggingServiceHandlers<void>(
    retag: (ctx, req, arguments_, result_, members_, failure_) async {
      seen.add([req, arguments_, result_, members_, failure_].join(':'));
      if (req == 'missing') throw const McpVersionErrorNotFound();
      return seen.last;
    },
  );
  final serve = createTaggingServiceMcpDispatcher<void>(handlers);
  final inFlight = TaggingServiceMcpInFlight();
  final client = TaggingServiceMcpClient(Loopback((text) => serve(text, (call) {}, (message) async {}, inFlight)));
  final ok = await client.retag('doc-1', 'a', 'r', 'm', 'f');
  final declared = await client.retag('missing', 'a', 'r', null, 'f');
  print(jsonEncode({
    'ok': ok is TaggingServiceRetagResultOk ? ok.value : null,
    'declared': declared is TaggingServiceRetagResultOperation ? declared.error.toJson() : null,
    'seen': seen,
  }));
}
";

/// The three lines of a stdio host that starts each line's answer without waiting for the one
/// before, a write that fails, and the HTTP requests that answer an event stream. A wait ends on
/// the event it waits for.
const WRITER_DRIVER: &str = "
class RecordingStream implements WriterServiceMcpHttpStream {
  RecordingStream([this.goneAfter]);
  final int? goneAfter;
  int attempts = 0;
  final List<Map<String, dynamic>> seen = <Map<String, dynamic>>[];
  final List<Completer<void>> waiting = <Completer<void>>[];

  void wake() {
    final woken = waiting.toList();
    waiting.clear();
    for (final one in woken) {
      one.complete();
    }
  }

  Future<void> until(bool Function() done) async {
    while (!done()) {
      final next = Completer<void>();
      waiting.add(next);
      await next.future;
    }
  }

  void take() {
    attempts += 1;
    final refused = goneAfter != null && seen.length >= goneAfter!;
    wake();
    if (refused) throw StateError('the client has gone');
  }

  @override
  Future<void> begin(int status, List<(String, String)> headers) async {
    take();
    seen.add({
      'began': [status, [for (final (name, value) in headers) [name, value]]],
    });
    wake();
  }

  @override
  Future<void> write(String event) async {
    take();
    seen.add({'wrote': event});
    wake();
  }
}

void main() async {
  final input = jsonDecode(inputText) as Map<String, dynamic>;
  final written = <String>[];
  final waiting = <Completer<void>>[];
  Future<void> stdout(String message) async {
    written.add(message);
    final woken = waiting.toList();
    waiting.clear();
    for (final wake in woken) {
      wake.complete();
    }
  }

  Future<void> until(bool Function() done) async {
    while (!done()) {
      final next = Completer<void>();
      waiting.add(next);
      await next.future;
    }
  }

  final inFlight = WriterServiceMcpInFlight();
  final released = Completer<void>();
  var observed = false;
  final handlers = WriterServiceHandlers<WriterServiceMcpCall>(
    exportFolder: (ctx, folder) async {
      await ctx.progress(1, total: 3, message: 'exported 1 of 3 files');
      if (folder == 'forever') {
        await released.future;
        observed = ctx.cancelled;
      }
    },
    getVersion: (ctx, req) async => WriterVersion(version: 7),
  );
  final dispatch = createWriterServiceMcpDispatcher<WriterServiceMcpCall>(handlers);
  final running = <Future<void>>[];
  void start(String line) {
    running.add(dispatch(line, (call) => call, stdout, inFlight).then((reply) async {
      if (reply != null) await stdout(reply);
    }));
  }

  final lines = (input['lines'] as List).cast<String>();
  start(lines[0]);
  await until(() => written.isNotEmpty);
  start(lines[1]);
  await until(() => written.length >= 2);
  final stopped = await dispatch(lines[2], (call) => call, stdout, inFlight);
  if (stopped != null) throw StateError('a notification was answered');
  released.complete();
  await Future.wait(running);

  WriterServiceMcpCall? seen;
  final reply = await dispatch(input['failed'] as String, (call) {
    seen = call;
    return call;
  }, (message) async => throw StateError('the client has gone'), inFlight);
  final failed = <String, dynamic>{'reply': reply, 'cancelled': seen!.cancelled, 'written': <String>[]};

  final serve = createWriterServiceMcpHttpDispatcher<WriterServiceMcpCall>(
    handlers,
    const WriterServiceMcpHttpSettings(allowedOrigins: []),
    (request, call) async => (ctx: call, denied: null, serverName: null),
  );
  final http = <Map<String, dynamic>>[];
  for (final row in (input['http'] as List).cast<Map<String, dynamic>>()) {
    final response = await respondWriterService(serve, WriterServiceMcpHttpRequest(
      method: row['method'] as String,
      path: row['path'] as String,
      headers: [for (final pair in row['headers'] as List) (pair[0] as String, pair[1] as String)],
      body: utf8.encode(row['body'] as String),
    ));
    http.add({
      'status': response.status,
      'headers': [for (final (name, value) in response.headers) [name, value]],
      'body': utf8.decode(response.body),
    });
  }
  final rows = input['streams'] as Map<String, dynamic>;
  WriterServiceMcpHttpRequest asRequest(String name) {
    final row = rows[name] as Map<String, dynamic>;
    return WriterServiceMcpHttpRequest(
      method: row['method'] as String,
      path: row['path'] as String,
      headers: [for (final pair in row['headers'] as List) (pair[0] as String, pair[1] as String)],
      body: utf8.encode(row['body'] as String),
    );
  }

  WriterServiceHandlers<WriterServiceMcpCall> heldHandlers(Future<void> opened, List<int> reached) =>
      WriterServiceHandlers<WriterServiceMcpCall>(
        exportFolder: (ctx, folder) async {
          reached[0] += 1;
          await ctx.progress(1, total: 3, message: 'exported 1 of 3 files');
          if (folder == 'held') await opened;
        },
        getVersion: (ctx, req) async => WriterVersion(version: 7),
      );

  Future<WriterServiceMcpHttpResponse?> Function(WriterServiceMcpHttpRequest, WriterServiceMcpHttpStream) serving(
    WriterServiceHandlers<WriterServiceMcpCall> held,
    List<WriterServiceMcpCall?> taken,
  ) =>
      createWriterServiceMcpHttpDispatcher<WriterServiceMcpCall>(
        held,
        const WriterServiceMcpHttpSettings(allowedOrigins: []),
        (request, call) async {
          taken[0] = call;
          return (ctx: call, denied: null, serverName: null);
        },
      );

  final streams = <String, dynamic>{};
  {
    final gate = Completer<void>();
    final stream = RecordingStream();
    var ended = false;
    final running = serving(heldHandlers(gate.future, [0]), [null])(asRequest('held'), stream).then((answered) {
      ended = true;
      return answered;
    });
    await stream.until(() => stream.seen.length >= 2);
    final held = stream.seen.toList();
    final endedWhileHeld = ended;
    gate.complete();
    final answered = await running;
    streams['progress'] = {
      'endedWhileHeld': endedWhileHeld,
      'full': stream.seen.toList(),
      'held': held,
      'returned': answered != null,
    };
  }
  {
    final gate = Completer<void>();
    final stream = RecordingStream(1);
    final taken = <WriterServiceMcpCall?>[null];
    final running = serving(heldHandlers(gate.future, [0]), taken)(asRequest('held'), stream);
    await stream.until(() => stream.attempts >= 2);
    gate.complete();
    final answered = await running;
    streams['failed'] = {'cancelled': taken[0]!.cancelled, 'returned': answered != null, 'seen': stream.seen.toList()};
  }
  {
    final stream = RecordingStream(0);
    final taken = <WriterServiceMcpCall?>[null];
    final reached = [0];
    final answered = await serving(heldHandlers(Future<void>.value(), reached), taken)(asRequest('docs'), stream);
    streams['refused'] = {
      'cancelled': taken[0]!.cancelled,
      'reached': reached[0],
      'returned': answered != null,
      'seen': stream.seen.toList(),
    };
  }
  {
    final stream = RecordingStream();
    final answered = await serving(heldHandlers(Future<void>.value(), [0]), [null])(asRequest('plain'), stream);
    streams['plain'] = {'returned': answered != null, 'seen': stream.seen.toList(), 'status': answered!.status};
  }
  print(jsonEncode({'written': written, 'observed': observed, 'failed': failed, 'http': http, 'streams': streams}));
}
";

/// A one-way handler that throws, served with a callback and without one, and a replying handler
/// that throws beside them.
const ONE_WAY_DRIVER: &str = r#"
const meta = {
  'io.modelcontextprotocol/protocolVersion': '2026-07-28',
  'io.modelcontextprotocol/clientCapabilities': <String, dynamic>{},
};

String call(int id, String name, Map<String, dynamic> arguments) => jsonEncode({
      'jsonrpc': '2.0',
      'id': id,
      'method': 'tools/call',
      'params': {'name': name, 'arguments': arguments, '_meta': meta},
    });

void main() async {
  final documents = DartDocumentServiceHandlers<void>(
    checkDrafts: (ctx, req) async => McpVersionResponse(version: req.drafts.length),
    checkMany: (ctx, req) async => McpVersionResponse(version: 7),
    checkMaybe: (ctx, req) async => McpVersionResponse(version: 7),
    checkNamed: (ctx, req) async => McpVersionResponse(version: 7),
    getTagged: (ctx, req, known_tag) async => (McpVersionResponse(version: 7), '"v7"'),
    getVersion: (ctx, req) async => throw 'the handler came apart',
    lookup: (ctx, req) async => McpVersionResponse(version: 7),
    renameDocument: (ctx, req) async {},
    touchDocument: (ctx, req) async => throw 'the handler came apart',
  );
  final faults = <Map<String, dynamic>>[];
  final touch = call(1, 'touch-document', {'documentId': 'doc-1', 'reader': 'boom'});
  Future<String?> serve(DartDocumentServiceHandlers<void> held, String text, [void Function(DartDocumentServiceFaultFields)? onFault]) =>
      createDartDocumentServiceMcpDispatcher<void>(held, onFault)(text, (call) {}, (message) async {}, DartDocumentServiceMcpInFlight());
  final heard = await serve(documents, touch, (fault) => faults.add(fault.toJson()));
  final replying = await serve(documents, call(2, 'get-version', {'document_id': 'doc-1'}), (fault) => faults.add(fault.toJson()));
  final routed = await createDartDocumentServiceMcpRoute<void>(documents, (fault) => faults.add(fault.toJson()))(
    null,
    'tools/call',
    {'name': 'touch-document', 'arguments': {'documentId': 'doc-1', 'reader': 'boom'}},
  );
  final silent = await serve(documents, touch);
  print(jsonEncode({'faults': faults, 'heard': heard, 'replying': replying, 'routed': routed?.result, 'silent': silent}));
}
"#;

/// The imports, the models, the document and billing services and their clients, and `driver`.
fn module(driver: &str) -> String {
    [
        "import 'dart:async';".to_owned(),
        "import 'dart:convert';".to_owned(),
        "import 'dart:io';".to_owned(),
        mcp_format_dart::dart_definition(),
        mcp_reference_dart::dart_definition(),
        mcp_draft_dart::dart_definition(),
        mcp_draft_batch_dart::dart_definition(),
        mcp_version_request_dart::dart_definition(),
        mcp_version_response_dart::dart_definition(),
        mcp_version_error_dart::dart_definition(),
        mcp_document_id_dart::dart_definition(),
        mcp_invoice_request_dart::dart_definition(),
        mcp_invoice_list_dart::dart_definition(),
        DartDocumentServiceSchema::dart_definition(),
        DartDocumentServiceSchema::dart_ws_client(),
        DartDocumentServiceSchema::dart_mcp_service(),
        DartDocumentServiceSchema::dart_mcp_client(),
        DartBillingServiceSchema::dart_definition(),
        DartBillingServiceSchema::dart_ws_client(),
        DartBillingServiceSchema::dart_mcp_service(),
        DartBillingServiceSchema::dart_mcp_client(),
        TaggingServiceSchema::dart_definition(),
        TaggingServiceSchema::dart_ws_client(),
        TaggingServiceSchema::dart_mcp_service(),
        TaggingServiceSchema::dart_mcp_client(),
        writer_version_request_dart::dart_definition(),
        writer_version_dart::dart_definition(),
        writer_refusal_dart::dart_definition(),
        WriterServiceSchema::dart_definition(),
        WriterServiceSchema::dart_ws_client(),
        WriterServiceSchema::dart_mcp_service(),
        dart_respond("DartDocumentService"),
        dart_respond("WriterService"),
        driver.replace("/*FAULT_CASES*/", &cases_json()),
    ]
    .join("\n\n")
}

fn driven(named: &str, driver: &str) -> Option<Value> {
    let wrote = ran(
        "dart",
        RUNTIME_VAR,
        "dart",
        &format!("{named}.dart"),
        &module(driver),
    )?;
    Some(serde_json::from_str(wrote.trim()).unwrap())
}

/// Every error the vendored definition `def` finds in `message`.
fn errors_of(def: &str, message: &Value) -> Vec<String> {
    let schema: Value = serde_json::from_str(MCP_SCHEMA).unwrap();
    let rooted = json!({ "$ref": format!("#/$defs/{def}"), "$defs": schema["$defs"] });
    let validator = jsonschema::validator_for(&rooted).unwrap();
    validator
        .iter_errors(message)
        .map(|error| format!("{error} at {}", error.instance_path()))
        .collect()
}

fn valid(def: &str, message: &Value) {
    if def.ends_with("Error") && !WHOLE_ERRORS.contains(&def) {
        let errors = errors_of(def, &message["error"]);
        assert!(errors.is_empty(), "{def}: {errors:#?}\n{message:#}");
        valid("JSONRPCErrorResponse", message);
        return;
    }
    let errors = errors_of(def, message);
    assert!(errors.is_empty(), "{def}: {errors:#?}\n{message:#}");
}

/// An error answer to a message that has no readable id carries no `id` member, since the
/// official schema allows only an integer or a string there.
fn valid_without_an_id(def: &str, message: &Value) {
    assert!(message.get("id").is_none(), "{message}");
    valid(def, message);
}

/// `value` written with its keys in order, as a server that sorts them writes it.
fn canonical(value: &Value) -> String {
    match value {
        Value::Object(members) => {
            let mut keys: Vec<&String> = members.keys().collect();
            keys.sort();
            let written: Vec<String> = keys
                .into_iter()
                .map(|key| {
                    format!(
                        "{}:{}",
                        Value::String(key.clone()),
                        canonical(&members[key])
                    )
                })
                .collect();
            format!("{{{}}}", written.join(","))
        }
        Value::Array(items) => {
            let written: Vec<String> = items.iter().map(canonical).collect();
            format!("[{}]", written.join(","))
        }
        scalar @ (Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_)) => {
            scalar.to_string()
        }
    }
}

/// The answer called `name`, held to `def` and to the bytes `expected` writes.
fn answered(driven: &Value, name: &str, def: &str, expected: &Value) {
    let text = driven["answers"][name].as_str().unwrap();
    let message: Value = serde_json::from_str(text).unwrap();
    valid(def, &message);
    assert_eq!(text, canonical(expected), "{name}");
}

/// A list of tools answered as `result.tools`, each tool kept as the text Rust wrote.
fn tools_answer(id: i32, name: &str, version: &str, tools: &[Value]) -> String {
    let info = canonical(&json!({ "io.modelcontextprotocol/serverInfo":
        { "name": name, "version": version } }));
    let texts: Vec<String> = tools.iter().map(Value::to_string).collect();
    format!(
        "{{\"id\":{id},\"jsonrpc\":\"2.0\",\"result\":{{\"_meta\":{info},\"cacheScope\":\"public\",\
         \"resultType\":\"complete\",\"tools\":[{}],\"ttlMs\":300000}}}}",
        texts.join(",")
    )
}

/// `name`, held to `def` and to `expected` as text.
fn answered_text(driven: &Value, name: &str, def: &str, expected: &str) {
    let text = driven["answers"][name].as_str().unwrap();
    valid(def, &serde_json::from_str(text).unwrap());
    assert_eq!(text, expected, "{name}");
}

fn info() -> Value {
    json!({ "io.modelcontextprotocol/serverInfo": { "name": "Workspace", "version": VERSION } })
}

fn complete(id: &Value, mut result: Value) -> Value {
    result["resultType"] = json!("complete");
    result["_meta"] = info();
    json!({ "id": id, "jsonrpc": "2.0", "result": result })
}

fn text_of(value: &Value) -> String {
    canonical(value)
}

fn fault_envelope(kind: &str, operation: &str, detail: &str, field: Option<&str>) -> Value {
    let mut fault = json!({ "detail": detail, "kind": kind, "operation": operation });
    if let Some(named) = field {
        fault["field"] = json!(named);
    }
    json!({ "fault": fault, "isServiceFault": true })
}

fn failed(id: &Value, carried: &Value) -> Value {
    complete(
        id,
        json!({ "content": [{ "text": text_of(carried), "type": "text" }], "isError": true }),
    )
}

#[test]
fn the_server_answers_the_messages_of_the_design_on_the_bytes_rust_writes() {
    let Some(driven) = driven("mcp_server", SERVER_DRIVER) else {
        return;
    };
    let mut tools = DartDocumentServiceSchema::mcp_tools();
    tools.extend(DartBillingServiceSchema::mcp_tools());

    answered(
        &driven,
        "discover",
        "DiscoverResultResponse",
        &json!({ "id": "discover-1", "jsonrpc": "2.0", "result": {
            "_meta": info(),
            "cacheScope": "public",
            "capabilities": { "resources": {}, "tools": {} },
            "resultType": "complete",
            "supportedVersions": ["2026-07-28"],
            "ttlMs": 300_000_i32,
        } }),
    );
    answered_text(
        &driven,
        "tools_list",
        "ListToolsResultResponse",
        &tools_answer(1, "Workspace", VERSION, &tools),
    );
    answered(
        &driven,
        "success",
        "CallToolResultResponse",
        &complete(
            &json!(2_i32),
            json!({
                "content": [{ "text": "{\"version\":7}", "type": "text" }],
                "isError": false,
                "structuredContent": { "version": 7_i32 },
            }),
        ),
    );
    answered(
        &driven,
        "declared",
        "CallToolResultResponse",
        &failed(&json!(4_i32), &json!({ "errorCode": "not-found" })),
    );
    answered(
        &driven,
        "refused",
        "CallToolResultResponse",
        &failed(
            &json!(6_i32),
            &fault_envelope(
                "failed-validation",
                "get-version",
                "'document_id': too short: minimum length is 1, got 0",
                Some("document_id"),
            ),
        ),
    );
    answered(
        &driven,
        "second_service",
        "CallToolResultResponse",
        &complete(
            &json!(7_i32),
            json!({
                "content": [{ "text": "{\"invoices\":[\"inv-1\",\"acme\"]}", "type": "text" }],
                "isError": false,
                "structuredContent": { "invoices": ["inv-1", "acme"] },
            }),
        ),
    );
}

#[test]
fn what_the_server_cannot_serve_answers_the_error_the_design_names() {
    let Some(driven) = driven("mcp_server_errors", SERVER_DRIVER) else {
        return;
    };
    answered(
        &driven,
        "unknown_tool",
        "InvalidParamsError",
        &json!({ "error": {
            "code": -32602_i32,
            "data": fault_envelope(
                "unknown-operation",
                "delete-everything",
                "the service answers to no operation by that name",
                None,
            ),
            "message": "Unknown tool: delete-everything",
        }, "id": 8_i32, "jsonrpc": "2.0" }),
    );
    assert!(driven["answers"]["notification"].is_null());
    answered(
        &driven,
        "old_version",
        "UnsupportedProtocolVersionError",
        &json!({ "error": {
            "code": -32022_i32,
            "data": { "requested": "2025-11-25", "supported": ["2026-07-28"] },
            "message": "Unsupported protocol version",
        }, "id": 9_i32, "jsonrpc": "2.0" }),
    );
    answered(
        &driven,
        "unknown_method",
        "MethodNotFoundError",
        &json!({ "error": { "code": -32601_i32, "message": "Method not found: tools/unheard" },
            "id": 10_i32, "jsonrpc": "2.0" }),
    );
    answered(
        &driven,
        "resource",
        "ReadResourceResultResponse",
        &complete(
            &json!(11_i32),
            json!({
                "cacheScope": "private",
                "contents": [{ "mimeType": "application/json", "text": "{\"total\":120}",
                    "uri": "billing://invoices/inv-1" }],
                "ttlMs": 0_i32,
            }),
        ),
    );
    answered(
        &driven,
        "templates",
        "ListResourceTemplatesResultResponse",
        &json!({ "id": 12_i32, "jsonrpc": "2.0", "result": {
            "_meta": info(),
            "cacheScope": "public",
            "resourceTemplates": [{ "name": "documents",
                "uriTemplate": "workspace://documents/{document_id}" }],
            "resultType": "complete",
            "ttlMs": 300_000_i32,
        } }),
    );
    answered(
        &driven,
        "prompts",
        "ListPromptsResultResponse",
        &json!({ "id": 25_i32, "jsonrpc": "2.0", "result": {
            "_meta": info(),
            "cacheScope": "public",
            "prompts": [],
            "resultType": "complete",
            "ttlMs": 300_000_i32,
        } }),
    );
}

#[test]
fn the_server_answers_what_a_handler_does_and_what_a_message_holds() {
    let Some(driven) = driven("mcp_server_calls", SERVER_DRIVER) else {
        return;
    };
    answered(
        &driven,
        "panic",
        "InternalError",
        &json!({ "error": {
            "code": -32603_i32,
            "data": fault_envelope(
                "handler-panic",
                "get-version",
                "the handler came apart",
                None,
            ),
            "message": "get-version: the handler came apart",
        }, "id": 13_i32, "jsonrpc": "2.0" }),
    );
    answered(
        &driven,
        "one_way",
        "CallToolResultResponse",
        &complete(&json!(14_i32), json!({ "content": [], "isError": false })),
    );
    assert_eq!(driven["touched"], json!(["doc-1:reviewer-7"]));
    answered(
        &driven,
        "wrapped",
        "CallToolResultResponse",
        &complete(
            &json!(15_i32),
            json!({
                "content": [{ "text": "{\"version\":6}", "type": "text" }],
                "isError": false,
                "structuredContent": { "version": 6_i32 },
            }),
        ),
    );
    answered(
        &driven,
        "headers",
        "CallToolResultResponse",
        &complete(
            &json!(16_i32),
            json!({
                "content": [{ "text": "{\"etag\":\"\\\"v7\\\"\",\"value\":{\"version\":7}}",
                    "type": "text" }],
                "isError": false,
                "structuredContent": { "etag": "\"v7\"", "value": { "version": 7_i32 } },
            }),
        ),
    );
    assert_eq!(driven["tags"], json!(["\"v6\"", null]));
    answered(
        &driven,
        "unit",
        "CallToolResultResponse",
        &complete(&json!(19_i32), json!({ "content": [], "isError": false })),
    );
}

/// The fault a `tools/call` answered as a tool error, from its text.
fn fault_of(driven: &Value, name: &str) -> Value {
    let message: Value = serde_json::from_str(driven["answers"][name].as_str().unwrap()).unwrap();
    valid("CallToolResultResponse", &message);
    assert_eq!(message["result"]["isError"], true, "{name}");
    let text = message["result"]["content"][0]["text"].as_str().unwrap();
    let carried: Value = serde_json::from_str(text).unwrap();
    assert_eq!(carried["isServiceFault"], true, "{name}");
    carried["fault"].clone()
}

#[test]
fn a_message_that_does_not_read_answers_the_fault_of_what_went_wrong() {
    let Some(driven) = driven("mcp_server_faults", SERVER_DRIVER) else {
        return;
    };
    let unread = fault_of(&driven, "unread");
    assert_eq!(unread["kind"], "failed-validation");
    assert_eq!(unread["operation"], "get-version");
    let missing = fault_of(&driven, "no_arguments");
    assert_eq!(missing["kind"], "failed-validation");
    assert_eq!(missing["field"], "document_id");
    let bad_header = fault_of(&driven, "bad_header");
    assert_eq!(bad_header["kind"], "failed-validation");
    assert_eq!(bad_header["field"], "if-none-match");
    assert_eq!(bad_header["operation"], "get-tagged");
    let absent: Value =
        serde_json::from_str(driven["answers"]["no_header"].as_str().unwrap()).unwrap();
    assert_eq!(absent["result"]["isError"], false);
}

#[test]
fn the_faults_of_the_server_carry_the_kind_field_and_tool_the_rust_dispatcher_writes() {
    let Some(driven) = driven("mcp_server_kinds", SERVER_DRIVER) else {
        return;
    };
    for (at, case) in cases::FAULT_CASES.iter().enumerate() {
        let message: Value =
            serde_json::from_str(driven["answers"][format!("fault_{at}")].as_str().unwrap())
                .unwrap();
        valid(
            if case.kind == "handler-panic" {
                "InternalError"
            } else {
                "CallToolResultResponse"
            },
            &message,
        );
        case.holds(&message);
    }
    let unknown_variant = fault_of(&driven, "unknown_variant");
    assert_eq!(unknown_variant["kind"], "failed-validation");
    assert_eq!(unknown_variant["field"], "format");
    assert_eq!(unknown_variant["operation"], "get-version");
}

#[test]
fn a_request_missing_its_meta_or_unreadable_answers_the_error_the_design_names() {
    let Some(driven) = driven("mcp_server_meta", SERVER_DRIVER) else {
        return;
    };
    let no_meta: Value =
        serde_json::from_str(driven["answers"]["no_meta"].as_str().unwrap()).unwrap();
    valid("InvalidParamsError", &no_meta);
    assert_eq!(no_meta["error"]["code"], -32_602_i32);
    assert_eq!(no_meta["id"], 20_i32);
    let unreadable: Value =
        serde_json::from_str(driven["answers"]["not_json"].as_str().unwrap()).unwrap();
    valid_without_an_id("InvalidRequestError", &unreadable);
    assert_eq!(unreadable["error"]["code"], -32_600_i32);
}

#[test]
fn resources_are_listed_a_page_at_a_time_across_the_services_of_an_endpoint() {
    let Some(driven) = driven("mcp_server_pages", SERVER_DRIVER) else {
        return;
    };
    let page = |id: i32, next: Option<&str>, resources: Value| {
        let mut result = json!({
            "_meta": info(),
            "cacheScope": "private",
            "resources": resources,
            "resultType": "complete",
            "ttlMs": 0_i32,
        });
        if let Some(cursor) = next {
            result["nextCursor"] = json!(cursor);
        }
        json!({ "id": id, "jsonrpc": "2.0", "result": result })
    };
    answered(
        &driven,
        "resources_first",
        "ListResourcesResultResponse",
        &page(
            21_i32,
            Some("0:page-2"),
            json!([
                { "mimeType": "text/markdown", "name": "configuration-guide",
                  "uri": "guide://configure-organization" },
                { "name": "id-card.pdf", "uri": "workspace://documents/doc-1" },
            ]),
        ),
    );
    answered(
        &driven,
        "resources_second",
        "ListResourcesResultResponse",
        &page(
            22_i32,
            Some("1:"),
            json!([{ "name": "passport.pdf", "uri": "workspace://documents/doc-2" }]),
        ),
    );
    answered(
        &driven,
        "resources_last",
        "ListResourcesResultResponse",
        &page(
            23_i32,
            None,
            json!([{ "name": "inv-1", "uri": "billing://invoices/inv-1" }]),
        ),
    );
    answered(
        &driven,
        "resources_invalid",
        "InvalidParamsError",
        &json!({ "error": { "code": -32602_i32, "message": "Invalid cursor" },
            "id": 24_i32, "jsonrpc": "2.0" }),
    );
}

#[test]
fn a_one_way_handler_that_throws_reaches_the_callback_or_standard_error() {
    let Some((written, standard_error)) = ran_with_stderr(
        "dart",
        RUNTIME_VAR,
        "dart",
        "mcp_one_way.dart",
        &module(ONE_WAY_DRIVER),
    ) else {
        return;
    };
    let report: Value = serde_json::from_str(written.trim()).unwrap();
    let empty = |reply: &Value| {
        let parsed: Value = serde_json::from_str(reply.as_str().unwrap()).unwrap();
        assert_eq!(parsed["result"]["isError"], false, "{parsed:#}");
        assert_eq!(parsed["result"]["content"], json!([]), "{parsed:#}");
    };
    empty(&report["heard"]);
    empty(&report["silent"]);
    assert_eq!(report["routed"]["isError"], false, "{report:#}");
    let faults = report["faults"].as_array().unwrap();
    assert_eq!(
        faults.len(),
        2,
        "one for the dispatcher, one for the route: {report:#}"
    );
    for fault in faults {
        assert_eq!(fault["kind"], "handler-panic", "{fault:#}");
        assert_eq!(fault["operation"], "touch-document", "{fault:#}");
        assert_eq!(fault["detail"], "the handler came apart", "{fault:#}");
    }
    let replying: Value = serde_json::from_str(report["replying"].as_str().unwrap()).unwrap();
    assert_eq!(replying["error"]["code"], -32_603_i32, "{replying:#}");
    let printed: Vec<Value> = standard_error
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    assert_eq!(printed.len(), 1, "{standard_error}");
    assert_eq!(printed[0]["kind"], "handler-panic", "{standard_error}");
    assert_eq!(
        printed[0]["operation"], "touch-document",
        "{standard_error}"
    );
}

#[test]
fn a_service_served_alone_answers_its_own_tools() {
    let Some(driven) = driven("mcp_server_alone", SERVER_DRIVER) else {
        return;
    };
    answered_text(
        &driven,
        "alone",
        "ListToolsResultResponse",
        &tools_answer(
            27,
            "DartDocumentService",
            VERSION,
            &DartDocumentServiceSchema::mcp_tools(),
        ),
    );
}

#[test]
fn the_http_rules_answer_the_status_and_the_error_each_names() {
    let Some(driven) = driven("mcp_http", HTTP_DRIVER) else {
        return;
    };
    let status = |name: &str| driven[name]["status"].as_i64().unwrap();
    let body = |name: &str| -> Value {
        let text = driven[name]["body"].as_str().unwrap();
        serde_json::from_str(text).unwrap()
    };
    for accepted in [
        "success",
        "lowercase_names",
        "base64_name",
        "unknown_tool",
        "allowed_origin",
    ] {
        assert_eq!(status(accepted), 200, "{accepted}");
        assert_eq!(
            driven[accepted]["headers"],
            json!(["Content-Type: application/json"]),
            "{accepted}"
        );
        let message = body(accepted);
        valid(
            if message.get("error").is_some() {
                "JSONRPCErrorResponse"
            } else {
                "CallToolResultResponse"
            },
            &message,
        );
    }
    assert_eq!(
        body("success")["result"]["structuredContent"],
        json!({"version": 7_i32})
    );
    assert_eq!(body("unknown_tool")["error"]["code"], -32_602_i32);
    assert_eq!(status("get"), 405);
    assert_eq!(driven["get"]["headers"], json!(["Allow: POST"]));
    assert_eq!(driven["get"]["body"], "");
    assert_eq!(status("foreign_origin"), 403);
    assert_eq!(driven["foreign_origin"]["body"], "");
    assert_eq!(status("unreadable"), 400);
    valid_without_an_id("InvalidRequestError", &body("unreadable"));
    assert_eq!(status("notification"), 202);
    assert_eq!(driven["notification"]["body"], "");
    for mismatched in [
        "no_version_header",
        "method_differs",
        "name_missing",
        "name_differs",
    ] {
        assert_eq!(status(mismatched), 400, "{mismatched}");
        valid("HeaderMismatchError", &body(mismatched));
        assert_eq!(
            body(mismatched)["error"]["code"],
            -32_020_i32,
            "{mismatched}"
        );
    }
    assert_eq!(status("no_meta"), 400);
    assert_eq!(body("no_meta")["error"]["code"], -32_602_i32);
    assert_eq!(status("old_version"), 400);
    valid("UnsupportedProtocolVersionError", &body("old_version"));
    assert_eq!(status("unknown_method"), 404);
    valid("MethodNotFoundError", &body("unknown_method"));
}

#[test]
fn the_client_reads_the_seven_outcomes_and_writes_each_call_as_the_server_reads_it() {
    let Some(driven) = driven("mcp_client", CLIENT_DRIVER) else {
        return;
    };
    assert_eq!(
        driven["lines"],
        json!([
            "ok version=7",
            "declared {\"errorCode\":\"not-found\"}",
            "fault failed-validation",
            "fault handler-panic",
            "fault transport-failure",
            "fault undeserializable-payload",
            "fault transport-failure",
            "fault undeserializable-payload",
        ])
    );
    let sent = driven["sent"].as_array().unwrap();
    let first = &sent[0];
    assert_eq!(first["method"], "tools/call");
    assert_eq!(first["name"], "get-version");
    assert_eq!(first["protocolVersion"], "2026-07-28");
    let request: Value = serde_json::from_str(first["text"].as_str().unwrap()).unwrap();
    assert_eq!(request["id"], 1_i32);
    assert_eq!(request["method"], "tools/call");
    assert_eq!(request["params"]["name"], "get-version");
    assert_eq!(
        request["params"]["arguments"],
        json!({"document_id": "doc-1"})
    );
    assert_eq!(
        request["params"]["_meta"],
        json!({
            "io.modelcontextprotocol/clientCapabilities": {},
            "io.modelcontextprotocol/clientInfo": {
                "name": "DartDocumentServiceMcpClient",
                "version": VERSION,
            },
            "io.modelcontextprotocol/protocolVersion": "2026-07-28",
        })
    );
    let argument = |at: usize| -> Value {
        let carried: Value = serde_json::from_str(sent[at]["text"].as_str().unwrap()).unwrap();
        carried["params"]["arguments"].clone()
    };
    assert_eq!(argument(1), json!({"value": "doc-12"}));
    assert_eq!(sent[1]["name"], "lookup-document");
    assert_eq!(
        argument(2),
        json!({"if-none-match": "\"v6\"", "value": "doc-1"})
    );
    assert_eq!(argument(3), json!({"documentId": "doc-1", "title": "Plan"}));
    assert_eq!(
        argument(4),
        json!({"documentId": "doc-1", "reader": "reviewer-7"})
    );
    assert_eq!(driven["lookup"], 12_i32);
    assert_eq!(driven["tagged"], json!([7_i32, "\"v7\""]));
    assert_eq!(driven["renamed"], true);
    assert_eq!(driven["refusal"], "failed-validation");
    assert_eq!(driven["down"], "transport-failure");
}

#[test]
fn a_header_argument_named_after_a_generated_local_still_reaches_the_handler_through_both_halves() {
    let Some(driven) = driven("mcp_loopback", LOOPBACK_DRIVER) else {
        return;
    };
    assert_eq!(driven["ok"], "doc-1:a:r:m:f");
    assert_eq!(driven["declared"], json!({"errorCode": "not-found"}));
    assert_eq!(
        driven["seen"],
        json!(["doc-1:a:r:m:f", "missing:a:r:null:f"])
    );
}

#[test]
fn a_const_endpoint_whose_services_share_a_name_does_not_compile() {
    let endpoint = |second: &str| {
        [
            "import 'dart:async';".to_owned(),
            "import 'dart:convert';".to_owned(),
            "import 'dart:io';".to_owned(),
            mcp_format_dart::dart_definition(),
            mcp_reference_dart::dart_definition(),
            mcp_draft_dart::dart_definition(),
            mcp_draft_batch_dart::dart_definition(),
            mcp_version_request_dart::dart_definition(),
            mcp_version_response_dart::dart_definition(),
            mcp_version_error_dart::dart_definition(),
            mcp_document_id_dart::dart_definition(),
            mcp_invoice_request_dart::dart_definition(),
            mcp_invoice_list_dart::dart_definition(),
            DartDocumentServiceSchema::dart_definition(),
            DartDocumentServiceSchema::dart_ws_client(),
            DartDocumentServiceSchema::dart_mcp_service(),
            DartBillingServiceSchema::dart_definition(),
            DartBillingServiceSchema::dart_ws_client(),
            DartBillingServiceSchema::dart_mcp_service(),
            AuditServiceSchema::dart_definition(),
            AuditServiceSchema::dart_ws_client(),
            AuditServiceSchema::dart_mcp_service(),
            format!(
                "const workspace = DartDocumentServiceMcpEndpoint(\n  \
                 {{...dartDocumentServiceMcpNames, ...{second}ServiceMcpNames}},\n  \
                 [dartDocumentServiceMcpCatalog, {second}ServiceMcpCatalog],\n);\n\n\
                 void main() {{\n  print(workspace.names.length);\n}}"
            ),
        ]
        .join("\n\n")
    };
    let Some((distinct, said)) = dart("endpoint-distinct", &[], &endpoint("dartBilling")) else {
        return;
    };
    assert!(distinct, "{said}");
    assert_eq!(said.trim(), "10");
    let (shared, refused) = dart("endpoint-shared", &[], &endpoint("audit")).unwrap();
    assert!(!shared, "{refused}");
    assert!(
        refused.contains(
            "The element '\"tool:get-version\"' conflicts with another existing element in the set"
        ),
        "{refused}"
    );
}

#[test]
fn the_dart_mcp_server_and_client_have_no_issue() {
    analyzed("mcp-server-analyze", &module(SERVER_DRIVER));
    analyzed("mcp-client-analyze", &module(CLIENT_DRIVER));
    analyzed("mcp-http-analyze", &module(HTTP_DRIVER));
    analyzed("mcp-loopback-analyze", &module(LOOPBACK_DRIVER));
}

#[test]
fn the_archive_answers_every_operation_of_every_service() {
    let version = || McpVersionRequest {
        document_id: "doc-1".to_owned(),
        format: None,
    };
    let answered = poll_once(DartDocumentService::get_version(&Archive, &(), version())).unwrap();
    assert_eq!(answered.unwrap().version, 5);
    let lookup = poll_once(Archive.lookup(&(), McpDocumentId("doc-1".to_owned()))).unwrap();
    assert_eq!(lookup.unwrap_err(), McpVersionError::NotFound);
    let tagged = poll_once(Archive.get_tagged(&(), "doc-1".to_owned(), None)).unwrap();
    assert_eq!(tagged.unwrap_err(), McpVersionError::NotFound);
    let renamed =
        poll_once(Archive.rename_document(&(), "doc-1".to_owned(), "Plan".to_owned())).unwrap();
    assert_eq!(renamed, Ok(()));
    poll_once(Archive.touch_document(&(), "doc-1".to_owned(), "reviewer-7".to_owned())).unwrap();
    let audited = poll_once(AuditService::get_version(&Archive, &(), version())).unwrap();
    assert_eq!(audited.unwrap_err(), McpVersionError::NotFound);
    let invoices = poll_once(Archive.list_invoices(
        &(),
        McpInvoiceRequest {
            organization: "acme".to_owned(),
        },
    ))
    .unwrap();
    assert_eq!(invoices.unwrap().invoices, ["acme"]);
    let tagging = poll_once(Archive.retag(
        &(),
        "doc-1".to_owned(),
        "a".to_owned(),
        "r".to_owned(),
        None,
        "f".to_owned(),
    ))
    .unwrap();
    assert_eq!(tagging.unwrap(), "doc-1:a:r::f");
}

/// What the Rust dispatcher of `DartDocumentService` answers to `row`.
fn rust_answer(row: &AuthRow) -> (u16, Vec<(String, String)>, String) {
    rust_auth_answer!(document_mcp, DartDocumentServiceSchema, Archive, row, ready)
}

fn ready<Answering>(answering: Answering) -> Answering::Output
where
    Answering: Future,
{
    poll_once(answering).unwrap()
}

fn auth_report() -> Option<Value> {
    let input = json!({
        "origin": ORIGIN,
        "oauth": {
            "authorizationServers": [AUTHORIZATION_SERVER],
            "resource": RESOURCE,
            "resourceMetadataUrl": METADATA_URL,
            "scopesSupported": [SCOPES_SUPPORTED],
        },
        "rows": auth_rows().iter().map(|row| json!({
            "body": row.body,
            "headers": row.headers.iter().map(|(name, value)| json!([name, value])).collect::<Vec<_>>(),
            "method": row.method,
            "path": row.path,
            "scheme": row.scheme.word(),
        })).collect::<Vec<_>>(),
        "queries": SCOPE_QUERIES.iter().map(|(method, name)| json!([method, name])).collect::<Vec<_>>(),
    });
    let driver = format!("const inputText = r\'\'\'{input}\'\'\';\n{AUTH_DRIVER}");
    driven("mcp_auth", &driver)
}

#[test]
fn the_http_server_answers_every_authorization_case_as_the_rust_dispatcher_does() {
    let Some(report) = auth_report() else {
        return;
    };
    let rows = auth_rows();
    let answers = report["answers"].as_array().unwrap();
    assert_eq!(answers.len(), rows.len());
    for (row, dart) in rows.iter().zip(answers) {
        let (status, headers, body) = rust_answer(row);
        assert_eq!(
            dart["status"].as_u64().unwrap(),
            u64::from(status),
            "{}: {dart}",
            row.name
        );
        let dart_headers: Vec<(String, String)> = dart["headers"]
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
        assert_eq!(dart_headers, headers, "{}", row.name);
        assert_eq!(dart["body"].as_str().unwrap(), body, "{}", row.name);
        assert_organization_answer(row, status, &body);
        if status != 200 {
            assert_eq!(dart["reached"], 0_i32, "{}: no handler is called", row.name);
        }
    }
}

#[test]
fn the_scopes_function_answers_what_the_rust_registry_answers() {
    let Some(report) = auth_report() else {
        return;
    };
    let dart = report["scopes"].as_array().unwrap();
    assert_eq!(dart.len(), SCOPE_QUERIES.len());
    for ((method, name), held) in SCOPE_QUERIES.iter().zip(dart) {
        let rust = DartDocumentServiceSchema::mcp_scopes(method, *name);
        assert_eq!(*held, json!(rust), "{method} {name:?}");
    }
}

#[test]
fn the_rust_dispatcher_answers_alone_beside_a_second_service_and_over_http_alike() {
    let (alone, served, status) =
        rust_serves_alike!(document_mcp, DartDocumentServiceSchema, Archive, ready);
    assert_eq!(alone, served);
    assert!(alone.is_some());
    assert_eq!(status, 200);
}

#[test]
fn the_dart_host_writes_the_messages_the_rust_host_writes() {
    let driver = format!(
        "const inputText = r\'\'\'{}\'\'\';\n{WRITER_DRIVER}",
        writer_input()
    );
    let Some(report) = driven("mcp_writer", &driver) else {
        return;
    };
    assert_matches(&report);
}

#[test]
fn the_rust_dispatcher_listens_alike_through_every_entry_point() {
    let (alone, served) =
        rust_listens_alike!(document_mcp, DartDocumentServiceSchema, Archive, ready);
    assert_eq!(alone, served);
    assert!(alone.is_some());
}
