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
use crate::service_schema::mcp_tools::{
    answers_key, has_tasks, is_content_list, is_content_pair, is_prompt_messages, read_apart,
};
use crate::service_schema::parse::{
    HttpShape, McpCompletesKind, McpResourceUri, OperationDef, OperationInputs, OperationOutcome,
    PathSegment, ServiceDef, is_unit_type, option_inner, wire_key, written,
};
use crate::service_schema::skills::written_frontmatter;
use crate::service_schema::support::{fault_fields_typescript_name, fault_issue_name};
use crate::service_schema::transport::mcp::resources::Resourced;
use core::fmt::Write as _;
use proc_macro2::TokenStream;
use quote::{format_ident, quote};

/// The type an `mcp(content)` operation answers a list of, as the service module declares it in
/// Rust: one block of a tool's `content`, read and written as the specification writes it.
const CONTENT_TYPE: &str = "/// One block of a tool's `content`: what an `mcp(content)` operation answers a list of.
sealed class McpContent {
  const McpContent();

  /// Reads one block as the specification writes it.
  factory McpContent.fromJson(Object? json) {
    if (json is! Map<String, dynamic>) throw const FormatException('a block is not an object');
    String text(Map<String, dynamic> from, String key) {
      final held = from[key];
      if (held is! String) throw FormatException('a block holds no `$key`');
      return held;
    }

    switch (json['type']) {
      case 'text':
        return McpContentText(text(json, 'text'));
      case 'image':
        return McpContentImage(base64Decode(text(json, 'data')), text(json, 'mimeType'));
      case 'audio':
        return McpContentAudio(base64Decode(text(json, 'data')), text(json, 'mimeType'));
      case 'resource_link':
        return McpContentResourceLink(
          uri: text(json, 'uri'),
          name: text(json, 'name'),
          title: json['title'] as String?,
          description: json['description'] as String?,
          mimeType: json['mimeType'] as String?,
        );
      case 'resource':
        final resource = json['resource'];
        if (resource is! Map<String, dynamic>) throw const FormatException('a resource block holds no resource');
        final blob = resource['blob'];
        return McpContentResource(
          uri: text(resource, 'uri'),
          mimeType: resource['mimeType'] as String?,
          contents: blob is String ? McpResourceContentsBlob(base64Decode(blob)) : McpResourceContentsText(text(resource, 'text')),
        );
      default:
        throw const FormatException('a block of a type this client does not read');
    }
  }

  /// This block as the specification writes it.
  Map<String, dynamic> toJson();
}

/// Text.
final class McpContentText extends McpContent {
  const McpContentText(this.text);
  final String text;

  @override
  Map<String, dynamic> toJson() => <String, dynamic>{'text': text, 'type': 'text'};
}

/// An image: its bytes and its content type.
final class McpContentImage extends McpContent {
  const McpContentImage(this.data, this.mimeType);
  final Uint8List data;
  final String mimeType;

  @override
  Map<String, dynamic> toJson() =>
      <String, dynamic>{'data': base64Encode(data), 'mimeType': mimeType, 'type': 'image'};
}

/// Audio: its bytes and its content type.
final class McpContentAudio extends McpContent {
  const McpContentAudio(this.data, this.mimeType);
  final Uint8List data;
  final String mimeType;

  @override
  Map<String, dynamic> toJson() =>
      <String, dynamic>{'data': base64Encode(data), 'mimeType': mimeType, 'type': 'audio'};
}

/// A link to a resource the client can read.
final class McpContentResourceLink extends McpContent {
  const McpContentResourceLink({required this.uri, required this.name, this.title, this.description, this.mimeType});
  final String uri;
  final String name;
  final String? title;
  final String? description;
  final String? mimeType;

  @override
  Map<String, dynamic> toJson() => <String, dynamic>{
        'name': name,
        'type': 'resource_link',
        'uri': uri,
        if (title != null) 'title': title,
        if (description != null) 'description': description,
        if (mimeType != null) 'mimeType': mimeType,
      };
}

/// A resource carried in the result.
final class McpContentResource extends McpContent {
  const McpContentResource({required this.uri, this.mimeType, required this.contents});
  final String uri;
  final String? mimeType;
  final McpResourceContents contents;

  @override
  Map<String, dynamic> toJson() => <String, dynamic>{
        'resource': <String, dynamic>{
          'uri': uri,
          if (mimeType != null) 'mimeType': mimeType,
          ...switch (contents) {
            McpResourceContentsText(:final text) => <String, dynamic>{'text': text},
            McpResourceContentsBlob(:final bytes) => <String, dynamic>{'blob': base64Encode(bytes)},
          },
        },
        'type': 'resource',
      };
}

/// What an embedded resource holds.
sealed class McpResourceContents {
  const McpResourceContents();
}

/// Text.
final class McpResourceContentsText extends McpResourceContents {
  const McpResourceContentsText(this.text);
  final String text;
}

/// Bytes.
final class McpResourceContentsBlob extends McpResourceContents {
  const McpResourceContentsBlob(this.bytes);
  final Uint8List bytes;
}

/// Who a prompt's message is from.
enum McpRole { user, assistant }

/// One message of a prompt: who it is from, and the block it holds.
final class McpPromptMessage {
  const McpPromptMessage({required this.role, required this.content});

  /// Reads one message as the specification writes it.
  factory McpPromptMessage.fromJson(Object? json) {
    if (json is! Map<String, dynamic>) throw const FormatException('a message is not an object');
    final role = switch (json['role']) {
      'user' => McpRole.user,
      'assistant' => McpRole.assistant,
      _ => throw const FormatException('a message of a role this client does not read'),
    };
    return McpPromptMessage(role: role, content: McpContent.fromJson(json['content']));
  }

  final McpRole role;
  final McpContent content;

  /// This message as the specification writes it.
  Map<String, dynamic> toJson() => <String, dynamic>{'content': content.toJson(), 'role': role.name};
}";

/// The types an `mcp(task(...))` operation answers, as the service module declares them in Rust:
/// the handle of a task, and where a task stands.
const TASK_TYPE: &str = "/// The handle of a task: the id the application keeps it under, and when it started.
class McpTask {
  const McpTask({
    required this.taskId,
    required this.createdAt,
    required this.lastUpdatedAt,
    this.ttlMs,
    this.pollIntervalMs,
  });

  /// Reads a handle as the specification writes it.
  factory McpTask.fromJson(Object? json) {
    if (json is! Map<String, dynamic>) throw const FormatException('a task is not an object');
    String text(String key) {
      final held = json[key];
      if (held is! String) throw FormatException('a task holds no `$key`');
      return held;
    }

    return McpTask(
      taskId: text('taskId'),
      createdAt: text('createdAt'),
      lastUpdatedAt: text('lastUpdatedAt'),
      ttlMs: json['ttlMs'] as int?,
      pollIntervalMs: json['pollIntervalMs'] as int?,
    );
  }

  final String taskId;
  final String createdAt;
  final String lastUpdatedAt;
  final int? ttlMs;
  final int? pollIntervalMs;

  /// The handle as it travels, `working` being where every task starts.
  Map<String, dynamic> toJson() => <String, dynamic>{
        'taskId': taskId,
        'status': 'working',
        'createdAt': createdAt,
        'lastUpdatedAt': lastUpdatedAt,
        'ttlMs': ttlMs,
        if (pollIntervalMs != null) 'pollIntervalMs': pollIntervalMs,
      };
}

/// Why a task failed, as the specification writes an error.
class McpTaskError {
  const McpTaskError({required this.code, required this.message, this.data});

  /// Reads an error as the specification writes it.
  factory McpTaskError.fromJson(Object? json) {
    if (json is! Map<String, dynamic>) throw const FormatException('an error is not an object');
    final code = json['code'];
    final message = json['message'];
    if (code is! int || message is! String) throw const FormatException('an error holds no `code` or `message`');
    return McpTaskError(code: code, message: message, data: json['data']);
  }

  final int code;
  final String message;
  final Object? data;

  Map<String, dynamic> toJson() => <String, dynamic>{
        'code': code,
        'message': message,
        if (data != null) 'data': data,
      };
}

/// Where a task stands: what `tasks/get` and the operation named in `task(get = ...)` answer.
sealed class McpTaskState<T> {
  const McpTaskState(this.task);

  /// Reads where a task stands as the specification writes it; `readResult` reads the result of a
  /// completed task.
  factory McpTaskState.fromJson(Object? json, T Function(dynamic) readResult) {
    final task = McpTask.fromJson(json);
    final members = json as Map<String, dynamic>;
    switch (members['status']) {
      case 'working':
        return McpTaskWorking<T>(task);
      case 'input_required':
        final requests = members['inputRequests'];
        if (requests is! Map<String, dynamic>) throw const FormatException('a task holds no `inputRequests`');
        return McpTaskInputRequired<T>(task, requests);
      case 'completed':
        return McpTaskCompleted<T>(task, readResult(members['result']));
      case 'failed':
        return McpTaskFailed<T>(task, McpTaskError.fromJson(members['error']));
      case 'cancelled':
        return McpTaskCancelled<T>(task);
      default:
        throw FormatException('a task of a status this client does not read: ${members['status']}');
    }
  }

  final McpTask task;

  /// The state as it travels; `encodeResult` writes the result of a completed task.
  Map<String, dynamic> toJson(Object? Function(T) encodeResult);
}

/// The work goes on.
class McpTaskWorking<T> extends McpTaskState<T> {
  const McpTaskWorking(super.task);

  @override
  Map<String, dynamic> toJson(Object? Function(T) encodeResult) =>
      <String, dynamic>{...this.task.toJson(), 'status': 'working'};
}

/// The work waits for the user: each request is an `elicitation/create` request.
class McpTaskInputRequired<T> extends McpTaskState<T> {
  const McpTaskInputRequired(super.task, this.inputRequests);
  final Map<String, dynamic> inputRequests;

  @override
  Map<String, dynamic> toJson(Object? Function(T) encodeResult) =>
      <String, dynamic>{...this.task.toJson(), 'status': 'input_required', 'inputRequests': inputRequests};
}

