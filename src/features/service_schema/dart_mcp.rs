//! The Dart `mcp` server and client: a catalog and a route a service puts on an endpoint, the
//! function that answers one JSON-RPC message for the services of that endpoint, the HTTP rules
//! around it, and a client that calls each operation as a tool.
//!
//! Neither text names a tool list or whether a message sits under `value`: the accessors in
//! [`seam`] write those over the markers below when they run in the consuming program.

use super::dart_http_client::{dart_call, dart_parameter, message_type};
use super::dart_ws_client::{
    dart_type_of, declared_error_decode_stmts, has_one_way, header_call_args, header_out_push_stmt,
    message_dart_typename, message_decode, message_encode, split_reply_record,
    success_decode_block,
};
use super::mcp_scopes::{ReadUri, ScopeTable};
use super::result::result_name;
use crate::features::dart::{
    dart_json_decode, dart_json_decode_unjudged, dart_json_encode, quoted,
};
use crate::rename_rule::RenameRule;
use crate::service_schema::parse::{
    HttpShape, OperationDef, OperationInputs, OperationOutcome, PathSegment, ServiceDef,
    is_unit_type, wire_key, written,
};
use crate::service_schema::support::fault_fields_typescript_name;
use crate::service_schema::transport::mcp::resources::Resourced;
use core::fmt::Write as _;
use proc_macro2::TokenStream;
use quote::{format_ident, quote};

const TOOLS_MARKER: &str = "@TOOLS@";
const NAMES_MARKER: &str = "@NAMES@";
const INSTRUCTIONS_MARKER: &str = "@INSTRUCTIONS@";
const VERSION_MARKER: &str = "@VERSION@";
const RESOURCES_MARKER: &str = "@RESOURCES@";
const TEMPLATES_MARKER: &str = "@TEMPLATES@";

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
  resources: <String>[@RESOURCES@],
  resourceTemplates: <String>[@TEMPLATES@],
  prompts: <String>[],
  skills: <String>[],
  capabilities: <String>[@CAPABILITIES@],
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

/// What the application's source of changes hands to a `subscriptions/listen` stream.
sealed class @N@McpChange {
  const @N@McpChange();
}

/// The list of resources changed.
class @N@McpResourcesListChanged extends @N@McpChange {
  const @N@McpResourcesListChanged();
}

/// One resource changed.
class @N@McpResourceUpdated extends @N@McpChange {
  const @N@McpResourceUpdated(this.uri);
  final String uri;
}

/// One task moved.
class @N@McpTaskMoved extends @N@McpChange {
  const @N@McpTaskMoved(this.taskId);
  final String taskId;
}

/// Keeps an event stream open: the comment line `:` on HTTP, nothing on stdio.
class @N@McpKeepAlive extends @N@McpChange {
  const @N@McpKeepAlive();
}

/// The application's source of changes: each call answers the next change, and `null` ends the stream.
typedef @N@McpChanges = Future<@N@McpChange?> Function();

/// The request in flight, handed to `admit` and to the function that builds `Ctx`, so that a
/// handler reaches it. `token` is the request's `progressToken`, `writer` takes what it writes
/// beyond its answer, and `comment` writes the comment line that keeps an event stream open.
class @N@McpCall {
  @N@McpCall({Object? token, @N@McpWriter? writer, Future<void> Function()? comment})
      : _token = token,
        _writer = writer,
        _comment = comment;
  final Object? _token;
  final @N@McpWriter? _writer;
  final Future<void> Function()? _comment;
  final Completer<void> _stopped = Completer<void>();
  bool _gone = false;

  void _cancel() {
    _gone = true;
    if (!_stopped.isCompleted) _stopped.complete();
  }

  Future<bool> _write(String message) async {
    if (_gone) return false;
    try {
      await _writer?.call(message);
      return true;
    } catch (_) {
      _cancel();
      return false;
    }
  }

  Future<bool> _keepAlive() async {
    if (_gone) return false;
    try {
      await _comment?.call();
      return true;
    } catch (_) {
      _cancel();
      return false;
    }
  }

  /// Writes `notifications/progress` where the request carries a `progressToken`, and does nothing
  /// otherwise. A write that fails cancels the call.
  Future<void> progress(num done, {num? total, String? message}) async {
    final token = _token;
    if (_writer == null || token == null) return;
    await _write(_@P@McpJson(<String, dynamic>{
      'jsonrpc': '2.0',
      'method': 'notifications/progress',
      'params': <String, dynamic>{
        'progress': done,
        'progressToken': token,
        if (total != null) 'total': total,
        if (message != null) 'message': message,
      },
    }));
  }

  /// Whether the client cancelled this request, or went away.
  bool get cancelled => _gone;
}

/// The requests a stdio host has in flight, so that `notifications/cancelled` reaches the right one.
class @N@McpInFlight {
  final Map<String, @N@McpCall> _calls = <String, @N@McpCall>{};

