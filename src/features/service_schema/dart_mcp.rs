//! The Dart `mcp` server and client: a catalog and a route a service puts on an endpoint, the
//! function that answers one JSON-RPC message for the services of that endpoint, the HTTP rules
//! around it, and a client that calls each operation as a tool.
//!
//! Neither text names a tool list or whether a message sits under `value`: the accessors in
//! [`seam`] write those over the markers below when they run in the consuming program.

use super::dart_http_client::{dart_call, dart_parameter};
use super::dart_ws_client::{
    dart_type_of, declared_error_decode_stmts, has_one_way, header_call_args, header_out_push_stmt,
    message_dart_typename, message_decode, message_encode, split_reply_record,
    success_decode_block,
};
use super::result::result_name;
use crate::features::dart::{dart_json_decode, quoted};
use crate::rename_rule::RenameRule;
use crate::service_schema::parse::{
    HttpShape, OperationDef, OperationInputs, OperationOutcome, ServiceDef, is_unit_type, written,
};
use crate::service_schema::support::fault_fields_typescript_name;
use core::fmt::Write as _;
use proc_macro2::TokenStream;
use quote::{format_ident, quote};

const TOOLS_MARKER: &str = "@TOOLS@";
const NAMES_MARKER: &str = "@NAMES@";
const INSTRUCTIONS_MARKER: &str = "@INSTRUCTIONS@";
const VERSION_MARKER: &str = "@VERSION@";

const SERVER_HEAD: &str = r#"/// The name `server/discover` reports this service under.
const @P@McpServerName = @NAMEQ@;

/// The version of the crate that declares this service, reported as `serverInfo.version`.
const @P@McpServerVersion = '@VERSION@';

/// The trait's rustdoc, answered as `instructions` by `server/discover`.
const String? @P@McpInstructions = @INSTRUCTIONS@;

/// Every name this service puts on an endpoint, as `kind:name`.
const @P@McpNames = <String>[@NAMES@];

/// Everything the list methods answer for this service, each item as the JSON text Rust wrote.
typedef @N@McpCatalog = ({
  List<String> tools,
  List<String> resources,
  List<String> resourceTemplates,
  List<String> prompts,
  List<String> skills,
  List<String> capabilities,
});

const @N@McpCatalog @P@McpCatalog = (
  tools: <String>[@TOOLS@],
  resources: <String>[],
  resourceTemplates: <String>[],
  prompts: <String>[],
  skills: <String>[],
  capabilities: <String>['{"tools":{}}'],
);

/// What a route answers: the `result` object of a message, or its JSON-RPC `error` object.
typedef @N@McpAnswer = ({Object? result, Object? error});

/// The services of one endpoint. A `const` endpoint whose services share a name does not compile.
class @N@McpEndpoint {
  const @N@McpEndpoint(this.names, this.catalogs);
  final Set<String> names;
  final List<@N@McpCatalog> catalogs;
}

/// An HTTP request in plain terms. `path` is the request's path, with or without its query.
class @N@McpHttpRequest {
  const @N@McpHttpRequest({required this.method, required this.path, required this.headers, required this.body});
  final String method;
  final String path;
  final List<(String, String)> headers;
  final List<int> body;

  /// The value of the header called `name`, whatever its case.
  String? header(String name) {
    final wanted = name.toLowerCase();
    for (final (key, value) in headers) {
      if (key.toLowerCase() == wanted) return value;
    }
    return null;
  }

  /// The token of an `Authorization: Bearer` header, whatever the case of the scheme.
  String? get bearerToken {
    final value = header('authorization');
    if (value == null) return null;
    final at = value.indexOf(' ');
    if (at < 0) return null;
    final token = value.substring(at + 1).trim();
    return value.substring(0, at).toLowerCase() == 'bearer' && token.isNotEmpty ? token : null;
  }

  /// The `Mcp-Name` header, decoded from `=?base64?...?=` where it is written so.
  String? get mcpName {
    final value = header('mcp-name');
    if (value == null || !value.startsWith('=?base64?') || !value.endsWith('?=')) return value;
    try {
      return utf8.decode(base64.decode(value.substring(9, value.length - 2)));
    } catch (_) {
      return null;
    }
  }
}

/// An HTTP response in plain terms.
class @N@McpHttpResponse {
  const @N@McpHttpResponse({required this.status, required this.headers, required this.body});
  final int status;
  final List<(String, String)> headers;
  final List<int> body;
}

/// The protected-resource metadata of a server that authorizes by OAuth.
class @N@McpOAuth {
  const @N@McpOAuth({
    required this.authorizationServers,
    required this.resource,
    required this.resourceMetadataUrl,
    required this.scopesSupported,
  });
  final List<String> authorizationServers;
  final String resource;
  final String resourceMetadataUrl;
  final List<String> scopesSupported;
}

/// What an HTTP server of this endpoint is configured with. Without `oauth` it serves no metadata
/// document and writes no challenge.
class @N@McpHttpSettings {
  const @N@McpHttpSettings({required this.allowedOrigins, this.oauth});
  final List<String> allowedOrigins;
  final @N@McpOAuth? oauth;
}

/// Why `admit` refused a request.
sealed class @N@McpDenied {
  const @N@McpDenied();
}

/// `401`: no credentials, or credentials that do not hold. `scope` names what the called operation needs.
class @N@McpNoValidCredentials extends @N@McpDenied {
  const @N@McpNoValidCredentials({this.scope});
  final String? scope;
}

/// `403` with a challenge: the credentials hold, but not the scopes `scope` names.
class @N@McpInsufficientScope extends @N@McpDenied {
  const @N@McpInsufficientScope({required this.scope});
  final String scope;
}

/// `403` with no challenge.
class @N@McpForbidden extends @N@McpDenied {
  const @N@McpForbidden();
}

/// Where the transport writes a message beyond the answer: one line on stdio, one event on HTTP. A
/// write that throws means the client has gone.
typedef @N@McpWriter = Future<void> Function(String message);

/// The request in flight, handed to `admit` and to the function that builds `Ctx`, so that a
/// handler reaches it. `token` is the request's `progressToken`, and `writer` takes what it writes
/// beyond its answer.
class @N@McpCall {
  @N@McpCall({Object? token, @N@McpWriter? writer})
      : _token = token,
        _writer = writer;
  final Object? _token;
  final @N@McpWriter? _writer;
  bool _gone = false;

  /// Writes `notifications/progress` where the request carries a `progressToken`, and does nothing
  /// otherwise. A write that fails cancels the call.
  Future<void> progress(num done, {num? total, String? message}) async {
    final writer = _writer;
    final token = _token;
    if (writer == null || token == null || _gone) return;
    try {
      await writer(_@P@McpJson(<String, dynamic>{
        'jsonrpc': '2.0',
        'method': 'notifications/progress',
        'params': <String, dynamic>{
          'progress': done,
          'progressToken': token,
          if (total != null) 'total': total,
          if (message != null) 'message': message,
        },
      }));
    } catch (_) {
      _gone = true;
    }
  }

  /// Whether the client cancelled this request, or went away.
  bool get cancelled => _gone;
}