/// The work ended: `result` is what the starting tool would have answered.
class McpTaskCompleted<T> extends McpTaskState<T> {
  const McpTaskCompleted(super.task, this.result);
  final T result;

  @override
  Map<String, dynamic> toJson(Object? Function(T) encodeResult) =>
      <String, dynamic>{...this.task.toJson(), 'status': 'completed', 'result': encodeResult(result)};
}

/// The work failed.
class McpTaskFailed<T> extends McpTaskState<T> {
  const McpTaskFailed(super.task, this.error);
  final McpTaskError error;

  @override
  Map<String, dynamic> toJson(Object? Function(T) encodeResult) =>
      <String, dynamic>{...this.task.toJson(), 'status': 'failed', 'error': error.toJson()};
}

/// The work was cancelled.
class McpTaskCancelled<T> extends McpTaskState<T> {
  const McpTaskCancelled(super.task);

  @override
  Map<String, dynamic> toJson(Object? Function(T) encodeResult) =>
      <String, dynamic>{...this.task.toJson(), 'status': 'cancelled'};
}";

const TOOLS_MARKER: &str = "@TOOLS@";
const NAMES_MARKER: &str = "@NAMES@";
const INSTRUCTIONS_MARKER: &str = "@INSTRUCTIONS@";
const VERSION_MARKER: &str = "@VERSION@";
const RESOURCES_MARKER: &str = "@RESOURCES@";
const TEMPLATES_MARKER: &str = "@TEMPLATES@";
const PROMPTS_MARKER: &str = "@PROMPTS@";
const READS_MARKER: &str = "@READS@";
const SKILLS_MARKER: &str = "@SKILLS@";
const SKILL_FILES_MARKER: &str = "@SKILLFILES@";

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
  prompts: <String>[@PROMPTS@],
  skills: <String>[@SKILLS@],
  capabilities: <String>[@CAPABILITIES@],
);

@SKILLFILESDECL@
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

