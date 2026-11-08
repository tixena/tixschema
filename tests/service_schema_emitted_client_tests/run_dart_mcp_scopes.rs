//! The emitted Dart `mcp` server run by the Dart VM over a service whose resources, template,
//! listing and completion declare scopes: its scopes function and its HTTP server under OAuth
//! answer what the Rust dispatcher answers.

#![cfg(feature = "dart")]

use super::StacksServiceSchema;
use super::mcp_library_service::{book_error_dart, book_info_dart, book_request_dart};
use super::mcp_stacks_service::{assert_matches_rust, input};
use super::runtime::ran;
use crate::mcp_streams::dart_respond;
use serde_json::Value;

const RUNTIME_VAR: &str = "TIXSCHEMA_DART";

const DRIVER: &str = "
void main() async {
  final input = jsonDecode(INPUT) as Map<String, dynamic>;
  var reached = 0;
  T found<T>(T value) {
    reached += 1;
    return value;
  }
  final handlers = StacksServiceHandlers<void>(
    stacksIndex: (ctx, req) async => found('# The stacks'),
    pinnedItem: (ctx, req) async => found('pinned'),
    getItem: (ctx, itemId) async => found(itemId),
    openNotice: (ctx, req) async => found('open'),
    listItems: (ctx, cursor) async => found(McpResourcePage(resources: [McpResource(uri: 'stacks://items/item-1', name: 'item-1')])),
    suggestItems: (ctx, req) async => found(['item-1', 'item-2'].where((name) => name.startsWith(req.typed)).toList()),
    countItems: (ctx, req) async => found(BookInfo(pages: 3)),
  );
  const tokens = <String, String>{'read-token': 'documents:read', 'write-token': 'documents:read documents:write'};
  Future<({void ctx, StacksServiceMcpDenied? denied})> admit(StacksServiceMcpHttpRequest request, StacksServiceMcpCall call) async {
    final needed = stacksServiceMcpScopes(request.header('mcp-method') ?? '', request.mcpName);
    final token = request.bearerToken;
    if (token == null) {
      return (ctx: null, denied: StacksServiceMcpNoValidCredentials(scope: needed.isEmpty ? null : needed.join(' ')));
    }
    final held = tokens[token];
    if (held == null) return (ctx: null, denied: const StacksServiceMcpNoValidCredentials());
    if (needed.any((scope) => !held.split(' ').contains(scope))) {
      return (ctx: null, denied: StacksServiceMcpInsufficientScope(scope: needed.join(' ')));
    }
    return (ctx: null, denied: null);
  }

  final oauth = input['oauth'] as Map<String, dynamic>;
  final serve = createStacksServiceMcpHttpDispatcher<void>(
    handlers,
    StacksServiceMcpHttpSettings(
      allowedOrigins: [input['origin'] as String],
      oauth: StacksServiceMcpOAuth(
        authorizationServers: (oauth['authorizationServers'] as List).cast<String>(),
        resource: oauth['resource'] as String,
        resourceMetadataUrl: oauth['resourceMetadataUrl'] as String,
        scopesSupported: (oauth['scopesSupported'] as List).cast<String>(),
      ),
    ),
    admit,
  );
  final answers = <Map<String, dynamic>>[];
  for (final row in input['rows'] as List) {
    final given = row as Map<String, dynamic>;
    final before = reached;
    final response = await respondStacksService(serve, StacksServiceMcpHttpRequest(
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
      stacksServiceMcpScopes((query as List)[0] as String, query[1] as String?),
  ];
  print(jsonEncode({'answers': answers, 'scopes': scopes}));
}
";

fn module() -> String {
    let literal = Value::String(input().to_string());
    let dollar_safe = literal.to_string().replace('$', "\\$");
    [
        "import 'dart:async';".to_owned(),
        "import 'dart:convert';".to_owned(),
        "import 'dart:typed_data';".to_owned(),
        book_request_dart::dart_definition(),
        book_info_dart::dart_definition(),
        book_error_dart::dart_definition(),
        StacksServiceSchema::dart_definition(),
        StacksServiceSchema::dart_ws_client(),
        StacksServiceSchema::dart_mcp_service(),
        dart_respond("StacksService"),
        format!("const INPUT = {dollar_safe};"),
        DRIVER.to_owned(),
    ]
    .join("\n\n")
}

#[test]
fn the_dart_server_answers_the_scopes_of_a_resource_as_the_rust_dispatcher_does() {
    let Some(wrote) = ran("dart", RUNTIME_VAR, "dart", "mcp-scopes.dart", &module()) else {
        return;
    };
    let report: Value = serde_json::from_str(wrote.trim()).unwrap();
    assert_matches_rust("dart", &report);
}