/// The requests a stdio host has in flight, so that `notifications/cancelled` reaches the right one.
class @N@McpInFlight {
  final Map<String, @N@McpCall> _calls = <String, @N@McpCall>{};

  void _cancel(Object? id) {
    _calls[jsonEncode(id)]?._gone = true;
  }

  void Function() _enter(Object? id, @N@McpCall call) {
    final key = jsonEncode(id);
    _calls[key] = call;
    return () {
      if (identical(_calls[key], call)) _calls.remove(key);
    };
  }
}

/// What `@P@McpServe` writes to, and the requests it has in flight.
class @N@McpHost {
  const @N@McpHost({required this.writer, required this.inFlight});
  final @N@McpWriter writer;
  final @N@McpInFlight inFlight;
}

/// What `admit` answers: the context the handlers run with in `ctx`, or why the request is refused in `denied`.
typedef @N@McpAdmit<Ctx> = Future<({Ctx? ctx, @N@McpDenied? denied})> Function(
  @N@McpHttpRequest request,
  @N@McpCall call,
);

/// The scopes a caller needs for the request `method` and `name` name, none for any other.
List<String> @P@McpScopes(String method, String? name) {
@SCOPES@
  return const <String>[];
}

Object? _@P@McpSorted(Object? value) {
  if (value is Map) {
    final keys = value.keys.map((key) => '$key').toList()..sort();
    return <String, Object?>{for (final key in keys) key: _@P@McpSorted(value[key])};
  }
  if (value is List) return value.map(_@P@McpSorted).toList();
  return value;
}

String _@P@McpJson(Object? value) => jsonEncode(_@P@McpSorted(jsonDecode(jsonEncode(value))));

String _@P@McpObject(Map<String, String> members) {
  final keys = members.keys.toList()..sort();
  return '{${[for (final key in keys) '${jsonEncode(key)}:${members[key]}'].join(',')}}';
}

void _@P@McpMerge(Map<String, dynamic> into, Map<String, dynamic> from) {
  from.forEach((key, value) {
    final held = into[key];
    if (held is Map<String, dynamic> && value is Map<String, dynamic>) {
      _@P@McpMerge(held, value);
    } else {
      into[key] = value;
    }
  });
}

Map<String, dynamic> _@P@McpUnknownOperation(String operation) => <String, dynamic>{
      'fault': <String, dynamic>{
        'detail': 'the service answers to no operation by that name',
        'kind': 'unknown-operation',
        'operation': operation,
      },
      'isServiceFault': true,
    };

Future<({String? text, int status})> _@P@McpRespond(
  String text,
  String serverName,
  String serverVersion,
  String? instructions,
  @N@McpEndpoint endpoint,
  Future<@N@McpAnswer?> Function(int part, String method, Map<String, dynamic> params) route,
) async {
  ({String? text, int status}) failed(Object? id, int status, int code, String message,
          [Object? data]) =>
      (
        text: _@P@McpJson(<String, dynamic>{
          'error': <String, dynamic>{'code': code, 'message': message, if (data != null) 'data': data},
          if (id != null) 'id': id,
          'jsonrpc': '2.0',
        }),
        status: status,
      );
  final Object? parsed;
  try {
    parsed = jsonDecode(text);
  } catch (_) {
    return failed(null, 400, -32600, 'Invalid Request');
  }
  if (parsed is! Map<String, dynamic>) return failed(null, 400, -32600, 'Invalid Request');
  if (!parsed.containsKey('id')) return (text: null, status: 202);
  final id = parsed['id'];
  final method = parsed['method'];
  if (method is! String) return failed(id, 400, -32600, 'Invalid Request');
  final given = parsed['params'];
  final params = given is Map<String, dynamic> ? given : <String, dynamic>{};
  final meta = params['_meta'];
  final version = meta is Map ? meta['io.modelcontextprotocol/protocolVersion'] : null;
  final held = meta is Map ? meta['io.modelcontextprotocol/clientCapabilities'] : null;
  if (version == null || held == null) {
    return failed(id, 400, -32602,
        'Missing _meta.io.modelcontextprotocol/protocolVersion or clientCapabilities');
  }
  if (version != '2026-07-28') {
    return failed(id, 400, -32022, 'Unsupported protocol version',
        <String, dynamic>{'requested': version, 'supported': <String>['2026-07-28']});
  }
  final info = _@P@McpObject(<String, String>{
    'io.modelcontextprotocol/serverInfo':
        _@P@McpJson(<String, dynamic>{'name': serverName, 'version': serverVersion}),
  });
  ({String? text, int status}) answered(String result) =>
      (text: '{"id":${_@P@McpJson(id)},"jsonrpc":"2.0","result":$result}', status: 200);

  if (method == 'server/discover') {
    final capabilities = <String, dynamic>{};
    for (final catalog in endpoint.catalogs) {
      for (final written in catalog.capabilities) {
        _@P@McpMerge(capabilities, jsonDecode(written) as Map<String, dynamic>);
      }
    }
    return answered(_@P@McpObject(<String, String>{
      '_meta': info,
      'cacheScope': '"public"',
      'capabilities': _@P@McpJson(capabilities),
      if (instructions != null) 'instructions': _@P@McpJson(instructions),
      'resultType': '"complete"',
      'supportedVersions': '["2026-07-28"]',
      'ttlMs': '300000',
    }));
  }

  final catalogs = endpoint.catalogs;
  final String? field;
  final List<String> listed;
  var scope = 'public';
  var ttl = 300000;
  switch (method) {
    case 'tools/list':
      field = 'tools';
      listed = [for (final catalog in catalogs) ...catalog.tools];
    case 'resources/templates/list':
      field = 'resourceTemplates';
      listed = [for (final catalog in catalogs) ...catalog.resourceTemplates];
    case 'prompts/list':
      field = 'prompts';
      listed = [for (final catalog in catalogs) ...catalog.prompts];
    case 'skills/list':
      field = 'skills';
      listed = [for (final catalog in catalogs) ...catalog.skills];
      scope = 'private';
      ttl = 0;
    default:
      field = null;
      listed = const <String>[];
  }
  if (field != null) {
    return answered(_@P@McpObject(<String, String>{
      '_meta': info,
      'cacheScope': '"$scope"',
      field: '[${listed.join(',')}]',
      'resultType': '"complete"',
      'ttlMs': '$ttl',
    }));
  }

  if (method == 'resources/list') {
    final cursor = params['cursor'];
    var place = 0;
    String? inner;
    if (cursor != null) {
      if (cursor is! String) return failed(id, 200, -32602, 'Invalid cursor');
      final colon = cursor.indexOf(':');
      final at = colon < 0 ? null : int.tryParse(cursor.substring(0, colon));
      if (at == null || at < 0 || at >= catalogs.length) {
        return failed(id, 200, -32602, 'Invalid cursor');
      }
      place = at;
      final rest = cursor.substring(colon + 1);
      inner = rest.isEmpty ? null : rest;
    }
    final items = <String>[
      if (cursor == null) for (final catalog in catalogs) ...catalog.resources,
    ];
    String? next;
    if (catalogs.isNotEmpty) {
      final page = await route(place, 'resources/list', <String, dynamic>{if (inner != null) 'cursor': inner});
      if (page != null) {
        if (page.error != null) {
          return (
            text: _@P@McpJson(<String, dynamic>{'error': page.error, 'id': id, 'jsonrpc': '2.0'}),
            status: 200,
          );
        }
        final result = page.result;
        if (result is Map) {
          final resources = result['resources'];
          if (resources is List) items.addAll(resources.map(_@P@McpJson));
          final following = result['nextCursor'];
          if (following is String) next = '$place:$following';
        }
      }
      if (next == null && place + 1 < catalogs.length) next = '${place + 1}:';
    }
    return answered(_@P@McpObject(<String, String>{
      '_meta': info,
      'cacheScope': '"private"',
      if (next != null) 'nextCursor': _@P@McpJson(next),
      'resultType': '"complete"',
      'resources': '[${items.join(',')}]',
      'ttlMs': '0',
    }));
  }

  const routed = <String>{
    'tools/call',
    'resources/read',
    'prompts/get',
    'completion/complete',
    'tasks/get',
    'tasks/cancel',
    'tasks/update',
    'skills/get',
  };
  if (!routed.contains(method)) return failed(id, 404, -32601, 'Method not found: $method');
  for (var part = 0; part < catalogs.length; part += 1) {
    final reply = await route(part, method, params);
    if (reply == null) continue;
    if (reply.error != null) {
      return (
        text: _@P@McpJson(<String, dynamic>{'error': reply.error, 'id': id, 'jsonrpc': '2.0'}),
        status: 200,
      );
    }
    final result = reply.result;
    final members = result is Map<String, dynamic> ? Map<String, dynamic>.of(result) : <String, dynamic>{};
    members.putIfAbsent('resultType', () => 'complete');
    members.putIfAbsent('_meta', () => jsonDecode(info));
    return answered(_@P@McpJson(members));
  }
  final asked = params['name'] ?? params['uri'] ?? method;
  return failed(
    id,
    200,
    -32602,
    method == 'tools/call' ? 'Unknown tool: $asked' : '$method: nothing on this endpoint answers it',
    _@P@McpUnknownOperation('$asked'),
  );
}