  void _cancel(Object? id) {
    _calls[jsonEncode(id)]?._cancel();
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
  const @N@McpHost({required this.writer, required this.inFlight, this.changes});
  final @N@McpWriter writer;
  final @N@McpInFlight inFlight;

  /// Where `subscriptions/listen` takes its changes from; `null` where the host offers none.
  final @N@McpChanges? changes;
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

const _@P@McpStopped = Object();

/// The `subscriptions/listen` of one call: the acknowledgment, then what `changes` yields that the
/// client asked for, until it yields `null` or the call is cancelled.
Future<({String? text, int status})> _@P@McpListen(
  Object? id,
  Map<String, dynamic> params,
  String info,
  @N@McpEndpoint endpoint,
  @N@McpCall call,
  @N@McpChanges changes,
) async {
  ({String? text, int status}) invalid(String detail) => (
        text: _@P@McpJson(<String, dynamic>{
          'error': <String, dynamic>{'code': -32602, 'message': 'Invalid params: $detail'},
          'id': id,
          'jsonrpc': '2.0',
        }),
        status: 200,
      );
  final filter = params['notifications'];
  if (filter is! Map<String, dynamic>) return invalid('subscriptions/listen needs `notifications`');
  final uris = filter['resourceSubscriptions'];
  if (filter.containsKey('resourcesListChanged') && filter['resourcesListChanged'] is! bool) {
    return invalid('`resourcesListChanged` is not a boolean');
  }
  if (filter.containsKey('resourceSubscriptions') && !(uris is List && uris.every((uri) => uri is String))) {
    return invalid('`resourceSubscriptions` is not an array of strings');
  }
  final agreed = <String, dynamic>{};
  final offersResources = endpoint.catalogs.any(
    (catalog) => catalog.capabilities.any((written) => (jsonDecode(written) as Map).containsKey('resources')),
  );
  if (offersResources) {
    if (filter['resourcesListChanged'] == true) agreed['resourcesListChanged'] = true;
    if (uris is List && uris.isNotEmpty) agreed['resourceSubscriptions'] = uris;
  }
  const subscription = 'io.modelcontextprotocol/subscriptionId';
  String notified(String method, Map<String, dynamic> given) => _@P@McpJson(<String, dynamic>{
        'jsonrpc': '2.0',
        'method': method,
        'params': <String, dynamic>{...given, '_meta': <String, dynamic>{subscription: id}},
      });
  final lists = agreed['resourcesListChanged'] == true;
  final watched = <String>{
    if (agreed['resourceSubscriptions'] case final List<dynamic> held) ...held.cast<String>(),
  };
  final meta = Map<String, dynamic>.of(jsonDecode(info) as Map<String, dynamic>)..[subscription] = id;
  final ended = (
    text: _@P@McpJson(<String, dynamic>{
      'id': id,
      'jsonrpc': '2.0',
      'result': <String, dynamic>{'_meta': meta, 'resultType': 'complete'},
    }),
    status: 200,
  );
  if (!await call._write(notified('notifications/subscriptions/acknowledged', <String, dynamic>{'notifications': agreed}))) {
    return ended;
  }
  while (true) {
    final Object? change = await Future.any<Object?>(<Future<Object?>>[
      changes(),
      call._stopped.future.then<Object?>((_) => _@P@McpStopped),
    ]);
    if (call.cancelled || change == null || identical(change, _@P@McpStopped)) return ended;
    final written = switch (change) {
      @N@McpResourcesListChanged() when lists =>
        await call._write(notified('notifications/resources/list_changed', const <String, dynamic>{})),
      @N@McpResourceUpdated(:final uri) when watched.contains(uri) =>
        await call._write(notified('notifications/resources/updated', <String, dynamic>{'uri': uri})),
      @N@McpKeepAlive() => await call._keepAlive(),
      _ => true,
    };
    if (!written) return ended;
  }
}

Future<({String? text, int status})> _@P@McpRespond(
  String text,
  String serverName,
  String serverVersion,
  String? instructions,
  @N@McpEndpoint endpoint,
  Future<@N@McpAnswer?> Function(int part, String method, Map<String, dynamic> params) route,
  Future<({String? text, int status})> Function(Object? id, Map<String, dynamic> params, String info)? listening,
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
  for (final key in const ['io.modelcontextprotocol/protocolVersion', 'io.modelcontextprotocol/clientCapabilities']) {
    if (meta is! Map || !meta.containsKey(key)) {
      return failed(id, 400, -32602, 'Missing required _meta key: $key');
    }
  }
  final version = meta['io.modelcontextprotocol/protocolVersion'];
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

  if (method == 'subscriptions/listen' && listening != null) return listening(id, params, info);

  if (method == 'server/discover') {
    final capabilities = <String, dynamic>{};
    for (final catalog in endpoint.catalogs) {
      for (final written in catalog.capabilities) {
        _@P@McpMerge(capabilities, jsonDecode(written) as Map<String, dynamic>);
      }
    }
    final resources = capabilities['resources'];
    if (listening != null && resources is Map<String, dynamic>) {
      resources['listChanged'] = true;
      resources['subscribe'] = true;
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
      final written = colon < 0 ? '' : cursor.substring(0, colon);
      final at = RegExp(r'^\+?[0-9]+$').hasMatch(written) ? int.tryParse(written) : null;
      if (at == null || at >= catalogs.length) return failed(id, 200, -32602, 'Invalid cursor');
      place = at;
      final rest = cursor.substring(colon + 1);
      inner = rest.isEmpty ? null : rest;
    }
    final items = <String>[
      if (cursor == null) for (final catalog in catalogs) ...catalog.resources,
    ];
    var ttl = 0;
    var scope = 'private';
    String? next;
    for (var part = place; part < catalogs.length; part += 1) {
      final page = await route(part, 'resources/list', <String, dynamic>{
        if (part == place && inner != null) 'cursor': inner,
      });
      if (page == null) continue;
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
        if (result['ttlMs'] is int) ttl = result['ttlMs'] as int;
        if (result['cacheScope'] is String) scope = result['cacheScope'] as String;
        final following = result['nextCursor'];
        if (following is String && following.isNotEmpty) next = '$part:$following';
      }
      if (next == null && part + 1 < catalogs.length) next = '${part + 1}:';
      break;
    }
    return answered(_@P@McpObject(<String, String>{
      '_meta': info,
      'cacheScope': '"$scope"',
      if (next != null) 'nextCursor': _@P@McpJson(next),
      'resultType': '"complete"',
      'resources': '[${items.join(',')}]',
      'ttlMs': '$ttl',
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
  final String? named;
  switch (method) {
    case 'tools/call':
      named = params['name'] is String ? params['name'] as String : null;
    case 'resources/read':
      named = params['uri'] is String ? params['uri'] as String : null;
    case 'completion/complete':
      final argument = params['argument'];
      final reference = params['ref'];
      named = argument is Map && argument['name'] is String && reference is Map && reference['uri'] is String
          ? reference['uri'] as String
          : null;
    default:
      named = null;
  }
  if (named == null && (method == 'tools/call' || method == 'resources/read' || method == 'completion/complete')) {
    return failed(id, 200, -32602, 'Invalid params: $method names nothing to answer');
  }
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
  final asked = named ?? params['name'] ?? params['uri'] ?? method;
  final unknown = switch (method) {
    'tools/call' => 'Unknown tool',
    'resources/read' => 'Resource not found',
    'completion/complete' => 'Unknown completion',
    _ => null,
  };
  return failed(
    id,
    200,
    -32602,
    unknown == null ? '$method: nothing on this endpoint answers it' : '$unknown: $asked',
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
/// cancelled request is dropped. A host that hands in `host.changes` also answers
/// `subscriptions/listen`.
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
  final changes = host.changes;
  try {
    final answered = await _@P@McpRespond(text, serverName, @P@McpServerVersion, instructions, endpoint,
        (part, method, params) {
      if (building) {
        building = false;
        leave = host.inFlight._enter(message?['id'], call);
        built = ctx(call);
      }
      return route(built as Ctx, part, method, params);
    }, changes == null
        ? null
        : (id, params, info) {
            leave = host.inFlight._enter(message?['id'], call);
            return _@P@McpListen(id, params, info, endpoint, call, changes);
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
@N@McpHttpResponse _@P@McpEventStream(List<String?> events, String? answered) => @N@McpHttpResponse(
      status: 200,
      headers: const [('Content-Type', 'text/event-stream'), ('X-Accel-Buffering', 'no')],
      body: utf8.encode([
        for (final event in [...events, if (answered != null) answered])
          event == null ? ':\n\n' : 'event: message\ndata: $event\n\n',
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
/// `admit` answered. A `subscriptions/listen` answers an event stream where `changes` is handed in:
/// it ends when `changes` answers `null`.
Future<@N@McpHttpResponse> @P@McpServeHttp<Ctx>(
  @N@McpHttpRequest request,
  @N@McpHttpSettings settings,
  String serverName,
  String? instructions,
  @N@McpEndpoint endpoint,
  @N@McpAdmit<Ctx> admit,
  Future<@N@McpAnswer?> Function(Ctx ctx, int part, String method, Map<String, dynamic> params) route, [
  @N@McpChanges? changes,
]) async {
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
  final events = <String?>[];
  final call = @N@McpCall(
    token: _@P@McpBodyToken(request),
    writer: (event) async => events.add(event),
    comment: () async => events.add(null),
  );
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
  }, changes == null
      ? null
      : (id, params, info) {
          routed = true;
          return _@P@McpListen(id, params, info, endpoint, call, changes);
        });
  if (!routed) return respond(answered.status, answered.text);
  final written = call.cancelled ? null : answered.text;
  if (_@P@McpToken(parsed) != null || method == 'subscriptions/listen') return _@P@McpEventStream(events, written);
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
  @N@McpInFlight inFlight, [
  @N@McpChanges? changes,
]) create@N@McpDispatcher<Ctx>(
  @N@Handlers<Ctx> handlers,
) {
  final route = create@N@McpRoute(handlers);
  return (text, ctx, writer, inFlight, [changes]) => @P@McpServe<Ctx>(
        text,
        @P@McpServerName,
        @P@McpInstructions,
        _@P@McpAlone,
        ctx,
        @N@McpHost(writer: writer, inFlight: inFlight, changes: changes),
        (built, part, method, params) => route(built, method, params),
      );
}

/// Answers the HTTP requests of this service alone. `admit` answers the `Ctx` of a request, or why
/// it is refused.
Future<@N@McpHttpResponse> Function(@N@McpHttpRequest) create@N@McpHttpDispatcher<Ctx>(
  @N@Handlers<Ctx> handlers,
  @N@McpHttpSettings settings,
  @N@McpAdmit<Ctx> admit, [
  @N@McpChanges? changes,
]) {
  final route = create@N@McpRoute(handlers);
  return (request) => @P@McpServeHttp<Ctx>(
        request,
        settings,
        @P@McpServerName,
        @P@McpInstructions,
        _@P@McpAlone,
        admit,
        (ctx, part, method, params) => route(ctx, method, params),
        changes,
      );
}";

const CAPTURED: &str =
    "/// What each placeholder of `segments` captures in `uri`: one non-empty run of text up to the
/// next `/`.
List<String>? _@P@McpCaptured(List<(String, bool)> segments, String uri) {
  var rest = uri;
  final captured = <String>[];
  for (final (text, placeholder) in segments) {
    if (placeholder) {
      final slash = rest.indexOf('/');
      final end = slash < 0 ? rest.length : slash;
      if (end == 0) return null;
      captured.add(rest.substring(0, end));
      rest = rest.substring(end);
    } else {
      if (!rest.startsWith(text)) return null;
      rest = rest.substring(text.length);
    }
  }
  return rest.isEmpty ? captured : null;
}";

const UNREAD: &str = "/// A request that would not read, or a handler that refused it: a `-32602`.
@N@McpAnswer _@P@McpUnread(@F@ fault) => (
      result: null,
      error: <String, dynamic>{
        'code': -32602,
        'data': <String, dynamic>{'fault': fault.toJson(), 'isServiceFault': true},
        'message': 'Invalid params',
      },
    );

@N@McpAnswer _@P@McpRefused(Object? error, String refusal) => (
      result: null,
      error: <String, dynamic>{'code': -32602, 'data': error, 'message': refusal},
    );";

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
        for operation in tools_of(service) {
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

/// The operations of `service` that are tools.
fn tools_of(service: &ServiceDef) -> impl Iterator<Item = &OperationDef> {
    service
        .operations
        .iter()
        .filter(|operation| operation.is_mcp_tool())
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
    let entries: Vec<String> = tools_of(service)
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
    let held = Resourced::of(service);
    let mut published = vec![
        filled(SERVER_HEAD, &named, &prefix)
            .replace("@CAPABILITIES@", &capabilities_literal(&held))
            .replace("@SCOPES@\n", &scopes_table(&ScopeTable::of(service))),
        page_types(service),
        wrapped_table(&prefix, "Server", service)
            .trim_end()
            .to_owned(),
    ];
    if !service.operations.is_empty() {
        published.push(answer_helpers(&named, &prefix, &calls, &held, service));
    }
    published.extend(tools_of(service).map(|operation| server_call(&named, &prefix, operation)));
    published.extend(resource_functions(service, &held, &named, &prefix));
    published.push(route_fn(service, &held, &named, &prefix));
    published.push(filled(SERVER_ALONE, &named, &prefix));
    published
        .into_iter()
        .filter(|text| !text.is_empty())
        .collect::<Vec<_>>()
        .join("\n\n")
}

fn scopes_table(table: &ScopeTable) -> String {
    let list = |scopes: &[String]| {
        format!(
            "const <String>[{}]",
            scopes
                .iter()
                .map(|scope| quoted(scope))
                .collect::<Vec<_>>()
                .join(", ")
        )
    };
    let mut text = String::new();
    if table.has_templates() {
        text.push_str(
            "  bool matches(String uri, List<(String, bool)> template) {\n    \
             var rest = uri;\n    \
             for (final (text, placeholder) in template) {\n      \
             if (placeholder) {\n        \
             final end = rest.indexOf('/');\n        \
             final stop = end < 0 ? rest.length : end;\n        \
             if (stop == 0) return false;\n        \
             rest = rest.substring(stop);\n      \
             } else if (rest.startsWith(text)) {\n        \
             rest = rest.substring(text.length);\n      \
             } else {\n        \
             return false;\n      \
             }\n    \
             }\n    \
             return rest.isEmpty;\n  \
             }\n\n",
        );
    }
    if !table.tools.is_empty() {
        text.push_str("  if (method == 'tools/call') {\n    switch (name) {\n");
        for (tool, scopes) in &table.tools {
            let _ = writeln!(
                text,
                "      case {}:\n        return {};",
                quoted(tool),
                list(scopes)
            );
        }
        text.push_str("    }\n  }\n");
    }
    if !table.reads.is_empty() {
        text.push_str("  if (method == 'resources/read' && name != null) {\n");
        for (uri, scopes) in &table.reads {
            match uri {
                ReadUri::Fixed(fixed) => {
                    let _ = writeln!(
                        text,
                        "    if (name == {}) return {};",
                        quoted(fixed),
                        list(scopes)
                    );
                }
                ReadUri::Template(parts) => {
                    let pairs: Vec<String> = parts
                        .iter()
                        .map(|(part, placeholder)| format!("({}, {placeholder})", quoted(part)))
                        .collect();
                    let _ = writeln!(
                        text,
                        "    if (matches(name, const <(String, bool)>[{}])) return {};",
                        pairs.join(", "),
                        list(scopes)
                    );
                }
            }
        }
        text.push_str("    return const <String>[];\n  }\n");
    }
    if let Some(scopes) = &table.listing {
        let _ = writeln!(
            text,
            "  if (method == 'resources/list') return {};",
            list(scopes)
        );
    }
    if !table.completions.is_empty() {
        text.push_str("  if (method == 'completion/complete' && name != null) {\n");
        for (uri, scopes) in &table.completions {
            let _ = writeln!(
                text,
                "    if (name == {}) return {};",
                quoted(uri),
                list(scopes)
            );
        }
        text.push_str("  }\n");
    }
    text
}

fn answer_helpers(
    named: &str,
    prefix: &str,
    calls: &Calls,
    held: &Resourced<'_>,
    service: &ServiceDef,
) -> String {
    let tools = tools_of(service).next().is_some();
    let panics = calls.declared || held.any();
    let fields = fault_fields_typescript_name(named);
    let mut text = format!(
        "{fields} _{prefix}McpFault({named}FaultKind kind, String operation, String detail, {{String? field}}) =>\n    \
         {fields}(detail: detail, field: field, kind: kind, operation: operation);\n\n\
         String? _{prefix}McpField(String message) {{\n  \
         if (!message.startsWith(\"'\")) return null;\n  \
         final end = message.indexOf(\"'\", 1);\n  \
         return end < 0 ? null : message.substring(1, end);\n\
         }}"
    );
    if tools {
        let _ = write!(
            text,
            "\n\n{named}McpAnswer _{prefix}McpFailed(String text) => (\n  \
             result: <String, dynamic>{{\n    \
             'content': <Object>[<String, dynamic>{{'text': text, 'type': 'text'}}],\n    \
             'isError': true,\n  \
             }},\n  \
             error: null,\n\
             );\n\n\
             {named}McpAnswer _{prefix}McpFaultResult({fields} fault) =>\n    \
             _{prefix}McpFailed(_{prefix}McpJson(<String, dynamic>{{'fault': fault.toJson(), 'isServiceFault': true}}));"
        );
    }
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
            "\n\n{named}McpAnswer _{prefix}McpDeclared(Object? error) => _{prefix}McpFailed(_{prefix}McpJson(error));"
        );
    }
    if panics {
        let _ = write!(
            text,
            "\n\n{named}McpAnswer _{prefix}McpPanic(String tool, String operation, String detail) => (\n  \
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

/// The message read by its `fromJsonUnjudged`: what throws here does not read, and a read that
/// answers where the judged one threw was refused by `validate()`.
fn unjudged_decode(operation: &OperationDef, payload: &str) -> String {
    let req_ty = message_dart_typename(operation);
    message_type(operation).map_or_else(
        || format!("{req_ty}.fromJsonUnjudged({payload} as Map<String, dynamic>)"),
        |ty| dart_json_decode_unjudged(&ty, payload),
    )
}

fn server_call(named: &str, prefix: &str, operation: &OperationDef) -> String {
    let shape = HttpShape::of(operation);
    let tool = quoted(operation.mcp_tool_name());
    let call = dart_call(operation);
    let req_ty = message_dart_typename(operation);
    let source = source_expr(prefix, operation, &shape);
    let decode = message_decode(operation, "source");
    let unjudged = unjudged_decode(operation, "source");
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
         _{prefix}McpFault({kind}.undeserializablePayload, {tool}, 'the arguments are not an object'),\n    \
         );\n  \
         }}\n  \
         final dynamic source = {source};\n  \
         final {req_ty} decoded;\n  \
         try {{\n    \
         decoded = {decode};\n  \
         }} on FormatException catch (rejected) {{\n    \
         try {{\n      \
         {unjudged};\n    \
         }} on FormatException catch (unread) {{\n      \
         return _{prefix}McpFaultResult(_{prefix}McpFault({kind}.undeserializablePayload, {tool}, unread.message));\n    \
         }} catch (unread) {{\n      \
         return _{prefix}McpFaultResult(_{prefix}McpFault({kind}.undeserializablePayload, {tool}, '$unread'));\n    \
         }}\n    \
         return _{prefix}McpFaultResult(\n      \
         _{prefix}McpFault({kind}.failedValidation, {tool}, rejected.message, field: _{prefix}McpField(rejected.message)),\n    \
         );\n  \
         }} catch (rejected) {{\n    \
         return _{prefix}McpFaultResult(_{prefix}McpFault({kind}.undeserializablePayload, {tool}, '$rejected'));\n  \
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
             _{prefix}McpFault({kind}.failedValidation, {tool}, '$rejected', field: {name}),\n    \
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
         return _{prefix}McpPanic({tool}, {tool}, '$unexpected');\n  \
         }}\n\
         }}"
    )
}

/// `create{Service}McpRoute`: answers `tools/call` for the tool each operation names, and the
/// resource, listing and completion requests for the operations that are those.
fn route_fn(service: &ServiceDef, held: &Resourced<'_>, named: &str, prefix: &str) -> String {
    let preamble = format!(
        "/// Answers the requests of this service on an endpoint shared by several, and `null` for a\n\
         /// request it does not own.\n\
         Future<{named}McpAnswer?> Function(Ctx ctx, String method, Map<String, dynamic> params)\n    \
         create{named}McpRoute<Ctx>({named}Handlers<Ctx> handlers) =>"
    );
    if service.operations.is_empty() {
        return format!("{preamble}\n        (ctx, method, params) async => null;");
    }
    let mut cases = String::new();
    let tools: Vec<&OperationDef> = tools_of(service).collect();
    if !tools.is_empty() {
        let mut arms = String::new();
        for operation in tools {
            let _ = writeln!(
                arms,
                "            case {}:\n              return _{prefix}McpCall{}(handlers, ctx, params['arguments']);",
                quoted(operation.mcp_tool_name()),
                pascal(operation)
            );
        }
        let _ = write!(
            cases,
            "        case 'tools/call':\n          switch (params['name']) {{\n{arms}            \
             default:\n              return null;\n          }}\n"
        );
    }
    if held.reads().next().is_some() {
        let _ = write!(
            cases,
            "        case 'resources/read':\n          final uri = params['uri'];\n          \
             return uri is String ? _{prefix}McpRead<Ctx>(handlers, ctx, uri) : null;\n"
        );
    }
    if held.listing.is_some() {
        let _ = write!(
            cases,
            "        case 'resources/list':\n          \
             return _{prefix}McpList<Ctx>(handlers, ctx, params['cursor']);\n"
        );
    }
    if !held.completions.is_empty() {
        let _ = write!(
            cases,
            "        case 'completion/complete':\n          \
             return _{prefix}McpComplete<Ctx>(handlers, ctx, params);\n"
        );
    }
    format!(
        "{preamble}\n        \
         (ctx, method, params) async {{\n      \
         switch (method) {{\n\
{cases}        \
         default:\n          \
         return null;\n      \
         }}\n    \
         }};"
    )
}

/// The `capabilities` the catalog lists, as the literals of their JSON text.
fn capabilities_literal(held: &Resourced<'_>) -> String {
    let mut capabilities = vec![quoted("{\"tools\":{}}")];
    if held.is_served() {
        capabilities.push(quoted("{\"resources\":{}}"));
    }
    if !held.completions.is_empty() {
        capabilities.push(quoted("{\"completions\":{}}"));
    }
    capabilities.join(", ")
}

/// `McpResource` and `McpResourcePage`, which a listing answers and the Rust service module
/// declares.
fn page_types(service: &ServiceDef) -> String {
    if !service
        .operations
        .iter()
        .any(|operation| operation.mcp.list_resources)
    {
        return String::new();
    }
    "/// One resource of a listing, as `resources/list` writes it.
class McpResource {
  McpResource({required this.uri, required this.name, this.title, this.description, this.mimeType, this.size});

  factory McpResource.fromJson(Map<String, dynamic> json) => McpResource(
        uri: json['uri'] as String,
        name: json['name'] as String,
        title: json['title'] as String?,
        description: json['description'] as String?,
        mimeType: json['mimeType'] as String?,
        size: json['size'] as int?,
      );

  final String uri;
  final String name;
  final String? title;
  final String? description;
  final String? mimeType;
  final int? size;

  Map<String, dynamic> toJson() => <String, dynamic>{
        'uri': uri,
        'name': name,
        if (title != null) 'title': title,
        if (description != null) 'description': description,
        if (mimeType != null) 'mimeType': mimeType,
        if (size != null) 'size': size,
      };
}

/// One page of `resources/list`, with the cursor of the next where there is one.
class McpResourcePage {
  McpResourcePage({required this.resources, this.nextCursor});

  factory McpResourcePage.fromJson(Map<String, dynamic> json) => McpResourcePage(
        resources: [
          for (final resource in json['resources'] as List<dynamic>)
            McpResource.fromJson(resource as Map<String, dynamic>),
        ],
        nextCursor: json['nextCursor'] as String?,
      );

  final List<McpResource> resources;
  final String? nextCursor;

  Map<String, dynamic> toJson() => <String, dynamic>{
        'resources': [for (final resource in resources) resource.toJson()],
        if (nextCursor != null) 'nextCursor': nextCursor,
      };
}"
    .to_owned()
}

/// The pieces of the resource requests, one function for each operation that answers one.
fn resource_functions(
    service: &ServiceDef,
    held: &Resourced<'_>,
    named: &str,
    prefix: &str,
) -> Vec<String> {
    if !held.any() {
        return Vec::new();
    }
    let fields = fault_fields_typescript_name(named);
    let mut published = vec![filled(UNREAD, named, prefix).replace("@F@", &fields)];
    if held.reads().next().is_some() {
        published.push(filled(CAPTURED, named, prefix));
        published.extend(
            held.reads()
                .map(|operation| resource_read(named, prefix, operation)),
        );
        published.push(read_dispatch(held, named, prefix));
    }
    if let Some(operation) = held.listing {
        published.push(resource_listing(named, prefix, operation));
    }
    published.extend(
        held.completions
            .iter()
            .filter_map(|operation| resource_completion(named, prefix, operation)),
    );
    if !held.completions.is_empty() {
        published.push(complete_dispatch(service, held, named, prefix));
    }
    published
}

/// The statements that read the message of `operation` from `source`, answering the request with
/// a fault where it does not read, and the call of its handler, ending in `success`, the declared
/// error `refusal` or a panic.
fn resource_body(
    named: &str,
    prefix: &str,
    operation: &OperationDef,
    source: &str,
    refusal: &str,
    answer: &str,
) -> String {
    let OperationOutcome::Reply { error, success: ty } = &operation.outcome else {
        return String::new();
    };
    let wire = quoted(&operation.wire_name);
    let call = dart_call(operation);
    let req_ty = message_dart_typename(operation);
    let decode = message_decode(operation, "source");
    let unjudged = unjudged_decode(operation, "source");
    let kind = format!("{named}FaultKind");
    let error_ty = dart_type_of(error);
    let declared = dart_json_encode(error, "declared", true);
    let encoded = dart_json_encode(ty, "answered", true);
    let success = answer.replace("@ENCODED@", &encoded);
    format!(
        "  final dynamic source = {source};\n  \
         final {req_ty} decoded;\n  \
         try {{\n    \
         decoded = {decode};\n  \
         }} on FormatException catch (rejected) {{\n    \
         try {{\n      \
         {unjudged};\n    \
         }} on FormatException catch (unread) {{\n      \
         return _{prefix}McpUnread(_{prefix}McpFault({kind}.undeserializablePayload, {wire}, unread.message));\n    \
         }} catch (unread) {{\n      \
         return _{prefix}McpUnread(_{prefix}McpFault({kind}.undeserializablePayload, {wire}, '$unread'));\n    \
         }}\n    \
         return _{prefix}McpUnread(\n      \
         _{prefix}McpFault({kind}.failedValidation, {wire}, rejected.message, field: _{prefix}McpField(rejected.message)),\n    \
         );\n  \
         }} catch (rejected) {{\n    \
         return _{prefix}McpUnread(_{prefix}McpFault({kind}.undeserializablePayload, {wire}, '$rejected'));\n  \
         }}\n  \
         try {{\n    \
         final answered = await handlers.{call}(ctx, decoded);\n    \
         {success}\n  \
         }} on {error_ty} catch (declared) {{\n    \
         return _{prefix}McpRefused({declared}, {refusal});\n  \
         }} catch (unexpected) {{\n    \
         return _{prefix}McpPanic({wire}, {wire}, '$unexpected');\n  \
         }}\n\
         }}"
    )
}

fn resource_read(named: &str, prefix: &str, operation: &OperationDef) -> String {
    let Some(resource) = operation.mcp.resource.as_ref() else {
        return String::new();
    };
    let slots = resource.placeholder_names();
    let uses_captured = !slots.is_empty() && !matches!(operation.inputs, OperationInputs::Empty);
    let source = match &operation.inputs {
        OperationInputs::Empty => "<String, dynamic>{}".to_owned(),
        OperationInputs::Named(_) if slots.is_empty() => "null".to_owned(),
        OperationInputs::Named(_) => "captured[0]".to_owned(),
        OperationInputs::Generated(fields) => {
            let members = slots
                .iter()
                .enumerate()
                .filter_map(|(at, name)| {
                    let (field, _) = fields.iter().find(|(field, _)| written(field) == *name)?;
                    Some(format!("{}: captured[{at}]", quoted(&wire_key(field))))
                })
                .collect::<Vec<_>>()
                .join(", ");
            format!("<String, dynamic>{{{members}}}")
        }
    };
    let ttl = operation.mcp.ttl_ms.unwrap_or(0);
    let scope = operation.mcp.cache_scope.as_deref().unwrap_or("private");
    let mime = operation
        .mcp
        .mime_type
        .as_ref()
        .map(|mime| format!("'mimeType': {}, ", quoted(mime)))
        .unwrap_or_default();
    let blob = matches!(
        &operation.outcome,
        OperationOutcome::Reply { success, error: _error } if matches!(success.as_ref(), syn::Type::Tuple(_))
    );
    let content = if blob {
        "<String, dynamic>{'uri': uri, 'mimeType': answered.$2, 'blob': base64Encode(answered.$1)}"
            .to_owned()
    } else {
        format!("<String, dynamic>{{'uri': uri, {mime}'text': @ENCODED@}}")
    };
    let success = format!(
        "return (\n      result: <String, dynamic>{{\n        \
         'cacheScope': {scope},\n        \
         'contents': <Object>[{content}],\n        \
         'resultType': 'complete',\n        \
         'ttlMs': {ttl},\n      }},\n      error: null,\n    );",
        scope = quoted(scope),
    );
    let captured = if uses_captured {
        ", List<String> captured"
    } else {
        ""
    };
    let head = format!(
        "Future<{named}McpAnswer> _{prefix}McpRead{op}<Ctx>(\n  \
         {named}Handlers<Ctx> handlers,\n  \
         Ctx ctx,\n  \
         String uri{captured},\n\
         ) async {{\n",
        op = pascal(operation),
    );
    format!(
        "{head}{}",
        resource_body(
            named,
            prefix,
            operation,
            &source,
            "'Resource not found'",
            &success,
        )
    )
}

fn read_dispatch(held: &Resourced<'_>, named: &str, prefix: &str) -> String {
    let mut text = format!(
        "Future<{named}McpAnswer?> _{prefix}McpRead<Ctx>({named}Handlers<Ctx> handlers, Ctx ctx, String uri) async {{\n"
    );
    for operation in held.reads() {
        let Some(resource) = operation.mcp.resource.as_ref() else {
            continue;
        };
        let segments = if resource.is_template() {
            resource
                .segments
                .iter()
                .map(|segment| match segment {
                    PathSegment::Literal(literal) => format!("({}, false)", quoted(literal)),
                    PathSegment::Placeholder(name) => format!("({}, true)", quoted(name)),
                })
                .collect::<Vec<_>>()
                .join(", ")
        } else {
            format!("({}, false)", quoted(&resource.uri))
        };
        let slots = resource.placeholder_names();
        let uses_captured =
            !slots.is_empty() && !matches!(operation.inputs, OperationInputs::Empty);
        let arguments = if uses_captured {
            "handlers, ctx, uri, captured"
        } else {
            "handlers, ctx, uri"
        };
        let _ = write!(
            text,
            "  {{\n    final {local} = _{prefix}McpCaptured(<(String, bool)>[{segments}], uri);\n    \
             if ({local} != null) return _{prefix}McpRead{op}<Ctx>({arguments});\n  }}\n",
            local = if uses_captured { "captured" } else { "matched" },
            op = pascal(operation),
        );
    }
    text.push_str("  return null;\n}");
    text
}

fn resource_listing(named: &str, prefix: &str, operation: &OperationDef) -> String {
    let source = if matches!(operation.inputs, OperationInputs::Named(_)) {
        "cursor is String ? cursor : null"
    } else {
        "<String, dynamic>{}"
    };
    let ttl = operation.mcp.ttl_ms.unwrap_or(0);
    let scope = operation.mcp.cache_scope.as_deref().unwrap_or("private");
    let success = format!(
        "return (\n      result: <String, dynamic>{{\n        \
         ...(@ENCODED@),\n        \
         'cacheScope': {scope},\n        \
         'resultType': 'complete',\n        \
         'ttlMs': {ttl},\n      }},\n      error: null,\n    );",
        scope = quoted(scope),
    );
    let head = format!(
        "Future<{named}McpAnswer> _{prefix}McpList<Ctx>(\n  \
         {named}Handlers<Ctx> handlers,\n  \
         Ctx ctx,\n  \
         Object? cursor,\n\
         ) async {{\n"
    );
    format!(
        "{head}{}",
        resource_body(
            named,
            prefix,
            operation,
            source,
            "'Resources could not be listed'",
            &success,
        )
    )
}

fn resource_completion(named: &str, prefix: &str, operation: &OperationDef) -> Option<String> {
    let OperationInputs::Generated(fields) = &operation.inputs else {
        return None;
    };
    let typed_key = quoted(&wire_key(&fields.first()?.0));
    let chosen_key = quoted(&wire_key(&fields.get(1)?.0));
    let source = format!("<String, dynamic>{{{typed_key}: typed, {chosen_key}: chosen}}");
    let success = "final suggested = <Object>[...answered];\n    \
         return (\n      result: <String, dynamic>{\n        \
         'completion': <String, dynamic>{\n          \
         'hasMore': suggested.length > 100,\n          \
         if (suggested.length > 100) 'total': suggested.length,\n          \
         'values': suggested.take(100).toList(),\n        },\n        \
         'resultType': 'complete',\n      },\n      error: null,\n    );"
        .to_owned();
    let head = format!(
        "Future<{named}McpAnswer> _{prefix}McpComplete{op}<Ctx>(\n  \
         {named}Handlers<Ctx> handlers,\n  \
         Ctx ctx,\n  \
         String typed,\n  \
         Object? chosen,\n\
         ) async {{\n",
        op = pascal(operation),
    );
    Some(format!(
        "{head}{}",
        resource_body(
            named,
            prefix,
            operation,
            &source,
            "'Completion refused'",
            &success,
        )
    ))
}

fn complete_dispatch(
    service: &ServiceDef,
    held: &Resourced<'_>,
    named: &str,
    prefix: &str,
) -> String {
    let mut text = format!(
        "Future<{named}McpAnswer?> _{prefix}McpComplete<Ctx>(\n  \
         {named}Handlers<Ctx> handlers,\n  \
         Ctx ctx,\n  \
         Map<String, dynamic> params,\n\
         ) async {{\n  \
         final reference = params['ref'];\n  \
         final argument = params['argument'];\n  \
         if (reference is! Map || reference['type'] != 'ref/resource') return null;\n  \
         final uri = reference['uri'];\n  \
         final parameter = argument is Map ? argument['name'] : null;\n  \
         if (uri is! String || parameter is! String) return null;\n  \
         final typed = argument is Map && argument['value'] is String ? argument['value'] as String : '';\n  \
         final context = params['context'];\n  \
         final chosen = context is Map && context.containsKey('arguments') ? context['arguments'] : <String, dynamic>{{}};\n"
    );
    for operation in &held.completions {
        let Some(completes) = operation.mcp.completes.as_ref() else {
            continue;
        };
        let Some(target) = service
            .operations
            .iter()
            .find(|candidate| written(&candidate.ident) == completes.resource)
        else {
            continue;
        };
        let Some(resource) = target.mcp.resource.as_ref() else {
            continue;
        };
        let _ = writeln!(
            text,
            "  if (uri == {} && parameter == {}) return _{prefix}McpComplete{}<Ctx>(handlers, ctx, typed, chosen);",
            quoted(&resource.uri),
            quoted(&completes.parameter),
            pascal(operation),
        );
    }
    text.push_str("  return null;\n}");
    text
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
    if tools_of(service).next().is_none() {
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
    let methods = tools_of(service)
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
            let resources = Self::mcp_resources()
                .iter()
                .map(|resource| literal(&resource.to_string()))
                .collect::<::std::vec::Vec<_>>()
                .join(", ");
            let templates = Self::mcp_resource_templates()
                .iter()
                .map(|template| literal(&template.to_string()))
                .collect::<::std::vec::Vec<_>>()
                .join(", ");
            let instructions = match Self::MCP_INSTRUCTIONS {
                ::core::option::Option::Some(text) => literal(text),
                ::core::option::Option::None => ::std::string::String::from("null"),
            };
            #server
                .replace(#TOOLS_MARKER, &tools)
                .replace(#NAMES_MARKER, &names)
                .replace(#RESOURCES_MARKER, &resources)
                .replace(#TEMPLATES_MARKER, &templates)
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