/// Where an HTTP server writes an event-stream answer. `begin` is called once, before the first
/// event, with the status and headers of the answer; `write` then takes the text of each event, or
/// of the comment line that keeps the stream open. A call that throws means the client has gone. The
/// host ends its response when the server answers `null`.
abstract interface class @N@McpHttpStream {
  Future<void> begin(int status, List<(String, String)> headers);
  Future<void> write(String event);
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

/// One task moved: where it stands now, as `McpTaskState` writes it.
class @N@McpTaskMoved extends @N@McpChange {
  const @N@McpTaskMoved(this.taskId, this.state);
  final String taskId;
  final Map<String, dynamic> state;
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

/// What `admit` answers: the context the handlers run with in `ctx` and, where the caller decides it, the name the server answers as in `serverName`, with every public answer of the request made private, or why the request is refused in `denied`.
typedef @N@McpAdmit<Ctx> = Future<({Ctx? ctx, @N@McpDenied? denied, String? serverName})> Function(
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

/// A task state as a task method and a notification write it: under the full task id, with a
/// completed task's result as the `CallToolResult` the starting tool would have answered.
Map<String, dynamic> _@P@McpTaskView(String taskId, Map<String, dynamic> state) {
  final view = Map<String, dynamic>.of(state)..['taskId'] = taskId;
  if (view.containsKey('result')) {
    view['result'] = <String, dynamic>{
      'content': <Object>[<String, dynamic>{'text': _@P@McpJson(view['result']), 'type': 'text'}],
      'isError': false,
      'resultType': 'complete',
      'structuredContent': view['result'],
    };
  }
  return view;
}

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
  final tasks = filter['taskIds'];
  if (filter.containsKey('taskIds') && !(tasks is List && tasks.every((task) => task is String))) {
    return invalid('`taskIds` is not an array of strings');
  }
  final agreed = <String, dynamic>{};
  final offersResources = endpoint.catalogs.any(
    (catalog) => catalog.capabilities.any((written) => (jsonDecode(written) as Map).containsKey('resources')),
  );
  if (offersResources) {
    if (filter['resourcesListChanged'] == true) agreed['resourcesListChanged'] = true;
    if (uris is List && uris.isNotEmpty) agreed['resourceSubscriptions'] = uris;
  }
  if (_@P@McpTasked(endpoint) && tasks is List && tasks.isNotEmpty) agreed['taskIds'] = tasks;
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
  final followed = <String>{
    if (agreed['taskIds'] case final List<dynamic> held) ...held.cast<String>(),
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
      @N@McpTaskMoved(:final taskId, :final state) when followed.contains(taskId) =>
        await call._write(notified('notifications/tasks', _@P@McpTaskView(taskId, state))),
      @N@McpKeepAlive() => await call._keepAlive(),
      _ => true,
    };
    if (!written) return ended;
  }
}

/// Whether any service of the endpoint offers the tasks extension.
bool _@P@McpTasked(@N@McpEndpoint endpoint) => endpoint.catalogs.any((catalog) => catalog.capabilities.any((written) {
      final extensions = (jsonDecode(written) as Map<String, dynamic>)['extensions'];
      return extensions is Map && extensions.containsKey('io.modelcontextprotocol/tasks');
    }));

Future<({String? text, int status})> _@P@McpRespond(
  String text,
  String serverName,
  bool callerNamed,
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
      'cacheScope': callerNamed ? '"private"' : '"public"',
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
  var scope = callerNamed ? 'private' : 'public';
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
      if (listed.any((text) => (jsonDecode(text) as Map<String, dynamic>)['resources'] == 'dynamic')) {
        scope = 'private';
        ttl = 0;
      }
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
        if (result['cacheScope'] is String) scope = callerNamed && result['cacheScope'] == 'public' ? 'private' : result['cacheScope'] as String;
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
    'skills/get',
  };
  final taskMethod = method == 'tasks/get' || method == 'tasks/cancel' || method == 'tasks/update';
  if (!(routed.contains(method) || (taskMethod && _@P@McpTasked(endpoint)))) {
    return failed(id, 404, -32601, 'Method not found: $method');
  }
  final String? named;
  switch (method) {
    case 'tools/call':
      named = params['name'] is String ? params['name'] as String : null;
    case 'resources/read':
      named = params['uri'] is String ? params['uri'] as String : null;
    case 'prompts/get':
      named = params['name'] is String ? params['name'] as String : null;
    case 'skills/get':
      named = params['name'] is String
          ? params['name'] as String
          : params['uri'] is String
              ? params['uri'] as String
              : null;
    case 'tasks/get' || 'tasks/cancel' || 'tasks/update':
      named = params['taskId'] is String ? params['taskId'] as String : null;
    case 'completion/complete':
      final argument = params['argument'];
      final reference = params['ref'];
      final held = reference is Map ? (reference['type'] == 'ref/prompt' ? reference['name'] : reference['uri']) : null;
      named = argument is Map && argument['name'] is String && held is String ? held : null;
    default:
      named = null;
  }
  if (named == null &&
      (method == 'tools/call' ||
          method == 'resources/read' ||
          method == 'prompts/get' ||
          method == 'completion/complete' ||
          taskMethod ||
          method == 'skills/get')) {
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
    if (callerNamed && members['cacheScope'] == 'public') members['cacheScope'] = 'private';
    return answered(_@P@McpJson(members));
  }
  final asked = named ?? params['name'] ?? params['uri'] ?? method;
  final unknown = switch (method) {
    'tools/call' => 'Unknown tool',
    'resources/read' => 'Resource not found',
    'prompts/get' => 'Unknown prompt',
    'skills/get' => 'Skill not found',
    'completion/complete' => 'Unknown completion',
    'tasks/get' || 'tasks/cancel' || 'tasks/update' => 'Unknown task',
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
    final answered = await _@P@McpRespond(text, serverName, false, @P@McpServerVersion, instructions, endpoint,
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

const _@P@McpEventHeaders = [('Content-Type', 'text/event-stream'), ('X-Accel-Buffering', 'no')];

/// One event of an event stream, carrying one JSON-RPC message.
String _@P@McpFrame(String message) => 'event: message\ndata: $message\n\n';

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
/// `admit` answered. A routed request that carries a `progressToken` answers an event stream written
/// to `stream` as it happens (`begin`, each notification, then the answer) and the server answers
/// `null`. So does a `subscriptions/listen` where `changes` is handed in: it ends when `changes`
/// answers `null`. Any other request answers the response, and `stream` is never begun.
Future<@N@McpHttpResponse?> @P@McpServeHttp<Ctx>(
  @N@McpHttpRequest request,
  @N@McpHttpSettings settings,
  @N@McpHttpStream stream,
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
  final held = <String>[];
  var live = false;
  Future<void> give(String text) async {
    if (live) return stream.write(text);
    held.add(text);
  }

  final call = @N@McpCall(
    token: _@P@McpBodyToken(request),
    writer: (event) => give(_@P@McpFrame(event)),
    comment: () => give(':\n\n'),
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
  final streams = _@P@McpToken(parsed) != null || method == 'subscriptions/listen';
  Future<void>? beginning;
  Future<void> begin() {
    if (!streams) return Future<void>.value();
    return beginning ??= () async {
      try {
        await stream.begin(200, _@P@McpEventHeaders);
        live = true;
        for (final text in held.toList()) {
          await stream.write(text);
        }
        held.clear();
      } catch (_) {
        call._cancel();
      }
    }();
  }

  var routed = false;
  final answered = await _@P@McpRespond(utf8.decode(request.body), admitted.serverName ?? serverName, admitted.serverName != null, @P@McpServerVersion, instructions,
      endpoint, (part, method, params) async {
    routed = true;
    await begin();
    return call.cancelled && streams ? null : route(ctx, part, method, params);
  }, changes == null
      ? null
      : (id, params, info) async {
          routed = true;
          await begin();
          return _@P@McpListen(id, params, info, endpoint, call, changes);
        });
  if (!routed) return respond(answered.status, answered.text);
  final written = call.cancelled ? null : answered.text;
  if (!streams) return written == null ? respond(202) : respond(answered.status, written);
  if (written != null) {
    try {
      await stream.write(_@P@McpFrame(written));
    } catch (_) {
      call._cancel();
    }
  }
  return null;
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

const CLIENT_PAIR: &str =
    "/// The bytes and the content type an answer's one image or audio block holds, written as the
/// structured pair, `null` where it holds none.
Object? _@P@McpPair(Object? content) {
  final block = content is List && content.isNotEmpty ? content.first : null;
  if (block is! Map<String, dynamic>) return null;
  final data = block['data'];
  final type = block['mimeType'];
  if (data is! String || type is! String) return null;
  try {
    return <Object>[base64Decode(data), type];
  } catch (_) {
    return null;
  }
}";

const CLIENT_CALL_METHOD: &str = "  int _nextId = 1;

  Future<Map<String, dynamic>> _call(
    String tool,
    String operation,
    Map<String, dynamic> arguments, {
    String method = 'tools/call',
  }) async {
    final text = jsonEncode(<String, dynamic>{
      'id': _nextId++,
      'jsonrpc': '2.0',
      'method': method,
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
        (text: text, protocolVersion: '2026-07-28', method: method, name: tool),
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
      if (method == 'prompts/get' && error['code'] == -32602 && data != null) {
        return <String, dynamic>{
          'content': <Object>[<String, dynamic>{'text': jsonEncode(data), 'type': 'text'}],
          'isError': true,
        };
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

const CLIENT_PROMPT_METHOD: &str = "  Future<Map<String, dynamic>> _prompt(
    String prompt,
    String operation,
    Map<String, dynamic> arguments,
    bool text,
  ) async {
    final result = await _call(prompt, operation, arguments, method: 'prompts/get');
    if (result['isError'] == true) return result;
    final messages = result['messages'];
    if (!text) return <String, dynamic>{'isError': false, 'structuredContent': messages};
    final first = messages is List && messages.isNotEmpty ? messages.first : null;
    final content = first is Map<String, dynamic> ? first['content'] : null;
    final held = content is Map<String, dynamic> ? content['text'] : null;
    if (held is! String) {
      throw _@N@McpCallFault(_@P@McpReadFailed(operation, 'the first message holds no text'));
    }
    return <String, dynamic>{'isError': false, 'structuredContent': held};
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
  @N@Handlers<Ctx> handlers, [
  void Function(@N@FaultFields fault)? onFault,
]) {
  final route = create@N@McpRoute(handlers, onFault);
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
/// it is refused. An event-stream answer is written to the stream it is handed and answers `null`.
Future<@N@McpHttpResponse?> Function(@N@McpHttpRequest, @N@McpHttpStream) create@N@McpHttpDispatcher<Ctx>(
  @N@Handlers<Ctx> handlers,
  @N@McpHttpSettings settings,
  @N@McpAdmit<Ctx> admit, [
  @N@McpChanges? changes,
  void Function(@N@FaultFields fault)? onFault,
]) {
  final route = create@N@McpRoute(handlers, onFault);
  return (request, stream) => @P@McpServeHttp<Ctx>(
        request,
        settings,
        stream,
        @P@McpServerName,
        @P@McpInstructions,
        _@P@McpAlone,
        admit,
        (ctx, part, method, params) => route(ctx, method, params),
        changes,
      );
}";

const INTERVIEW_HELPERS: &str = "/// Whether the client declared the elicitation `mode`: an empty `elicitation` is a form.
bool _@P@McpElicits(Map<String, dynamic> params, String mode) {
  final meta = params['_meta'];
  final capabilities = meta is Map ? meta['io.modelcontextprotocol/clientCapabilities'] : null;
  final held = capabilities is Map ? capabilities['elicitation'] : null;
  if (held is! Map) return false;
  if (mode == 'url') return held['url'] is Map;
  return held['form'] is Map || !held.containsKey('url');
}

/// Whether the user declined or dismissed the request keyed `key`.
bool _@P@McpDismissed(Map<String, dynamic> params, String key) {
  final responses = params['inputResponses'];
  final response = responses is Map ? responses[key] : null;
  final action = response is Map ? response['action'] : null;
  return action == 'decline' || action == 'cancel';
}

/// The result that asks the client for the elicitation `request`, keyed `key`.
@N@McpAnswer _@P@McpInputRequired(String key, Map<String, dynamic> request) => (
      result: <String, dynamic>{
        'inputRequests': <String, dynamic>{
          key: <String, dynamic>{'method': 'elicitation/create', 'params': request},
        },
        'resultType': 'input_required',
      },
      error: null,
    );

/// The result that asks the user where `error` is the arm that asks and the client can be asked,
/// `null` otherwise.
@N@McpAnswer? _@P@McpAsking(
  String tag,
  String wire,
  String content,
  String? answers,
  String? form,
  String doc,
  Object? error,
  Map<String, dynamic> params,
  bool declined,
) {
  if (error is! Map<String, dynamic>) return null;
  final Object? named = tag.isEmpty ? error[wire] : (error[tag] == wire ? error : null);
  if (named is! Map<String, dynamic>) return null;
  final Object? held = content.isEmpty ? named : named[content];
  if (held is! Map<String, dynamic>) return null;
  final message = held['message'];
  if (answers != null) {
    if (declined || !_@P@McpElicits(params, 'form')) return null;
    return _@P@McpInputRequired(answers, <String, dynamic>{
      'message': message is String ? message : doc,
      'mode': 'form',
      'requestedSchema': jsonDecode(form ?? '{}'),
    });
  }
  final url = held['url'];
  if (url is! String || message is! String) return null;
  if (_@P@McpDismissed(params, wire) || !_@P@McpElicits(params, 'url')) return null;
  return _@P@McpInputRequired(wire, <String, dynamic>{'message': message, 'mode': 'url', 'url': url});
}";

const CONTENT_HELPERS: &str = "/// The result of a tool that answers blocks.
@N@McpAnswer _@P@McpBlocks(List<Object> content) => (
      result: <String, dynamic>{'content': content, 'isError': false},
      error: null,
    );";

const MEDIA_HELPER: &str =
    "/// The image or audio block of `bytes`, where `contentType` says they are one.
@N@McpAnswer? _@P@McpMedia(List<int> bytes, String contentType) {
  final kind = contentType.startsWith('image/')
      ? 'image'
      : contentType.startsWith('audio/')
          ? 'audio'
          : null;
  if (kind == null) return null;
  return _@P@McpBlocks(<Object>[
    <String, dynamic>{'data': base64Encode(bytes), 'mimeType': contentType, 'type': kind},
  ]);
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
                        calls.ok |= !is_content_list(operation);
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

/// The schema a read's refused message is walked against, and whether every mismatch is listed
/// or the first named alone.
struct Walked<'text> {
    every: bool,
    schema: &'text str,
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
            .replace(
                "@SKILLFILESDECL@\n",
                &skill_files_declaration(service, &prefix),
            )
            .replace(
                "@CAPABILITIES@",
                &capabilities_literal(service, &held, skilled(service)),
            )
            .replace("@SCOPES@\n", &scopes_table(&ScopeTable::of(service))),
        wrapped_table(&prefix, "Server", service)
            .trim_end()
            .to_owned(),
    ];
    if !service.operations.is_empty() {
        published.push(answer_helpers(&named, &prefix, &calls, &held, service));
    }
    if service
        .operations
        .iter()
        .any(|operation| operation.mcp.interview.is_some())
    {
        published.push(filled(INTERVIEW_HELPERS, &named, &prefix));
    }
    if service
        .operations
        .iter()
        .any(|operation| operation.mcp.content.is_some())
    {
        published.push(filled(CONTENT_HELPERS, &named, &prefix));
    }
    if service.operations.iter().any(is_content_pair) {
        published.push(filled(MEDIA_HELPER, &named, &prefix));
    }
    published.extend(tools_of(service).map(|operation| server_call(&named, &prefix, operation)));
    published.extend(resource_functions(service, &held, &named, &prefix));
    if service
        .operations
        .iter()
        .any(|operation| operation.mcp.task.is_some())
    {
        published.push(task_helpers(service, &named, &prefix));
    }
    published.push(route_fn(service, &held, &named, &prefix));
    published.push(filled(SERVER_ALONE, &named, &prefix));
    published
        .into_iter()
        .filter(|text| !text.is_empty())
        .collect::<Vec<_>>()
        .join("\n\n")
}

/// The scopes `prompts/get` needs, by the name of the prompt.
fn prompt_scopes(table: &ScopeTable) -> String {
    if table.prompts.is_empty() {
        return String::new();
    }
    let mut text = String::from("  if (method == 'prompts/get') {\n    switch (name) {\n");
    for (prompt, scopes) in &table.prompts {
        let list = scopes
            .iter()
            .map(|scope| quoted(scope))
            .collect::<Vec<_>>()
            .join(", ");
        let _ = writeln!(
            text,
            "      case {}:\n        return const <String>[{list}];",
            quoted(prompt)
        );
    }
    text.push_str("    }\n  }\n");
    text
}

/// The scopes the task methods need, by the tool that started the task.
fn task_scopes(table: &ScopeTable) -> String {
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
    for (method, rows) in &table.tasks {
        let _ = writeln!(
            text,
            "  if (method == {} && name != null) {{\n    switch (name.split(':').first) {{",
            quoted(method)
        );
        for (tool, scopes) in rows {
            let _ = writeln!(
                text,
                "      case {}:\n        return {};",
                quoted(tool),
                list(scopes)
            );
        }
        text.push_str("    }\n  }\n");
    }
    text
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
    text.push_str(&prompt_scopes(table));
    text.push_str(&task_scopes(table));
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

/// The Dart that names the path of each value a message does not read at, from the JSON schema
/// the catalog holds for it: the first one, or with `every` all of them, a key of `passed` read
/// past at the top.
fn unread_helpers(named: &str, prefix: &str) -> String {
    let fields = fault_fields_typescript_name(named);
    let issue = fault_issue_name(named);
    format!(
        "{fields} _{prefix}McpUnreadFault(String operation, String detail, Object? schema, Object? source, \
         {{List<String> passed = const <String>[], bool every = false}}) {{\n  \
         final found = <(String, String)>[];\n  \
         _{prefix}McpMismatches(schema, source, null, passed, const <String>[], found);\n  \
         if (found.isEmpty || found.first.$1.isEmpty) {{\n    \
         return _{prefix}McpFault({named}FaultKind.failedValidation, operation, detail);\n  \
         }}\n  \
         final path = found.first.$1;\n  \
         final first = \"'$path': $detail\";\n  \
         if (!every || found.length < 2) {{\n    \
         return _{prefix}McpFault({named}FaultKind.failedValidation, operation, first, field: path);\n  \
         }}\n  \
         final lines = <String>[first];\n  \
         final issues = <{issue}>[{issue}(detail: detail, field: path)];\n  \
         for (final (at, said) in found.skip(1)) {{\n    \
         lines.add(at.isEmpty ? said : \"'$at': $said\");\n    \
         issues.add({issue}(detail: said, field: at.isEmpty ? null : at));\n  \
         }}\n  \
         return _{prefix}McpFault({named}FaultKind.failedValidation, operation, lines.join('; '), field: path, issues: issues);\n\
         }}\n\n\
         void _{prefix}McpMismatches(Object? schema, Object? value, Object? defs, List<String> passed, List<String> at, \
         List<(String, String)> found) {{\n  \
         if (schema is! Map) return;\n  \
         defs ??= schema['\\$defs'];\n  \
         void refuse(List<String> steps, String said) => found.add((steps.join('.'), said));\n  \
         final reference = schema['\\$ref'];\n  \
         if (reference is String) {{\n    \
         _{prefix}McpMismatches(defs is Map ? defs[reference.substring(reference.lastIndexOf('/') + 1)] : null, value, defs, passed, at, found);\n    \
         return;\n  \
         }}\n  \
         final alternatives = schema['anyOf'];\n  \
         if (alternatives is List) {{\n    \
         if (value == null) return;\n    \
         List<(String, String)>? named;\n    \
         for (final alternative in alternatives) {{\n      \
         if (alternative is Map && alternative['type'] == 'null') continue;\n      \
         final own = <(String, String)>[];\n      \
         _{prefix}McpMismatches(alternative, value, defs, passed, at, own);\n      \
         if (own.isEmpty) return;\n      \
         named ??= own;\n    \
         }}\n    \
         if (named != null) found.addAll(named);\n    \
         return;\n  \
         }}\n  \
         final allowed = schema['enum'];\n  \
         if (allowed is List && !allowed.contains(value)) {{\n    \
         refuse(at, 'invalid value, expected one of the listed values');\n    \
         return;\n  \
         }}\n  \
         switch (schema['type']) {{\n    \
         case 'object':\n      \
         if (value is! Map) {{\n        \
         refuse(at, 'invalid type, expected an object');\n        \
         return;\n      \
         }}\n      \
         final properties = schema['properties'];\n      \
         final others = schema['additionalProperties'];\n      \
         for (final entry in value.entries) {{\n        \
         final key = '${{entry.key}}';\n        \
         if (passed.contains(key)) continue;\n        \
         final declared = properties is Map && properties.containsKey(entry.key);\n        \
         if (!declared && others == false) {{\n          \
         refuse(<String>[...at, key], 'unknown field `$key`');\n          \
         continue;\n        \
         }}\n        \
         _{prefix}McpMismatches(declared ? properties[entry.key] : others, entry.value, defs, const <String>[], <String>[...at, key], found);\n      \
         }}\n      \
         final required = schema['required'];\n      \
         if (required is List) {{\n        \
         for (final key in required) {{\n          \
         if (!value.containsKey(key) && !passed.contains(key)) refuse(<String>[...at, '$key'], 'missing field `$key`');\n        \
         }}\n      \
         }}\n    \
         case 'array':\n      \
         if (value is! List) {{\n        \
         refuse(at, 'invalid type, expected an array');\n        \
         return;\n      \
         }}\n      \
         for (var index = 0; index < value.length; index += 1) {{\n        \
         _{prefix}McpMismatches(schema['items'], value[index], defs, const <String>[], <String>[...at, '$index'], found);\n      \
         }}\n    \
         case 'string':\n      \
         if (value is! String) refuse(at, 'invalid type, expected a string');\n    \
         case 'integer':\n      \
         if (!(value is int || (value is double && value == value.truncateToDouble()))) refuse(at, 'invalid type, expected an integer');\n    \
         case 'number':\n      \
         if (value is! num) refuse(at, 'invalid type, expected a number');\n    \
         case 'boolean':\n      \
         if (value is! bool) refuse(at, 'invalid type, expected a boolean');\n  \
         }}\n\
         }}",
    )
}

/// The Dart that finds the JSON schema of a tool or a prompt in the catalog.
fn schema_lookups(prefix: &str, tools: bool, service: &ServiceDef) -> String {
    let mut text = String::new();
    if tools {
        let _ = write!(
            text,
            "\n\nObject? _{prefix}McpMessageSchema(String tool, bool wrapped) {{\n  \
             for (final written in {prefix}McpCatalog.tools) {{\n    \
             final parsed = jsonDecode(written);\n    \
             if (parsed is Map && parsed['name'] == tool) {{\n      \
             final schema = parsed['inputSchema'];\n      \
             if (wrapped && schema is Map && schema['properties'] is Map) {{\n        \
             final held = (schema['properties'] as Map)['value'];\n        \
             return held is Map ? <Object?, Object?>{{...held, '\\$defs': schema['\\$defs']}} : held;\n      \
             }}\n      \
             return schema;\n    \
             }}\n  \
             }}\n  \
             return null;\n\
             }}"
        );
    }
    if service.operations.iter().any(OperationDef::is_mcp_prompt) {
        let _ = write!(
            text,
            "\n\nObject? _{prefix}McpPromptSchema(String prompt) {{\n  \
             for (final written in {prefix}McpCatalog.prompts) {{\n    \
             final parsed = jsonDecode(written);\n    \
             if (parsed is Map && parsed['name'] == prompt && parsed['arguments'] is List) {{\n      \
             final arguments = (parsed['arguments'] as List).whereType<Map>().toList();\n      \
             return <String, dynamic>{{\n        \
             'type': 'object',\n        \
             'properties': <String, dynamic>{{for (final argument in arguments) '${{argument['name']}}': <String, dynamic>{{'type': 'string'}}}},\n        \
             'required': <String>[for (final argument in arguments) if (argument['required'] == true) '${{argument['name']}}'],\n      \
             }};\n    \
             }}\n  \
             }}\n  \
             return null;\n\
             }}"
        );
    }
    if reads_templates(service) {
        let _ = write!(
            text,
            "\n\n/// The schema of the message each resource template reads, as the JSON text Rust wrote.\n\
             const _{prefix}McpReadInputs = <String>[{READS_MARKER}];\n\n\
             Object? _{prefix}McpReadSchema(String operation) {{\n  \
             for (final written in _{prefix}McpReadInputs) {{\n    \
             final parsed = jsonDecode(written);\n    \
             if (parsed is Map && parsed['name'] == operation) return parsed['input'];\n  \
             }}\n  \
             return null;\n\
             }}"
        );
    }
    text
}

/// Whether a resource template of the service reads a message, which a refused read is walked
/// against.
fn reads_templates(service: &ServiceDef) -> bool {
    service.operations.iter().any(|operation| {
        operation
            .mcp
            .resource
            .as_ref()
            .is_some_and(McpResourceUri::is_template)
    })
}

fn answer_helpers(
    named: &str,
    prefix: &str,
    calls: &Calls,
    held: &Resourced<'_>,
    service: &ServiceDef,
) -> String {
    let tools = tools_of(service).next().is_some();
    let panics =
        calls.declared || held.any() || service.operations.iter().any(OperationDef::is_mcp_prompt);
    let fields = fault_fields_typescript_name(named);
    let issue = fault_issue_name(named);
    let mut text = format!(
        "{fields} _{prefix}McpFault({named}FaultKind kind, String operation, String detail, {{String? field, List<{issue}>? issues}}) =>\n    \
         {fields}(detail: detail, field: field, issues: issues, kind: kind, operation: operation);\n\n\
         String? _{prefix}McpField(String message) {{\n  \
         if (!message.startsWith(\"'\")) return null;\n  \
         final end = message.indexOf(\"'\", 1);\n  \
         return end < 0 ? null : message.substring(1, end);\n\
         }}"
    );
    text.push_str("\n\n");
    text.push_str(&unread_helpers(named, prefix));
    text.push_str(&schema_lookups(prefix, tools, service));
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
    if tools_of(service).any(|operation| matches!(operation.outcome, OperationOutcome::OneWay)) {
        let _ = write!(
            text,
            "\n\nvoid _{prefix}McpToStderr({named}FaultFields fault) => stderr.writeln(_{prefix}McpJson(fault.toJson()));"
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

/// The Dart expression telling whether the message of a tool sits under the `value` key, and the
/// statement refusing a call that holds none where it must.
fn wrapped_check(
    named: &str,
    prefix: &str,
    operation: &OperationDef,
    shape: &HttpShape,
) -> (String, String) {
    let tool = quoted(operation.mcp_tool_name());
    let known = wrapped_at_expansion(operation, shape);
    let wrapped = known.map_or_else(
        || format!("_{prefix}McpServerWrapped[{tool}]!"),
        |under| under.to_string(),
    );
    let absent = match known {
        Some(false) => return (wrapped, String::new()),
        Some(true) => "!members.containsKey('value')".to_owned(),
        None => format!("{wrapped} && !members.containsKey('value')"),
    };
    let refusal = format!(
        "if ({absent}) {{\n    \
         return _{prefix}McpFaultResult(\n      \
         _{prefix}McpFault({named}FaultKind.failedValidation, {tool}, \"'value': missing field `value`\", field: 'value'),\n    \
         );\n  \
         }}\n  "
    );
    (wrapped, refusal)
}

/// The arguments of `_McpUnreadFault` that list every mismatch of a tool's message, read past the
/// keys read apart from it.
fn every_mismatch(operation: &OperationDef) -> String {
    let passed: Vec<String> = read_apart(operation)
        .iter()
        .map(|key| quoted(key))
        .collect();
    format!("passed: const <String>[{}], every: true", passed.join(", "))
}

fn server_call(named: &str, prefix: &str, operation: &OperationDef) -> String {
    let shape = HttpShape::of(operation);
    let tool = quoted(operation.mcp_tool_name());
    let call = dart_call(operation);
    let req_ty = message_dart_typename(operation);
    let source = source_expr(prefix, operation, &shape);
    let (wrapped, missing_value) = wrapped_check(named, prefix, operation, &shape);
    let decode = message_decode(operation, "source");
    let unjudged = unjudged_decode(operation, "source");
    let kind = format!("{named}FaultKind");
    let mut text = format!(
        "Future<{named}McpAnswer> _{prefix}McpCall{op}<Ctx>(\n  \
         {named}Handlers<Ctx> handlers,\n  \
         Ctx ctx,\n  \
         Map<String, dynamic> params,\n  \
         Object? arguments,{fault_param}\n\
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
         }}\n\
         {answers}  \
         {missing_value}  \
         final dynamic source = {source};\n  \
         final {req_ty} decoded;\n  \
         try {{\n    \
         decoded = {decode};\n  \
         }} on FormatException catch (rejected) {{\n    \
         try {{\n      \
         {unjudged};\n    \
         }} on FormatException catch (unread) {{\n      \
         return _{prefix}McpFaultResult(_{prefix}McpUnreadFault({tool}, unread.message, _{prefix}McpMessageSchema({tool}, {wrapped}), source, {every}));\n    \
         }} catch (unread) {{\n      \
         return _{prefix}McpFaultResult(_{prefix}McpUnreadFault({tool}, '$unread', _{prefix}McpMessageSchema({tool}, {wrapped}), source, {every}));\n    \
         }}\n    \
         return _{prefix}McpFaultResult(\n      \
         _{prefix}McpFault({kind}.failedValidation, {tool}, rejected.message, field: _{prefix}McpField(rejected.message)),\n    \
         );\n  \
         }} catch (rejected) {{\n    \
         return _{prefix}McpFaultResult(_{prefix}McpUnreadFault({tool}, '$rejected', _{prefix}McpMessageSchema({tool}, {wrapped}), source, {every}));\n  \
         }}\n",
        op = pascal(operation),
        every = every_mismatch(operation),
        answers = answers_stmt(named, prefix, operation),
        fault_param = if matches!(operation.outcome, OperationOutcome::OneWay) {
            format!("\n  void Function({named}FaultFields fault)? onFault,")
        } else {
            String::new()
        },
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
             (onFault ?? _{prefix}McpToStderr)(\n      \
             _{prefix}McpFault({kind}.handlerPanic, {tool}, '$unexpected'),\n    \
             );\n  \
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
    let ok = if is_content_list(operation) {
        format!("_{prefix}McpBlocks(<Object>[for (final block in answered) block.toJson()])")
    } else if unit {
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
    let (asking, declared_body) = asking_stmt(prefix, operation, &declared);
    let media = if is_content_pair(operation) {
        format!(
            "    final media = _{prefix}McpMedia(answered.$1, answered.$2);\n    \
             if (media != null) return media;\n"
        )
    } else {
        String::new()
    };
    format!(
        "  try {{\n    \
         {call_stmt}\n\
{value_pre}{media}    \
         return {ok};\n  \
         }} on {error_ty} catch (declared) {{\n\
{error_pre}{asking}    \
         return _{prefix}McpDeclared({declared_body});\n  \
         }} catch (unexpected) {{\n    \
         return _{prefix}McpPanic({tool}, {tool}, '$unexpected');\n  \
         }}\n\
         }}"
    )
}

/// Stands in the emitted text for the key serde tags the arm an operation asks with, and for the
/// name it writes the arm under and the key it holds the arm's members under: `part` is `TAG`, `WIRE`
/// or `CONTENT`.
fn arm_marker(operation: &OperationDef, part: &str) -> String {
    format!("@ARM_{part}:{}@", written(&operation.ident))
}

/// One `.replace` for the tag key, the wire name and the content key of the arm each interview operation asks with.
fn arm_replacements(declared: &ServiceDef) -> Vec<TokenStream> {
    declared
        .operations
        .iter()
        .filter(|operation| operation.mcp.interview.is_some())
        .flat_map(|operation| {
            [
                ("TAG", "mcp_arm_tag"),
                ("WIRE", "mcp_arm_wire"),
                ("CONTENT", "mcp_arm_content"),
            ]
            .map(|(part, head)| {
                let marker = arm_marker(operation, part);
                let method = format_ident!(
                    "{head}_{}",
                    written(&operation.ident),
                    span = operation.ident.span()
                );
                quote! { .replace(#marker, &literal(Self::#method())) }
            })
        })
        .collect()
}

/// Stands in the emitted text for the JSON text of the form `mcp_form_{operation}()` answers.
fn form_marker(operation: &OperationDef) -> String {
    format!("@FORM:{}@", written(&operation.ident))
}

/// The statements that read the answers of a retried form call into the arguments, ahead of the
/// message's own read.
fn answers_stmt(named: &str, prefix: &str, operation: &OperationDef) -> String {
    let Some(wire) = answers_key(operation) else {
        return String::new();
    };
    let key = quoted(&wire);
    let held = match &operation.inputs {
        OperationInputs::Generated(fields) => fields
            .iter()
            .find(|(field, _)| wire_key(field) == wire)
            .and_then(|(_, ty)| option_inner(ty)),
        OperationInputs::Empty | OperationInputs::Named(_) => None,
    };
    let read = held.map_or_else(String::new, |form| {
        format!(
            "      try {{\n        \
             {};\n      \
             }} catch (rejected) {{\n        \
             return _{prefix}McpFaultResult(\n          \
             _{prefix}McpFault({named}FaultKind.failedValidation, {}, '$rejected', field: {key}),\n        );\n      \
             }}\n",
            dart_json_decode(form, "content"),
            quoted(&operation.wire_name),
        )
    });
    format!(
        "  var declined = false;\n  \
         final responses = params['inputResponses'];\n  \
         final response = responses is Map<String, dynamic> ? responses[{key}] : null;\n  \
         if (response is Map<String, dynamic>) {{\n    \
         if (response['action'] == 'accept') {{\n      \
         final content = response['content'] ?? <String, dynamic>{{}};\n{read}      \
         members[{key}] = content;\n    \
         }} else {{\n      \
         members.remove({key});\n      \
         declined = true;\n    \
         }}\n  \
         }}\n"
    )
}

/// The statements that answer the result that asks, where the declared error is the arm that asks,
/// and the body the declared error is answered with otherwise.
fn asking_stmt(prefix: &str, operation: &OperationDef, declared: &str) -> (String, String) {
    let Some(interview) = operation.mcp.interview.as_ref() else {
        return (String::new(), declared.to_owned());
    };
    let answers = answers_key(operation).map_or_else(|| "null".to_owned(), |key| quoted(&key));
    let form = if interview.answers.is_some() {
        form_marker(operation)
    } else {
        "null".to_owned()
    };
    let declined = if interview.answers.is_some() {
        "declined"
    } else {
        "false"
    };
    let asking = format!(
        "    final Object? declaredBody = {declared};\n    \
         final asking = _{prefix}McpAsking({}, {}, {}, {answers}, {form}, {}, declaredBody, params, {declined});\n    \
         if (asking != null) return asking;\n",
        arm_marker(operation, "TAG"),
        arm_marker(operation, "WIRE"),
        arm_marker(operation, "CONTENT"),
        quoted(operation.doc.as_deref().unwrap_or_default()),
    );
    (asking, "declaredBody".to_owned())
}

/// The statements of the tasks route that call one of the operations a task tool names.
fn task_call(
    service: &ServiceDef,
    prefix: &str,
    start: &OperationDef,
    method: &str,
) -> Option<String> {
    let task = start.mcp.task.as_ref()?;
    let wanted = match method {
        "tasks/get" => task.get.as_deref(),
        "tasks/cancel" => task.cancel.as_deref(),
        _ => task.update.as_deref(),
    }?;
    let target = service
        .operations
        .iter()
        .find(|candidate| written(&candidate.ident) == wanted)?;
    let tool = quoted(start.mcp_tool_name());
    let calling = |arguments: &str| {
        format!(
            "_{prefix}McpCall{}(handlers, ctx, params, <String, dynamic>{{{arguments}}})",
            pascal(target)
        )
    };
    let body = match method {
        "tasks/get" => format!(
            "      return _{prefix}McpTaskAnswer(\n          {},\n          '{}: the service refused the task',\n          \
             (structured) => <String, dynamic>{{\
             ..._{prefix}McpTaskView(taskId, structured as Map<String, dynamic>),\
             'resultType': 'complete',\
             }});",
            calling("'value': held"),
            start.mcp_tool_name()
        ),
        "tasks/cancel" => format!(
            "      return _{prefix}McpTaskAnswer(\n          {},\n          '{}: the service refused to cancel the task',\n          \
             (structured) => <String, dynamic>{{}});",
            calling("'value': held"),
            start.mcp_tool_name()
        ),
        _ => {
            let OperationInputs::Generated(fields) = &target.inputs else {
                return None;
            };
            let keys: Vec<String> = fields
                .iter()
                .map(|(field, _)| quoted(&wire_key(field)))
                .collect();
            let [id_key, key_key, action_key, content_key] = keys.as_slice() else {
                return None;
            };
            let arguments = format!(
                "{id_key}: held, {key_key}: key, {action_key}: given['action'] is String ? given['action'] : '', \
                 if (given.containsKey('content')) {content_key}: _{prefix}McpJson(given['content'])"
            );
            format!(
                "      final responses = params['inputResponses'];\n      \
                 if (responses is! Map<String, dynamic>) {{\n        \
                 return (result: null, error: <String, dynamic>{{'code': -32602, 'message': 'Invalid params: tasks/update needs `inputResponses`'}});\n      \
                 }}\n      \
                 for (final key in responses.keys.toList()..sort()) {{\n        \
                 final response = responses[key];\n        \
                 final given = response is Map<String, dynamic> ? response : <String, dynamic>{{}};\n        \
                 final answered = await _{prefix}McpTaskAnswer(\n            {},\n            '{}: the service refused the answer',\n            \
                 (structured) => <String, dynamic>{{}});\n        \
                 if (answered.error != null) return answered;\n      \
                 }}\n      \
                 return (result: <String, dynamic>{{}}, error: null);",
                calling(&arguments),
                start.mcp_tool_name()
            )
        }
    };
    Some(format!("    case ({}, {tool}):\n{body}\n", quoted(method)))
}

/// What the tasks route and the tool that starts a task share: whether the client declared the
/// extension, the handle a declared client receives, and the answer of a task method.
fn task_helpers(service: &ServiceDef, named: &str, prefix: &str) -> String {
    let mut cases = String::new();
    for start in service
        .operations
        .iter()
        .filter(|operation| operation.mcp.task.is_some())
    {
        for method in ["tasks/get", "tasks/cancel", "tasks/update"] {
            if let Some(text) = task_call(service, prefix, start, method) {
                cases.push_str(&text);
            }
        }
    }
    format!(
        "/// Whether the client declared the tasks extension.\n\
         bool _{prefix}McpDeclaresTasks(Map<String, dynamic> params) {{\n  \
         final meta = params['_meta'];\n  \
         final capabilities = meta is Map ? meta['io.modelcontextprotocol/clientCapabilities'] : null;\n  \
         final extensions = capabilities is Map ? capabilities['extensions'] : null;\n  \
         return extensions is Map && extensions['io.modelcontextprotocol/tasks'] is Map;\n\
         }}\n\n\
         /// The answer of the tool that starts a task: the task a declared client receives, and the\n\
         /// handle as a structured value for any other.\n\
         Future<{named}McpAnswer> _{prefix}McpTaskStart(Future<{named}McpAnswer> answering, String tool, Map<String, dynamic> params) async {{\n  \
         final answered = await answering;\n  \
         final result = answered.result;\n  \
         if (answered.error != null || result is! Map<String, dynamic> || result['isError'] == true || !_{prefix}McpDeclaresTasks(params)) {{\n    \
         return answered;\n  \
         }}\n  \
         final handle = Map<String, dynamic>.of(result['structuredContent'] as Map<String, dynamic>);\n  \
         handle['taskId'] = '$tool:${{handle['taskId']}}';\n  \
         handle['resultType'] = 'task';\n  \
         return (result: handle, error: null);\n\
         }}\n\n\
         /// The answer of a task method: what `success` makes of the structured value of the tool it\n\
         /// calls, or the `-32602` of the fault or declared error that tool answered.\n\
         Future<{named}McpAnswer> _{prefix}McpTaskAnswer(\n    \
         Future<{named}McpAnswer> answering,\n    \
         String refusal,\n    \
         Object? Function(Object? structured) success,\n\
         ) async {{\n  \
         final answered = await answering;\n  \
         final result = answered.result;\n  \
         if (answered.error != null || result is! Map<String, dynamic>) return answered;\n  \
         if (result['isError'] != true) return (result: success(result['structuredContent']), error: null);\n  \
         final parsed = jsonDecode(((result['content'] as List).first as Map<String, dynamic>)['text'] as String);\n  \
         final fault = parsed is Map<String, dynamic> && parsed['isServiceFault'] == true;\n  \
         return (\n    \
         result: null,\n    \
         error: <String, dynamic>{{'code': -32602, 'data': parsed, 'message': fault ? 'Invalid params' : refusal}},\n  \
         );\n\
         }}\n\n\
         /// Answers `tasks/get`, `tasks/cancel` and `tasks/update` for the tasks the tools of this service\n\
         /// start, and `null` for a task another tool started.\n\
         Future<{named}McpAnswer?> _{prefix}McpTasks<Ctx>(\n    \
         {named}Handlers<Ctx> handlers,\n    \
         Ctx ctx,\n    \
         String method,\n    \
         Map<String, dynamic> params,\n\
         ) async {{\n  \
         final taskId = params['taskId'];\n  \
         final colon = taskId is String ? taskId.indexOf(':') : -1;\n  \
         if (taskId is! String || colon < 0) return null;\n  \
         final tool = taskId.substring(0, colon);\n  \
         final held = taskId.substring(colon + 1);\n  \
         switch ((method, tool)) {{\n{cases}  \
         default:\n      \
         return null;\n  \
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
         create{named}McpRoute<Ctx>({named}Handlers<Ctx> handlers, [void Function({named}FaultFields fault)? onFault]) =>"
    );
    if service.operations.is_empty() {
        return format!("{preamble}\n        (ctx, method, params) async => null;");
    }
    let mut cases = String::new();
    let tools: Vec<&OperationDef> = tools_of(service).collect();
    if !tools.is_empty() {
        let mut arms = String::new();
        for operation in tools {
            let fault_argument = if matches!(operation.outcome, OperationOutcome::OneWay) {
                ", onFault"
            } else {
                ""
            };
            let call = format!(
                "_{prefix}McpCall{}(handlers, ctx, params, params['arguments']{fault_argument})",
                pascal(operation)
            );
            let returned = if operation.mcp.task.is_some() {
                format!(
                    "_{prefix}McpTaskStart({call}, {}, params)",
                    quoted(operation.mcp_tool_name())
                )
            } else {
                call
            };
            let _ = writeln!(
                arms,
                "            case {}:\n              return {returned};",
                quoted(operation.mcp_tool_name()),
            );
        }
        let _ = write!(
            cases,
            "        case 'tools/call':\n          switch (params['name']) {{\n{arms}            \
             default:\n              return null;\n          }}\n"
        );
    }
    if held.reads().next().is_some() || skilled(service) {
        let _ = write!(
            cases,
            "        case 'resources/read':\n          final uri = params['uri'];\n          \
             return uri is String ? _{prefix}McpRead<Ctx>(handlers, ctx, uri) : null;\n"
        );
    }
    if skilled(service) {
        let _ = write!(
            cases,
            "        case 'skills/get':\n          return _{prefix}McpSkillGet(params);\n"
        );
    }
    if held.listing.is_some() {
        let _ = write!(
            cases,
            "        case 'resources/list':\n          \
             return _{prefix}McpList<Ctx>(handlers, ctx, params['cursor']);\n"
        );
    }
    if service
        .operations
        .iter()
        .any(|operation| operation.mcp.task.is_some())
    {
        let _ = write!(
            cases,
            "        case 'tasks/get' || 'tasks/cancel' || 'tasks/update':\n          \
             return _{prefix}McpTasks<Ctx>(handlers, ctx, method, params);\n"
        );
    }
    if service.operations.iter().any(OperationDef::is_mcp_prompt) {
        let _ = write!(
            cases,
            "        case 'prompts/get':\n          \
             return _{prefix}McpGetPrompt<Ctx>(handlers, ctx, params);\n"
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
fn capabilities_literal(service: &ServiceDef, held: &Resourced<'_>, skilled: bool) -> String {
    let mut capabilities = vec![quoted("{\"tools\":{}}")];
    if service.operations.iter().any(OperationDef::is_mcp_prompt) {
        capabilities.push(quoted("{\"prompts\":{}}"));
    }
    if held.is_served() || skilled {
        capabilities.push(quoted("{\"resources\":{}}"));
    }
    if skilled {
        capabilities.push(quoted(
            "{\"extensions\":{\"io.modelcontextprotocol/skills\":{}}}",
        ));
    }
    if !held.completions.is_empty() {
        capabilities.push(quoted("{\"completions\":{}}"));
    }
    if service
        .operations
        .iter()
        .any(|operation| operation.mcp.task.is_some())
    {
        capabilities.push(quoted(
            "{\"extensions\":{\"io.modelcontextprotocol/tasks\":{}}}",
        ));
    }
    capabilities.join(", ")
}

/// Whether the service ships a skills folder or has an operation that writes a skill.
fn skilled(service: &ServiceDef) -> bool {
    service.skills.is_some() || !dynamic_skills(service).is_empty()
}

/// The operations that write a skill at run time.
fn dynamic_skills(service: &ServiceDef) -> Vec<&OperationDef> {
    service
        .operations
        .iter()
        .filter(|operation| operation.mcp.skill.is_some())
        .collect()
}

/// The constant that holds the files of the skills folder, over the marker the accessor fills.
fn skill_files_declaration(service: &ServiceDef, prefix: &str) -> String {
    if service.skills.is_none() {
        return String::new();
    }
    format!(
        "/// The content `resources/read` answers for each file of the skills folder, as JSON text, by URI.\n\
         const _{prefix}McpSkillFiles = <String, String>{{{SKILL_FILES_MARKER}}};\n\n"
    )
}

/// `McpResource` and `McpResourcePage`, which a listing answers and the Rust service module
/// declares.
pub fn page_types(service: &ServiceDef) -> String {
    if !service
        .operations
        .iter()
        .any(|operation| operation.mcp.list_resources)
    {
        return String::new();
    }
    "/// One resource of a listing, as `resources/list` writes it.
class @N@McpResource {
  @N@McpResource({required this.uri, required this.name, this.title, this.description, this.mimeType, this.size});

  factory @N@McpResource.fromJson(Map<String, dynamic> json) => @N@McpResource(
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
class @N@McpResourcePage {
  @N@McpResourcePage({required this.resources, this.nextCursor});

  factory @N@McpResourcePage.fromJson(Map<String, dynamic> json) => @N@McpResourcePage(
        resources: [
          for (final resource in json['resources'] as List<dynamic>)
            @N@McpResource.fromJson(resource as Map<String, dynamic>),
        ],
        nextCursor: json['nextCursor'] as String?,
      );

  final List<@N@McpResource> resources;
  final String? nextCursor;

  Map<String, dynamic> toJson() => <String, dynamic>{
        'resources': [for (final resource in resources) resource.toJson()],
        if (nextCursor != null) 'nextCursor': nextCursor,
      };
}"
    .replace("@N@", &service.ident.to_string())
}

/// The pieces of the resource requests, one function for each operation that answers one.
fn resource_functions(
    service: &ServiceDef,
    held: &Resourced<'_>,
    named: &str,
    prefix: &str,
) -> Vec<String> {
    let prompts: Vec<&OperationDef> = service
        .operations
        .iter()
        .filter(|operation| operation.is_mcp_prompt())
        .collect();
    let dynamic = dynamic_skills(service);
    if !held.any() && prompts.is_empty() && !skilled(service) {
        return Vec::new();
    }
    let fields = fault_fields_typescript_name(named);
    let mut published = Vec::new();
    if held.any() || !prompts.is_empty() || !dynamic.is_empty() {
        published.push(filled(UNREAD, named, prefix).replace("@F@", &fields));
    }
    if !prompts.is_empty() {
        published.extend(
            prompts
                .iter()
                .map(|operation| prompt_function(named, prefix, operation)),
        );
        published.push(prompt_dispatch(&prompts, named, prefix));
    }
    if held.reads().next().is_some() {
        published.push(filled(CAPTURED, named, prefix));
        published.extend(
            held.reads()
                .map(|operation| resource_read(named, prefix, operation)),
        );
    }
    if held.reads().next().is_some() || skilled(service) {
        published.extend(
            dynamic
                .iter()
                .map(|operation| skill_read(named, prefix, operation)),
        );
        published.push(read_dispatch(service, held, named, prefix));
    }
    if skilled(service) {
        published.push(skill_get(named, prefix));
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
    walked: &Walked<'_>,
    refusal: &str,
    answer: &str,
) -> String {
    let schema = if walked.every {
        format!("{}, source, every: true", walked.schema)
    } else {
        format!("{}, source", walked.schema)
    };
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
         return _{prefix}McpUnread(_{prefix}McpUnreadFault({wire}, unread.message, {schema}));\n    \
         }} catch (unread) {{\n      \
         return _{prefix}McpUnread(_{prefix}McpUnreadFault({wire}, '$unread', {schema}));\n    \
         }}\n    \
         return _{prefix}McpUnread(\n      \
         _{prefix}McpFault({kind}.failedValidation, {wire}, rejected.message, field: _{prefix}McpField(rejected.message)),\n    \
         );\n  \
         }} catch (rejected) {{\n    \
         return _{prefix}McpUnread(_{prefix}McpUnreadFault({wire}, '$rejected', {schema}));\n  \
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
    let read_schema = if resource.is_template() {
        format!("_{prefix}McpReadSchema({})", quoted(&operation.wire_name))
    } else {
        "null".to_owned()
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
            &Walked {
                every: resource.is_template(),
                schema: &read_schema,
            },
            "'Resource not found'",
            &success,
        )
    )
}

/// The read of a file of a skill an operation writes: the frontmatter ahead of the manifest's text.
fn skill_read(named: &str, prefix: &str, operation: &OperationDef) -> String {
    let Some((skill, _)) = operation.mcp.skill.as_ref() else {
        return String::new();
    };
    let blob = matches!(
        &operation.outcome,
        OperationOutcome::Reply { success, error: _error } if matches!(success.as_ref(), syn::Type::Tuple(_))
    );
    let content = if blob {
        "<String, dynamic>{'uri': uri, 'mimeType': answered.$2, 'blob': base64Encode(answered.$1)}"
            .to_owned()
    } else {
        let frontmatter = quoted(&written_frontmatter(
            skill,
            operation.doc.as_deref().unwrap_or_default(),
        ));
        format!(
            "<String, dynamic>{{'uri': uri, 'mimeType': 'text/markdown', \
             'text': path == 'SKILL.md' ? {frontmatter} + @ENCODED@ : @ENCODED@}}"
        )
    };
    let success = format!(
        "return (\n      result: <String, dynamic>{{\n        \
         'cacheScope': 'private',\n        \
         'contents': <Object>[{content}],\n        \
         'resultType': 'complete',\n        \
         'ttlMs': 0,\n      }},\n      error: null,\n    );"
    );
    let head = format!(
        "Future<{named}McpAnswer> _{prefix}McpReadSkill{op}<Ctx>(\n  \
         {named}Handlers<Ctx> handlers,\n  \
         Ctx ctx,\n  \
         String uri,\n  \
         String path,\n\
         ) async {{\n",
        op = pascal(operation),
    );
    format!(
        "{head}{}",
        resource_body(
            named,
            prefix,
            operation,
            "path",
            &Walked {
                every: false,
                schema: "null",
            },
            "'Skill file not found'",
            &success,
        )
    )
}

/// The `skills/get` of the service: the entry its catalog lists under the name or the URI asked.
fn skill_get(named: &str, prefix: &str) -> String {
    format!(
        "{named}McpAnswer? _{prefix}McpSkillGet(Map<String, dynamic> params) {{\n  \
         final name = params['name'];\n  \
         final uri = params['uri'];\n  \
         for (final text in {prefix}McpCatalog.skills) {{\n    \
         final entry = jsonDecode(text) as Map<String, dynamic>;\n    \
         final frontmatter = entry['frontmatter'];\n    \
         final held = (name is String && frontmatter is Map && frontmatter['name'] == name) ||\n        \
         (uri is String && entry['uri'] == uri);\n    \
         if (!held) continue;\n    \
         final written = entry['resources'] == 'dynamic';\n    \
         return (\n      result: <String, dynamic>{{\n        \
         'cacheScope': written ? 'private' : 'public',\n        \
         'resultType': 'complete',\n        \
         'skill': entry,\n        \
         'ttlMs': written ? 0 : 300000,\n      }},\n      error: null,\n    );\n  }}\n  \
         return null;\n}}"
    )
}

fn read_dispatch(service: &ServiceDef, held: &Resourced<'_>, named: &str, prefix: &str) -> String {
    let mut text = format!(
        "Future<{named}McpAnswer?> _{prefix}McpRead<Ctx>({named}Handlers<Ctx> handlers, Ctx ctx, String uri) async {{\n"
    );
    if service.skills.is_some() {
        let _ = write!(
            text,
            "  final folder = _{prefix}McpSkillFiles[uri];\n  \
             if (folder != null) {{\n    \
             return (\n      result: <String, dynamic>{{\n        \
             'cacheScope': 'public',\n        \
             'contents': <Object>[jsonDecode(folder)],\n        \
             'resultType': 'complete',\n        \
             'ttlMs': 300000,\n      }},\n      error: null,\n    );\n  }}\n"
        );
    }
    for operation in dynamic_skills(service) {
        let Some((skill, _)) = operation.mcp.skill.as_ref() else {
            continue;
        };
        let root = quoted(&format!("skill://{skill}/"));
        let _ = write!(
            text,
            "  {{\n    const root = {root};\n    \
             if (uri.startsWith(root) && uri.length > root.length) {{\n      \
             return _{prefix}McpReadSkill{op}<Ctx>(handlers, ctx, uri, uri.substring(root.length));\n    }}\n  }}\n",
            op = pascal(operation),
        );
    }
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
            &Walked {
                every: false,
                schema: "null",
            },
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
    let completion_schema = format!(
        "<String, dynamic>{{'type': 'object', 'properties': <String, dynamic>{{{typed_key}: <String, dynamic>{{'type': 'string'}}, {chosen_key}: <String, dynamic>{{'type': 'object', 'additionalProperties': <String, dynamic>{{'type': 'string'}}}}}}}}"
    );
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
            &Walked {
                every: false,
                schema: &completion_schema,
            },
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
         if (reference is! Map) return null;\n  \
         final kind = reference['type'];\n  \
         final uri = kind == 'ref/prompt' ? reference['name'] : kind == 'ref/resource' ? reference['uri'] : null;\n  \
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
            .find(|candidate| written(&candidate.ident) == completes.operation)
        else {
            continue;
        };
        let (reference, name, parameter) = match completes.kind {
            McpCompletesKind::Resource => {
                let Some(resource) = target.mcp.resource.as_ref() else {
                    continue;
                };
                (
                    "ref/resource",
                    resource.uri.clone(),
                    completes.parameter.clone(),
                )
            }
            McpCompletesKind::Prompt => {
                let Some(argument) = target.mcp.prompt.as_ref().and_then(|prompt| {
                    prompt
                        .arguments
                        .iter()
                        .find(|(argument, _)| written(argument) == completes.parameter)
                }) else {
                    continue;
                };
                (
                    "ref/prompt",
                    target.mcp_tool_name().to_owned(),
                    wire_key(&argument.0),
                )
            }
        };
        let _ = writeln!(
            text,
            "  if (kind == {} && uri == {} && parameter == {}) return _{prefix}McpComplete{}<Ctx>(handlers, ctx, typed, chosen);",
            quoted(reference),
            quoted(&name),
            quoted(&parameter),
            pascal(operation),
        );
    }
    text.push_str("  return null;\n}");
    text
}

/// The prompt `operation`: reads its arguments, calls its handler and answers its messages.
fn prompt_function(named: &str, prefix: &str, operation: &OperationDef) -> String {
    let source = match &operation.inputs {
        OperationInputs::Empty => "<String, dynamic>{}".to_owned(),
        OperationInputs::Generated(_) => "arguments".to_owned(),
        OperationInputs::Named(_) => {
            format!("arguments[{}]", quoted(&prompt_argument_key(operation)))
        }
    };
    let description = quoted(operation.doc.as_deref().unwrap_or_default());
    let messages = if is_prompt_messages(operation) {
        "<Object>[for (final message in answered) message.toJson()]".to_owned()
    } else {
        "<Object>[\n          <String, dynamic>{\n            'content': <String, dynamic>{'text': answered, 'type': 'text'},\n            'role': 'user',\n          },\n        ]".to_owned()
    };
    let success = format!(
        "return (\n      result: <String, dynamic>{{\n        \
         'description': {description},\n        \
         'messages': {messages},\n        \
         'resultType': 'complete',\n      }},\n      error: null,\n    );"
    );
    let refusal = quoted(&format!(
        "{}: the service refused the prompt",
        operation.mcp_tool_name()
    ));
    let head = format!(
        "Future<{named}McpAnswer> _{prefix}McpPrompt{op}<Ctx>(\n  \
         {named}Handlers<Ctx> handlers,\n  \
         Ctx ctx,\n  \
         Map<String, dynamic> arguments,\n\
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
            &Walked {
                every: true,
                schema: &format!(
                    "_{prefix}McpPromptSchema({})",
                    quoted(operation.mcp_tool_name())
                ),
            },
            &refusal,
            &success,
        )
    )
}

/// The wire key of the one argument a prompt takes as its whole message.
fn prompt_argument_key(operation: &OperationDef) -> String {
    operation
        .mcp
        .prompt
        .iter()
        .filter_map(|prompt| prompt.arguments.first())
        .map(|(argument, _)| wire_key(argument))
        .next()
        .unwrap_or_default()
}

/// `_McpGetPrompt`: finds the prompt a `prompts/get` names.
fn prompt_dispatch(prompts: &[&OperationDef], named: &str, prefix: &str) -> String {
    let mut text = format!(
        "Future<{named}McpAnswer?> _{prefix}McpGetPrompt<Ctx>(\n  \
         {named}Handlers<Ctx> handlers,\n  \
         Ctx ctx,\n  \
         Map<String, dynamic> params,\n\
         ) async {{\n  \
         final name = params['name'];\n  \
         if (name is! String) return null;\n  \
         final given = params['arguments'];\n  \
         final arguments = given is Map<String, dynamic> ? given : <String, dynamic>{{}};\n"
    );
    for operation in prompts {
        let _ = writeln!(
            text,
            "  if (name == {}) return _{prefix}McpPrompt{}<Ctx>(handlers, ctx, arguments);",
            quoted(operation.mcp_tool_name()),
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
    if served_by_client(service).next().is_none() {
        published.push(format!(
            "/// A `{named}` caller over `mcp`.\n\
             class {named}McpClient {{\n  \
             {named}McpClient({named}McpTransport transport);\n\
             }}"
        ));
        return published.join("\n\n");
    }
    published.push(with_fields(CLIENT_CALL));
    if service.operations.iter().any(is_content_pair) {
        published.push(with_fields(CLIENT_PAIR));
    }
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
    let methods = served_by_client(service)
        .map(|operation| client_method(&named, &prefix, operation))
        .collect::<Vec<_>>()
        .join("\n\n");
    let prompting = service.operations.iter().any(OperationDef::is_mcp_prompt);
    let prompt_call = if prompting {
        format!("{}\n\n", with_fields(CLIENT_PROMPT_METHOD))
    } else {
        String::new()
    };
    published.push(format!(
        "/// A `{named}` caller over `mcp`.\n\
         class {named}McpClient {{\n  \
         {named}McpClient(this._transport);\n  \
         final {named}McpTransport _transport;\n\n\
         {call}\n\n\
         {prompt_call}{methods}\n\
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

/// The operations the client has a method for: the tools and the prompts.
fn served_by_client(service: &ServiceDef) -> impl Iterator<Item = &OperationDef> {
    service
        .operations
        .iter()
        .filter(|operation| operation.is_mcp_tool() || operation.is_mcp_prompt())
}

fn client_method(named: &str, prefix: &str, operation: &OperationDef) -> String {
    let shape = HttpShape::of(operation);
    let mut head = method_head(prefix, operation, &shape);
    if operation.is_mcp_prompt() {
        return prompt_method(named, prefix, operation, &shape, &mut head);
    }
    match &operation.outcome {
        OperationOutcome::OneWay => one_way_method(named, prefix, &head),
        OperationOutcome::Reply { error, success } => {
            reply_method(named, prefix, operation, &shape, &head, (success, error))
        }
    }
}

/// A prompt's method: the message's text members as the arguments, and the messages read as the
/// success its result type declares.
fn prompt_method(
    named: &str,
    prefix: &str,
    operation: &OperationDef,
    shape: &HttpShape,
    head: &mut MethodHead,
) -> String {
    let OperationOutcome::Reply { error, success } = &operation.outcome else {
        return String::new();
    };
    let sent = message_encode(operation);
    head.arguments = match &operation.inputs {
        OperationInputs::Named(_) => format!(
            "    final Object? sent = {sent};\n    \
             final arguments = <String, dynamic>{{if (sent is String) {}: sent}};\n",
            quoted(&prompt_argument_key(operation))
        ),
        OperationInputs::Empty | OperationInputs::Generated(_) => format!(
            "    final arguments = <String, dynamic>{{\n      \
             for (final entry in {sent}.entries)\n        \
             if (entry.value is String) entry.key: entry.value,\n    }};\n"
        ),
    };
    let text = if is_prompt_messages(operation) {
        "false"
    } else {
        "true"
    };
    let tool = &head.tool;
    let wire = &head.wire;
    reply_method(named, prefix, operation, shape, head, (success, error)).replace(
        &format!("result = await _call({tool}, {wire}, arguments);"),
        &format!("result = await _prompt({tool}, {wire}, arguments, {text});"),
    )
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
        let value = if is_content_list(operation) {
            "result['content']".to_owned()
        } else if is_content_pair(operation) {
            format!("result['structuredContent'] ?? _{prefix}McpPair(result['content'])")
        } else {
            "result['structuredContent']".to_owned()
        };
        format!("      final reply = <String, dynamic>{{'value': {value}}};\n")
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

/// One `.replace` per operation that asks with a form, writing `mcp_form_{operation}()` over its marker.
fn form_replacements(declared: &ServiceDef) -> Vec<TokenStream> {
    declared
        .operations
        .iter()
        .filter(|operation| {
            operation
                .mcp
                .interview
                .as_ref()
                .is_some_and(|interview| interview.answers.is_some())
        })
        .map(|operation| {
            let marker = form_marker(operation);
            let method = format_ident!(
                "mcp_form_{}",
                written(&operation.ident),
                span = operation.ident.span()
            );
            quote! { .replace(#marker, &literal(&Self::#method().to_string())) }
        })
        .collect()
}

/// The `skills` and `skill_files` literals `dart_mcp_service()` writes over their markers.
fn skill_literals() -> TokenStream {
    quote! {
        let skills = Self::mcp_skills()
            .iter()
            .map(|skill| literal(&skill.to_string()))
            .collect::<::std::vec::Vec<_>>()
            .join(", ");
        let skill_files = Self::mcp_skill_files()
            .iter()
            .map(|file| {
                ::std::format!(
                    "{}: {}",
                    literal(file["uri"].as_str().unwrap_or_default()),
                    literal(&file.to_string())
                )
            })
            .collect::<::std::vec::Vec<_>>()
            .join(", ");
    }
}

/// `dart_mcp_content()`: the value types of the service's blocks, messages, tasks and pages.
fn content_accessor(declared: &ServiceDef) -> Option<TokenStream> {
    let listed = declared
        .operations
        .iter()
        .any(|operation| is_content_list(operation) || is_prompt_messages(operation));
    let pages = page_types(declared);
    let tasked = has_tasks(declared);
    (listed || tasked || !pages.is_empty()).then(|| {
        let written = [
            listed.then(|| CONTENT_TYPE.to_owned()),
            Some(pages),
            tasked.then(|| TASK_TYPE.to_owned()),
        ]
            .into_iter()
            .flatten()
            .filter(|held| !held.is_empty())
            .collect::<Vec<_>>()
            .join("\n\n");
        quote! {
            #[doc = " The `McpContent`, `McpRole` and `McpPromptMessage` types a `List<McpContent>`"]
            #[doc = " success of an `mcp(content)` operation and a prompt's messages are written in."]
            #[doc = " A bundle names them once, and imports"]
            #[doc = " `dart:convert` and `dart:typed_data`."]
            pub fn dart_mcp_content() -> ::std::string::String {
                #written.to_owned()
            }
        }
    })
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
    let forms = form_replacements(declared);
    let arms = arm_replacements(declared);
    let skill_literals = skill_literals();
    let content = content_accessor(declared);
    quote! {
        #content

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
            let prompts = Self::mcp_prompts()
                .iter()
                .map(|prompt| literal(&prompt.to_string()))
                .collect::<::std::vec::Vec<_>>()
                .join(", ");
            let reads = Self::mcp_read_inputs()
                .into_iter()
                .map(|(name, input)| literal(&::serde_json::json!({ "input": input, "name": name }).to_string()))
                .collect::<::std::vec::Vec<_>>()
                .join(", ");
            #skill_literals
            let instructions = match Self::MCP_INSTRUCTIONS {
                ::core::option::Option::Some(text) => literal(text),
                ::core::option::Option::None => ::std::string::String::from("null"),
            };
            #server
                .replace(#TOOLS_MARKER, &tools)
                .replace(#NAMES_MARKER, &names)
                .replace(#RESOURCES_MARKER, &resources)
                .replace(#TEMPLATES_MARKER, &templates)
                .replace(#PROMPTS_MARKER, &prompts)
                .replace(#READS_MARKER, &reads)
                .replace(#SKILLS_MARKER, &skills)
                .replace(#SKILL_FILES_MARKER, &skill_files)
                .replace(#INSTRUCTIONS_MARKER, &instructions)
                .replace(#VERSION_MARKER, env!("CARGO_PKG_VERSION"))
                #(#wrapped_server)*
                #(#forms)*
                #(#arms)*
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