Map<String, dynamic>? _@P@McpMessage(String text) {
  try {
    final parsed = jsonDecode(text);
    return parsed is Map<String, dynamic> ? parsed : null;
  } catch (_) {
    return null;
  }
}

Object? _@P@McpToken(Map<String, dynamic>? message) {
  final params = message?['params'];
  final meta = params is Map ? params['_meta'] : null;
  final token = meta is Map ? meta['progressToken'] : null;
  return token is String || token is num ? token : null;
}

/// One JSON-RPC message in, one out, for the services of `endpoint`. `route` answers for the
/// service at that place in the endpoint, or `null` for a request it does not own, and receives
/// the `Ctx` that `ctx` built from the request in flight. A notification answers `null`, and
/// `notifications/cancelled` marks the request it names in `host.inFlight`. The answer of a
/// cancelled request is dropped.
Future<String?> @P@McpServe<Ctx>(
  String text,
  String serverName,
  String? instructions,
  @N@McpEndpoint endpoint,
  Ctx Function(@N@McpCall call) ctx,
  @N@McpHost host,
  Future<@N@McpAnswer?> Function(Ctx ctx, int part, String method, Map<String, dynamic> params) route,
) async {
  final message = _@P@McpMessage(text);
  if (message != null && !message.containsKey('id')) {
    if (message['method'] == 'notifications/cancelled') {
      final params = message['params'];
      host.inFlight._cancel(params is Map ? params['requestId'] : null);
    }
    return null;
  }
  final call = @N@McpCall(token: _@P@McpToken(message), writer: host.writer);
  void Function()? leave;
  Ctx? built;
  var building = true;
  try {
    final answered = await _@P@McpRespond(text, serverName, @P@McpServerVersion, instructions, endpoint,
        (part, method, params) {
      if (building) {
        building = false;
        leave = host.inFlight._enter(message?['id'], call);
        built = ctx(call);
      }
      return route(built as Ctx, part, method, params);
    });
    return call.cancelled ? null : answered.text;
  } finally {
    leave?.call();
  }
}

Object? _@P@McpBodyToken(@N@McpHttpRequest request) {
  try {
    return _@P@McpToken(_@P@McpMessage(utf8.decode(request.body)));
  } catch (_) {
    return null;
  }
}

/// The `200` event-stream answer: each event written, then the answer.
@N@McpHttpResponse _@P@McpEventStream(List<String> events, String? answered) => @N@McpHttpResponse(
      status: 200,
      headers: const [('Content-Type', 'text/event-stream'), ('X-Accel-Buffering', 'no')],
      body: utf8.encode([
        for (final event in [...events, if (answered != null) answered]) 'event: message\ndata: $event\n\n',
      ].join()),
    );

String _@P@McpUrlPath(String url) {
  final separator = url.indexOf('://');
  final after = separator < 0 ? url : url.substring(separator + 3);
  final at = after.indexOf('/');
  final fromPath = at < 0 ? '/' : after.substring(at);
  return fromPath.split(RegExp('[?#]')).first;
}

@N@McpHttpResponse _@P@McpReplied(int status, {String? body, List<(String, String)> headers = const <(String, String)>[]}) =>
    @N@McpHttpResponse(
      status: status,
      headers: body == null ? headers : const [('Content-Type', 'application/json')],
      body: body == null ? const <int>[] : utf8.encode(body),
    );

@N@McpHttpResponse _@P@McpDenial(@N@McpDenied denied, @N@McpHttpSettings settings, @N@McpHttpRequest request) {
  final oauth = settings.oauth;
  String? challenge;
  var status = 403;
  if (denied is @N@McpNoValidCredentials) {
    status = 401;
    if (oauth != null) {
      final scope = denied.scope;
      final parts = <String>[
        'resource_metadata="${oauth.resourceMetadataUrl}"',
        if (scope != null) 'scope="$scope"',
        if (request.bearerToken != null) 'error="invalid_token"',
      ];
      challenge = 'Bearer ${parts.join(', ')}';
    }
  } else if (denied is @N@McpInsufficientScope && oauth != null) {
    challenge =
        'Bearer error="insufficient_scope", scope="${denied.scope}", resource_metadata="${oauth.resourceMetadataUrl}"';
  }
  return _@P@McpReplied(status, headers: challenge == null ? const [] : [('WWW-Authenticate', challenge)]);
}

@N@McpHttpResponse _@P@McpMetadata(@N@McpOAuth oauth) => _@P@McpReplied(200, body: _@P@McpJson(<String, dynamic>{
      'authorization_servers': oauth.authorizationServers,
      'bearer_methods_supported': <String>['header'],
      'resource': oauth.resource,
      if (oauth.scopesSupported.isNotEmpty) 'scopes_supported': oauth.scopesSupported,
    }));

/// The Streamable HTTP rules around `@P@McpServe`: the metadata document, the method, the `Origin`,
/// `admit`, the body and the headers that must agree with it. `route` also receives the `Ctx` that
/// `admit` answered.
Future<@N@McpHttpResponse> @P@McpServeHttp<Ctx>(
  @N@McpHttpRequest request,
  @N@McpHttpSettings settings,
  String serverName,
  String? instructions,
  @N@McpEndpoint endpoint,
  @N@McpAdmit<Ctx> admit,
  Future<@N@McpAnswer?> Function(Ctx ctx, int part, String method, Map<String, dynamic> params) route,
) async {
  @N@McpHttpResponse respond(int status, [String? body]) => _@P@McpReplied(status, body: body);
  String? header(String name) => request.header(name);

  String error(Object? id, int code, String message) => _@P@McpJson(<String, dynamic>{
        'error': <String, dynamic>{'code': code, 'message': message},
        if (id != null) 'id': id,
        'jsonrpc': '2.0',
      });
  final oauth = settings.oauth;
  if (oauth != null &&
      request.method == 'GET' &&
      request.path.split(RegExp('[?#]')).first == _@P@McpUrlPath(oauth.resourceMetadataUrl)) {
    return _@P@McpMetadata(oauth);
  }
  if (request.method.toUpperCase() != 'POST') {
    return _@P@McpReplied(405, headers: const [('Allow', 'POST')]);
  }
  final origin = header('origin');
  if (origin != null && !settings.allowedOrigins.contains(origin)) return respond(403);
  final events = <String>[];
  final call = @N@McpCall(token: _@P@McpBodyToken(request), writer: (event) async => events.add(event));
  final admitted = await admit(request, call);
  final denied = admitted.denied;
  if (denied != null) return _@P@McpDenial(denied, settings, request);
  final ctx = admitted.ctx as Ctx;
  final Object? parsed;
  try {
    parsed = jsonDecode(utf8.decode(request.body));
  } catch (_) {
    return respond(400, error(null, -32600, 'Invalid Request'));
  }
  if (parsed is! Map<String, dynamic>) return respond(400, error(null, -32600, 'Invalid Request'));
  if (!parsed.containsKey('id')) return respond(202);
  final id = parsed['id'];
  final method = parsed['method'];
  if (method is! String) return respond(400, error(id, -32600, 'Invalid Request'));
  final given = parsed['params'];
  final params = given is Map<String, dynamic> ? given : <String, dynamic>{};
  final meta = params['_meta'];
  final version = meta is Map ? meta['io.modelcontextprotocol/protocolVersion'] : null;
  final versionHeader = header('mcp-protocol-version');
  if (versionHeader == null || (version != null && versionHeader != version)) {
    return respond(400, error(id, -32020, 'Header mismatch: MCP-Protocol-Version'));
  }
  if (header('mcp-method') != method) {
    return respond(400, error(id, -32020, 'Header mismatch: Mcp-Method'));
  }
  final named = method == 'tools/call' || method == 'prompts/get'
      ? params['name']
      : method == 'resources/read'
          ? params['uri']
          : null;
  if (method == 'tools/call' || method == 'prompts/get' || method == 'resources/read') {
    final asked = request.mcpName;
    if (asked == null || asked != named) {
      return respond(400, error(id, -32020, 'Header mismatch: Mcp-Name'));
    }
  }
  var routed = false;
  final answered = await _@P@McpRespond(utf8.decode(request.body), serverName, @P@McpServerVersion, instructions,
      endpoint, (part, method, params) {
    routed = true;
    return route(ctx, part, method, params);
  });
  if (!routed) return respond(answered.status, answered.text);
  final written = call.cancelled ? null : answered.text;
  if (_@P@McpToken(parsed) != null) return _@P@McpEventStream(events, written);
  return written == null ? respond(202) : respond(answered.status, written);
}
"#;

const CLIENT_HEAD: &str = "/// What binds a `@N@` Dart client to a real MCP transport. `send` is handed the JSON-RPC text and
/// the values of the `MCP-Protocol-Version`, `Mcp-Method` and `Mcp-Name` headers, and answers the
/// text of the reply.
abstract class @N@McpTransport {
  Future<String> send(({String text, String protocolVersion, String method, String? name}) message);
}";

const CLIENT_CALL: &str = "class _@N@McpCallFault implements Exception {
  _@N@McpCallFault(this.fault);
  final @F@ fault;
}

@F@ _@P@McpClientFault(@N@FaultKind kind, String operation, String detail, {String? field}) =>
    @F@(detail: detail, field: field, kind: kind, operation: operation);

@F@ _@P@McpReadFailed(String operation, String detail, {String? field}) =>
    _@P@McpClientFault(@N@FaultKind.undeserializablePayload, operation, detail, field: field);";

const CLIENT_CALL_METHOD: &str = "  int _nextId = 1;

  Future<Map<String, dynamic>> _call(
    String tool,
    String operation,
    Map<String, dynamic> arguments,
  ) async {
    final text = jsonEncode(<String, dynamic>{
      'id': _nextId++,
      'jsonrpc': '2.0',
      'method': 'tools/call',
      'params': <String, dynamic>{
        '_meta': <String, dynamic>{
          'io.modelcontextprotocol/clientCapabilities': <String, dynamic>{},
          'io.modelcontextprotocol/clientInfo': <String, dynamic>{
            'name': '@N@McpClient',
            'version': '@VERSION@',
          },
          'io.modelcontextprotocol/protocolVersion': '2026-07-28',
        },
        'arguments': arguments,
        'name': tool,
      },
    });
    final String answer;
    try {
      answer = await _transport.send(
        (text: text, protocolVersion: '2026-07-28', method: 'tools/call', name: tool),
      );
    } catch (uncarried) {
      throw _@N@McpCallFault(
        _@P@McpClientFault(@N@FaultKind.transportFailure, operation, '$uncarried'),
      );
    }
    final Object? parsed;
    try {
      parsed = jsonDecode(answer);
    } catch (rejected) {
      throw _@N@McpCallFault(_@P@McpReadFailed(operation, '$rejected'));
    }
    if (parsed is! Map<String, dynamic>) {
      throw _@N@McpCallFault(_@P@McpReadFailed(operation, 'the answer is not a JSON-RPC message'));
    }
    final error = parsed['error'];
    if (error is Map<String, dynamic>) {
      final data = error['data'];
      if (data is Map<String, dynamic> && data['isServiceFault'] == true) {
        final @F@ fault;
        try {
          fault = @F@.fromJson(data['fault'] as Map<String, dynamic>);
        } catch (rejected) {
          throw _@N@McpCallFault(_@P@McpReadFailed(operation, '$rejected'));
        }
        throw _@N@McpCallFault(fault);
      }
      throw _@N@McpCallFault(
        _@P@McpClientFault(
          @N@FaultKind.transportFailure,
          operation,
          'the server answered ${error['code']}: ${error['message']}',
        ),
      );
    }
    final result = parsed['result'];
    if (result is! Map<String, dynamic>) {
      throw _@N@McpCallFault(_@P@McpReadFailed(operation, 'the answer holds no result'));
    }
    return result;
  }";

const CLIENT_FAULT_OF: &str = "@F@ _@P@McpFaultOf(Map<String, dynamic> result, String operation) {
  final Object? parsed;
  try {
    parsed = jsonDecode(((result['content'] as List).first as Map<String, dynamic>)['text'] as String);
  } catch (rejected) {
    return _@P@McpReadFailed(operation, '$rejected');
  }
  if (parsed is Map<String, dynamic> && parsed['isServiceFault'] == true) {
    try {
      return @F@.fromJson(parsed['fault'] as Map<String, dynamic>);
    } catch (rejected) {
      return _@P@McpReadFailed(operation, '$rejected');
    }
  }
  return _@P@McpReadFailed(operation, 'a one-way operation declares no error');
}";

const SERVER_ALONE: &str = "const _@P@McpAlone = @N@McpEndpoint({...@P@McpNames}, [@P@McpCatalog]);

/// Answers the requests of this service alone, one text in and one out. Usable as is for stdio.
/// `ctx` builds the handlers' context from the request in flight, `writer` takes what the request
/// writes beyond its answer, and `inFlight` lists the requests `notifications/cancelled` can reach.
Future<String?> Function(
  String text,
  Ctx Function(@N@McpCall call) ctx,
  @N@McpWriter writer,
  @N@McpInFlight inFlight,
) create@N@McpDispatcher<Ctx>(
  @N@Handlers<Ctx> handlers,
) {
  final route = create@N@McpRoute(handlers);
  return (text, ctx, writer, inFlight) => @P@McpServe<Ctx>(
        text,
        @P@McpServerName,
        @P@McpInstructions,
        _@P@McpAlone,
        ctx,
        @N@McpHost(writer: writer, inFlight: inFlight),
        (built, part, method, params) => route(built, method, params),
      );
}

/// Answers the HTTP requests of this service alone. `admit` answers the `Ctx` of a request, or why
/// it is refused.
Future<@N@McpHttpResponse> Function(@N@McpHttpRequest) create@N@McpHttpDispatcher<Ctx>(
  @N@Handlers<Ctx> handlers,
  @N@McpHttpSettings settings,
  @N@McpAdmit<Ctx> admit,
) {
  final route = create@N@McpRoute(handlers);
  return (request) => @P@McpServeHttp<Ctx>(
        request,
        settings,
        @P@McpServerName,
        @P@McpInstructions,
        _@P@McpAlone,
        admit,
        (ctx, part, method, params) => route(ctx, method, params),
      );
}";

/// What of the answer helpers the operations of a service call, so none is left unused.
struct Calls {
    declared: bool,
    empty: bool,
    ok: bool,
}

impl Calls {
    fn of(service: &ServiceDef) -> Self {
        let mut calls = Self {
            declared: false,
            empty: false,
            ok: false,
        };
        for operation in &service.operations {
            match &operation.outcome {
                OperationOutcome::OneWay => calls.empty = true,
                OperationOutcome::Reply {
                    success,
                    error: _error,
                } => {
                    calls.declared = true;
                    if is_unit_type(success) {
                        calls.empty = true;
                    } else {
                        calls.ok = true;
                    }
                }
            }
        }
        calls
    }
}

/// What the methods of both kinds share: the doc, the name, the parameters, the arguments and
/// the names the call is made under.
struct MethodHead {
    arguments: String,
    call: String,
    doc: String,
    parameters: String,
    tool: String,
    wire: String,
}

/// Whether the message sits under `value`, where it does not depend on the type the operation
/// names: `None` where `wrapped_{operation}()` answers it when the accessor runs.
const fn wrapped_at_expansion(operation: &OperationDef, shape: &HttpShape) -> Option<bool> {
    match &operation.inputs {
        OperationInputs::Named(_) if shape.header_in.is_empty() => None,
        OperationInputs::Named(_) => Some(true),
        OperationInputs::Empty | OperationInputs::Generated(_) => Some(false),
    }
}

fn wrapped_marker(operation: &OperationDef) -> String {
    format!("@WRAPPED:{}@", operation.mcp_tool_name())
}

fn wrapped_table(prefix: &str, side: &str, service: &ServiceDef) -> String {
    let entries: Vec<String> = service
        .operations
        .iter()
        .filter(|operation| wrapped_at_expansion(operation, &HttpShape::of(operation)).is_none())
        .map(|operation| {
            format!(
                "  {}: {},",
                quoted(operation.mcp_tool_name()),
                wrapped_marker(operation)
            )
        })
        .collect();
    if entries.is_empty() {
        return String::new();
    }
    format!(
        "const _{prefix}Mcp{side}Wrapped = <String, bool>{{\n{}\n}};\n\n",
        entries.join("\n")
    )
}

fn filled(template: &str, named: &str, prefix: &str) -> String {
    template
        .replace("@P@", prefix)
        .replace("@N@", named)
        .replace("@NAMEQ@", &quoted(named))
}

/// The server text: the catalog, the route, the function that serves an endpoint, and its
/// dispatchers.
pub fn service_text(declared: &ServiceDef) -> String {
    let service = &declared.enveloped();
    let named = service.ident.to_string();
    let prefix = RenameRule::CamelCase.apply_to_variant(&named);
    let calls = Calls::of(service);
    let mut published = vec![
        filled(SERVER_HEAD, &named, &prefix).replace("@SCOPES@\n", &scopes_table(service)),
        wrapped_table(&prefix, "Server", service)
            .trim_end()
            .to_owned(),
    ];
    if !service.operations.is_empty() {
        published.push(answer_helpers(&named, &prefix, &calls));
    }
    published.extend(
        service
            .operations
            .iter()
            .map(|operation| server_call(&named, &prefix, operation)),
    );
    published.push(route_fn(service, &named, &prefix));
    published.push(filled(SERVER_ALONE, &named, &prefix));
    published
        .into_iter()
        .filter(|text| !text.is_empty())
        .collect::<Vec<_>>()
        .join("\n\n")
}

fn scopes_table(service: &ServiceDef) -> String {
    let cases: Vec<String> = service
        .operations
        .iter()
        .filter(|operation| operation.is_mcp_tool() && !operation.mcp.scopes.is_empty())
        .map(|operation| {
            let needed: Vec<String> = operation
                .mcp
                .scopes
                .iter()
                .map(|(scope, _)| quoted(scope))
                .collect();
            format!(
                "      case {}:\n        return const <String>[{}];\n",
                quoted(operation.mcp_tool_name()),
                needed.join(", ")
            )
        })
        .collect();
    if cases.is_empty() {
        return String::new();
    }
    format!(
        "  if (method == 'tools/call') {{\n    switch (name) {{\n{}    }}\n  }}\n",
        cases.concat()
    )
}

fn answer_helpers(named: &str, prefix: &str, calls: &Calls) -> String {
    let fields = fault_fields_typescript_name(named);
    let mut text = format!(
        "{fields} _{prefix}McpFault({named}FaultKind kind, String operation, String detail, {{String? field}}) =>\n    \
         {fields}(detail: detail, field: field, kind: kind, operation: operation);\n\n\
         String? _{prefix}McpField(String message) {{\n  \
         if (!message.startsWith(\"'\")) return null;\n  \
         final end = message.indexOf(\"'\", 1);\n  \
         return end < 0 ? null : message.substring(1, end);\n\
         }}\n\n\
         {named}McpAnswer _{prefix}McpFailed(String text) => (\n  \
         result: <String, dynamic>{{\n    \
         'content': <Object>[<String, dynamic>{{'text': text, 'type': 'text'}}],\n    \
         'isError': true,\n  \
         }},\n  \
         error: null,\n\
         );\n\n\
         {named}McpAnswer _{prefix}McpFaultResult({fields} fault) =>\n    \
         _{prefix}McpFailed(_{prefix}McpJson(<String, dynamic>{{'fault': fault.toJson(), 'isServiceFault': true}}));"
    );
    if calls.empty {
        let _ = write!(
            text,
            "\n\n{named}McpAnswer _{prefix}McpEmpty() => (\n  \
             result: <String, dynamic>{{'content': <Object>[], 'isError': false}},\n  \
             error: null,\n\
             );"
        );
    }
    if calls.ok {
        let _ = write!(
            text,
            "\n\n{named}McpAnswer _{prefix}McpOk(Object? structured) => (\n  \
             result: <String, dynamic>{{\n    \
             'content': <Object>[<String, dynamic>{{'text': _{prefix}McpJson(structured), 'type': 'text'}}],\n    \
             'isError': false,\n    \
             'structuredContent': structured,\n  \
             }},\n  \
             error: null,\n\
             );"
        );
    }
    if calls.declared {
        let _ = write!(
            text,
            "\n\n{named}McpAnswer _{prefix}McpDeclared(Object? error) => _{prefix}McpFailed(_{prefix}McpJson(error));\n\n\
             {named}McpAnswer _{prefix}McpPanic(String tool, String operation, String detail) => (\n  \
             result: null,\n  \
             error: <String, dynamic>{{\n    \
             'code': -32603,\n    \
             'data': <String, dynamic>{{\n      \
             'fault': _{prefix}McpFault({named}FaultKind.handlerPanic, operation, detail).toJson(),\n      \
             'isServiceFault': true,\n    \
             }},\n    \
             'message': '$tool: $detail',\n  \
             }},\n\
             );"
        );
    }
    text
}

fn pascal(operation: &OperationDef) -> String {
    RenameRule::PascalCase.apply_to_field(&written(&operation.ident))
}

fn source_expr(prefix: &str, operation: &OperationDef, shape: &HttpShape) -> String {
    match wrapped_at_expansion(operation, shape) {
        Some(true) => "members['value']".to_owned(),
        Some(false) if shape.header_in.is_empty() => "members".to_owned(),
        Some(false) => {
            let kept = shape
                .header_in
                .iter()
                .map(|header| format!("entry.key != {}", quoted(&header.name)))
                .collect::<Vec<_>>()
                .join(" && ");
            format!(
                "<String, dynamic>{{for (final entry in members.entries) if ({kept}) entry.key: entry.value}}"
            )
        }
        None => format!(
            "(_{prefix}McpServerWrapped[{}]! ? members['value'] : members)",
            quoted(operation.mcp_tool_name())
        ),
    }
}

fn server_call(named: &str, prefix: &str, operation: &OperationDef) -> String {
    let shape = HttpShape::of(operation);
    let wire = quoted(&operation.wire_name);
    let call = dart_call(operation);
    let req_ty = message_dart_typename(operation);
    let source = source_expr(prefix, operation, &shape);
    let decode = message_decode(operation, "source");
    let kind = format!("{named}FaultKind");
    let mut text = format!(
        "Future<{named}McpAnswer> _{prefix}McpCall{op}<Ctx>(\n  \
         {named}Handlers<Ctx> handlers,\n  \
         Ctx ctx,\n  \
         Object? arguments,\n\
         ) async {{\n  \
         final Map<String, dynamic> members;\n  \
         if (arguments is Map<String, dynamic>) {{\n    \
         members = arguments;\n  \
         }} else if (arguments == null) {{\n    \
         members = <String, dynamic>{{}};\n  \
         }} else {{\n    \
         return _{prefix}McpFaultResult(\n      \
         _{prefix}McpFault({kind}.undeserializablePayload, {wire}, 'the arguments are not an object'),\n    \
         );\n  \
         }}\n  \
         final dynamic source = {source};\n  \
         final {req_ty} decoded;\n  \
         try {{\n    \
         decoded = {decode};\n  \
         }} on FormatException catch (rejected) {{\n    \
         return _{prefix}McpFaultResult(\n      \
         _{prefix}McpFault({kind}.failedValidation, {wire}, rejected.message, field: _{prefix}McpField(rejected.message)),\n    \
         );\n  \
         }} catch (rejected) {{\n    \
         return _{prefix}McpFaultResult(_{prefix}McpFault({kind}.undeserializablePayload, {wire}, '$rejected'));\n  \
         }}\n",
        op = pascal(operation),
    );
    let mut locals = Vec::new();
    for header in &shape.header_in {
        let parameter = dart_parameter(&header.parameter);
        let name = quoted(&header.name);
        let decoded = dart_json_decode(&header.ty, &format!("members[{name}]"));
        let dart_ty = dart_type_of(&header.ty);
        let _ = write!(
            text,
            "  final {dart_ty} {parameter};\n  \
             try {{\n    \
             {parameter} = {decoded};\n  \
             }} catch (rejected) {{\n    \
             return _{prefix}McpFaultResult(\n      \
             _{prefix}McpFault({kind}.failedValidation, {wire}, '$rejected', field: {name}),\n    \
             );\n  \
             }}\n"
        );
        locals.push(parameter);
    }
    let header_args = header_call_args(&locals);
    match &operation.outcome {
        OperationOutcome::OneWay => {
            let _ = write!(
                text,
                "  try {{\n    \
             await handlers.{call}(ctx, decoded{header_args});\n  \
             }} catch (unexpected) {{\n    \
             return _{prefix}McpEmpty();\n  \
             }}\n  \
             return _{prefix}McpEmpty();\n\
             }}"
            );
        }
        OperationOutcome::Reply {
            error: _error,
            success: _success,
        } => {
            text.push_str(&reply_tail(prefix, operation, &shape, &header_args));
        }
    }
    text
}

/// The call of a reply operation's handler, and the three ways it ends: an answer, its declared
/// error, or anything else it throws.
fn reply_tail(
    prefix: &str,
    operation: &OperationDef,
    shape: &HttpShape,
    header_args: &str,
) -> String {
    let OperationOutcome::Reply { error, success } = &operation.outcome else {
        return String::new();
    };
    let tool = quoted(operation.mcp_tool_name());
    let wire = quoted(&operation.wire_name);
    let call = dart_call(operation);
    let error_ty = dart_type_of(error);
    let unit = is_unit_type(success);
    let call_stmt = if unit {
        format!("await handlers.{call}(ctx, decoded{header_args});")
    } else {
        format!("final answered = await handlers.{call}(ctx, decoded{header_args});")
    };
    let (value_pre, value_expr, value_headers) = if unit {
        (String::new(), "null".to_owned(), None)
    } else {
        split_reply_record(
            "answered",
            "bodyOut",
            "headersOut",
            success,
            &shape.header_out,
        )
    };
    let ok = if unit {
        format!("_{prefix}McpEmpty()")
    } else if let Some(headers) = value_headers {
        format!("_{prefix}McpOk(<String, dynamic>{{'value': {value_expr}, ...{headers}}})")
    } else {
        format!("_{prefix}McpOk({value_expr})")
    };
    let (error_pre, error_expr, error_headers) = split_reply_record(
        "declared",
        "declaredHead",
        "errorHeadersOut",
        error,
        &shape.error_header_out,
    );
    let declared = error_headers.map_or_else(
        || error_expr.clone(),
        |headers| format!("<String, dynamic>{{'value': {error_expr}, ...{headers}}}"),
    );
    format!(
        "  try {{\n    \
         {call_stmt}\n\
{value_pre}    \
         return {ok};\n  \
         }} on {error_ty} catch (declared) {{\n\
{error_pre}    \
         return _{prefix}McpDeclared({declared});\n  \
         }} catch (unexpected) {{\n    \
         return _{prefix}McpPanic({tool}, {wire}, '$unexpected');\n  \
         }}\n\
         }}"
    )
}

/// `create{Service}McpRoute`: answers `tools/call` for the tool each operation names.
fn route_fn(service: &ServiceDef, named: &str, prefix: &str) -> String {
    let head = format!(
        "/// Answers the requests of this service on an endpoint shared by several, and `null` for a\n\
         /// request it does not own.\n\
         Future<{named}McpAnswer?> Function(Ctx ctx, String method, Map<String, dynamic> params)\n    \
         create{named}McpRoute<Ctx>({named}Handlers<Ctx> handlers) =>"
    );
    if service.operations.is_empty() {
        return format!("{head}\n        (ctx, method, params) async => null;");
    }
    let mut arms = String::new();
    for operation in &service.operations {
        let _ = writeln!(
            arms,
            "        case {}:\n          return _{prefix}McpCall{}(handlers, ctx, params['arguments']);",
            quoted(operation.mcp_tool_name()),
            pascal(operation)
        );
    }
    format!(
        "{head}\n        \
         (ctx, method, params) async {{\n      \
         if (method != 'tools/call') return null;\n      \
         switch (params['name']) {{\n\
{arms}        \
         default:\n          \
         return null;\n      \
         }}\n    \
         }};"
    )
}

/// The client text: the transport a consumer implements with one `send`, and the client with a
/// method per operation.
pub fn client_text(declared: &ServiceDef) -> String {
    let service = &declared.enveloped();
    let named = service.ident.to_string();
    let prefix = RenameRule::CamelCase.apply_to_variant(&named);
    let fields = fault_fields_typescript_name(&named);
    let with_fields = |template: &str| filled(template, &named, &prefix).replace("@F@", &fields);
    let mut published = vec![with_fields(CLIENT_HEAD)];
    if has_one_way(service) {
        published.push(format!(
            "/// What a one-way `{named}` `mcp` method throws when it cannot deliver its call.\n\
             class {named}McpRefusal implements Exception {{\n  \
             {named}McpRefusal(this.fault);\n  \
             final {fields} fault;\n  \
             @override\n  \
             String toString() =>\n      \
             '{named}McpRefusal: ${{fault.kind}} in `${{fault.operation}}`: ${{fault.detail}}';\n\
             }}"
        ));
    }
    if service.operations.is_empty() {
        published.push(format!(
            "/// A `{named}` caller over `mcp`.\n\
             class {named}McpClient {{\n  \
             {named}McpClient({named}McpTransport transport);\n\
             }}"
        ));
        return published.join("\n\n");
    }
    published.push(with_fields(CLIENT_CALL));
    let table = wrapped_table(&prefix, "Client", service);
    if !table.is_empty() {
        published.push(format!(
            "{}\nMap<String, dynamic> _{prefix}McpClientArguments(String tool, Object? sent) =>\n    \
             _{prefix}McpClientWrapped[tool]! ? <String, dynamic>{{'value': sent}} : sent as Map<String, dynamic>;",
            table.trim_end()
        ));
    }
    if has_one_way(service) {
        published.push(with_fields(CLIENT_FAULT_OF));
    }
    let methods = service
        .operations
        .iter()
        .map(|operation| client_method(&named, &prefix, operation))
        .collect::<Vec<_>>()
        .join("\n\n");
    published.push(format!(
        "/// A `{named}` caller over `mcp`.\n\
         class {named}McpClient {{\n  \
         {named}McpClient(this._transport);\n  \
         final {named}McpTransport _transport;\n\n\
         {call}\n\n\
         {methods}\n\
         }}",
        call = with_fields(CLIENT_CALL_METHOD),
    ));
    published.join("\n\n")
}

fn arguments_stmt(prefix: &str, operation: &OperationDef, shape: &HttpShape) -> String {
    let sent = message_encode(operation);
    let mut stmt = match wrapped_at_expansion(operation, shape) {
        Some(true) => format!("    final arguments = <String, dynamic>{{'value': {sent}}};\n"),
        Some(false) if shape.header_in.is_empty() => format!("    final arguments = {sent};\n"),
        Some(false) => format!("    final arguments = <String, dynamic>{{...{sent}}};\n"),
        None => format!(
            "    final arguments = _{prefix}McpClientArguments({}, {sent});\n",
            quoted(operation.mcp_tool_name())
        ),
    };
    for header in &shape.header_in {
        let pushed = header_out_push_stmt(
            "arguments",
            &header.name,
            &dart_parameter(&header.parameter),
            &header.ty,
        );
        for line in pushed.lines() {
            stmt.push_str(line.strip_prefix("        ").unwrap_or(line));
            stmt.push('\n');
        }
    }
    stmt
}

fn method_head(prefix: &str, operation: &OperationDef, shape: &HttpShape) -> MethodHead {
    let mut params = vec![format!("{} req", message_dart_typename(operation))];
    for header in &shape.header_in {
        params.push(format!(
            "{} {}",
            dart_type_of(&header.ty),
            dart_parameter(&header.parameter)
        ));
    }
    MethodHead {
        arguments: arguments_stmt(prefix, operation, shape),
        call: dart_call(operation),
        doc: format!(
            "  /// Calls `{}` over `mcp`.",
            super::on_one_line(&operation.wire_name)
        ),
        parameters: params.join(", "),
        tool: quoted(operation.mcp_tool_name()),
        wire: quoted(&operation.wire_name),
    }
}

fn client_method(named: &str, prefix: &str, operation: &OperationDef) -> String {
    let shape = HttpShape::of(operation);
    let head = method_head(prefix, operation, &shape);
    match &operation.outcome {
        OperationOutcome::OneWay => one_way_method(named, prefix, &head),
        OperationOutcome::Reply { error, success } => {
            reply_method(named, prefix, operation, &shape, &head, (success, error))
        }
    }
}

fn one_way_method(named: &str, prefix: &str, head: &MethodHead) -> String {
    let MethodHead {
        arguments,
        call,
        doc,
        parameters,
        tool,
        wire,
    } = head;
    format!(
        "{doc}\n  \
         Future<void> {call}({parameters}) async {{\n\
{arguments}    \
         final Map<String, dynamic> result;\n    \
         try {{\n      \
         result = await _call({tool}, {wire}, arguments);\n    \
         }} on _{named}McpCallFault catch (failure) {{\n      \
         throw {named}McpRefusal(failure.fault);\n    \
         }}\n    \
         if (result['isError'] == true) {{\n      \
         throw {named}McpRefusal(_{prefix}McpFaultOf(result, {wire}));\n    \
         }}\n  \
         }}"
    )
}

fn reply_method(
    named: &str,
    prefix: &str,
    operation: &OperationDef,
    shape: &HttpShape,
    head: &MethodHead,
    (success, error): (&syn::Type, &syn::Type),
) -> String {
    let MethodHead {
        arguments,
        call,
        doc,
        parameters,
        tool,
        wire,
    } = head;
    let result = result_name(named, operation).unwrap_or_default();
    let fields = fault_fields_typescript_name(named);
    let read_failed = format!("_{prefix}McpReadFailed");
    let ws_failed = format!("_{prefix}WsFailedValidation");
    let unit_ok = is_unit_type(success) && shape.header_out.is_empty();
    let success_block = success_decode_block(&result, prefix, wire, success, &shape.header_out)
        .replace(&ws_failed, &read_failed);
    let reply = if unit_ok {
        String::new()
    } else if shape.header_out.is_empty() {
        "      final reply = <String, dynamic>{'value': result['structuredContent']};\n".to_owned()
    } else {
        "      final structured = result['structuredContent'];\n      \
         final reply = <String, dynamic>{\n        \
         'value': structured is Map<String, dynamic> ? structured['value'] : null,\n        \
         'headers': structured is Map<String, dynamic>\n            \
         ? <String, dynamic>{\n                \
         for (final entry in structured.entries)\n                  \
         if (entry.key != 'value') entry.key: entry.value,\n              \
         }\n            \
         : <String, dynamic>{},\n      \
         };\n"
            .to_owned()
    };
    let error_block =
        declared_error_decode_stmts(&result, prefix, wire, error, &shape.error_header_out)
            .replace(&ws_failed, &read_failed);
    let error_source = if shape.error_header_out.is_empty() {
        "    final dynamic error = parsed;\n".to_owned()
    } else {
        "    final reply = <String, dynamic>{\n      \
         'headers': parsed is Map<String, dynamic>\n          \
         ? <String, dynamic>{\n              \
         for (final entry in parsed.entries)\n                \
         if (entry.key != 'value') entry.key: entry.value,\n            \
         }\n          \
         : <String, dynamic>{},\n    \
         };\n    \
         final dynamic error = parsed is Map<String, dynamic> ? parsed['value'] : null;\n"
            .to_owned()
    };
    format!(
        "{doc}\n  \
         Future<{result}> {call}({parameters}) async {{\n\
{arguments}    \
         final Map<String, dynamic> result;\n    \
         try {{\n      \
         result = await _call({tool}, {wire}, arguments);\n    \
         }} on _{named}McpCallFault catch (failure) {{\n      \
         return {result}Fault(failure.fault);\n    \
         }}\n    \
         if (result['isError'] != true) {{\n\
{reply}{success_block}    \
         }}\n    \
         final Object? parsed;\n    \
         try {{\n      \
         parsed = jsonDecode(((result['content'] as List).first as Map<String, dynamic>)['text'] as String);\n    \
         }} catch (rejected) {{\n      \
         return {result}Fault({read_failed}({wire}, '$rejected'));\n    \
         }}\n    \
         if (parsed is Map<String, dynamic> && parsed['isServiceFault'] == true) {{\n      \
         late final {fields} fault;\n      \
         try {{\n        \
         fault = {fields}.fromJson(parsed['fault'] as Map<String, dynamic>);\n      \
         }} catch (rejected) {{\n        \
         return {result}Fault({read_failed}({wire}, '$rejected'));\n      \
         }}\n      \
         return {result}Fault(fault);\n    \
         }}\n\
{error_source}{error_block}  \
         }}"
    )
}

/// `dart_mcp_service()` and `dart_mcp_client()`, for a service that asks for `mcp`. They write the
/// tool list, the names, the instructions, the version and each `wrapped_{operation}()` over the
/// markers when they run, since only the compiled types know them.
pub fn seam(declared: &ServiceDef) -> TokenStream {
    let server = service_text(declared);
    let client = client_text(declared);
    let wrapped: Vec<TokenStream> = declared
        .enveloped()
        .operations
        .iter()
        .map(|operation| {
            let marker = wrapped_marker(operation);
            let method = format_ident!(
                "wrapped_{}",
                written(&operation.ident),
                span = operation.ident.span()
            );
            quote! { .replace(#marker, &Self::#method().to_string()) }
        })
        .collect();
    let wrapped_server = wrapped.clone();
    quote! {
        #[doc = " The service's generated Dart `mcp` server: its catalog and names, the route"]
        #[doc = " for the endpoint it sits on, the function that serves an endpoint, and the"]
        #[doc = " dispatchers over stdio and HTTP. Takes the handlers `dart_ws_client()` declares,"]
        #[doc = " so a bundle names that first."]
        pub fn dart_mcp_service() -> ::std::string::String {
            fn literal(text: &str) -> ::std::string::String {
                ::serde_json::Value::String(text.to_owned())
                    .to_string()
                    .replace('$', "\\$")
            }
            let tools = Self::mcp_tools()
                .iter()
                .map(|tool| literal(&tool.to_string()))
                .collect::<::std::vec::Vec<_>>()
                .join(", ");
            let names = Self::MCP_NAMES
                .iter()
                .map(|(kind, name)| literal(&format!("{kind}:{name}")))
                .collect::<::std::vec::Vec<_>>()
                .join(", ");
            let instructions = match Self::MCP_INSTRUCTIONS {
                ::core::option::Option::Some(text) => literal(text),
                ::core::option::Option::None => ::std::string::String::from("null"),
            };
            #server
                .replace(#TOOLS_MARKER, &tools)
                .replace(#NAMES_MARKER, &names)
                .replace(#INSTRUCTIONS_MARKER, &instructions)
                .replace(#VERSION_MARKER, env!("CARGO_PKG_VERSION"))
                #(#wrapped_server)*
        }

        #[doc = " The service's generated Dart `mcp` client: the transport a consumer implements"]
        #[doc = " with one `send`, and the client with a method per operation. Answers the results"]
        #[doc = " `dart_definition()` declares, so a bundle names that first."]
        pub fn dart_mcp_client() -> ::std::string::String {
            #client
                .replace(#VERSION_MARKER, env!("CARGO_PKG_VERSION"))
                #(#wrapped)*
        }
    }
}
