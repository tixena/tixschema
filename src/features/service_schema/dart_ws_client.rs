//! The Dart `ws_rpc` client: a transport over the sink and stream a `WebSocketChannel` already
//! exposes, a per-operation client that calls out, and a dispatcher attachment for a service the
//! app implements — one service, one `emit()`, emitting both halves together.
//!
//! # Sink and stream, not a socket class
//!
//! Nothing here names `web_socket_channel` or `dart:io`: the transport's constructor asks for
//! `StreamSink<dynamic>`/`Stream<dynamic>`, the pair every `WebSocketChannel` already exposes, so a
//! caller hands over `channel.sink`/`channel.stream` and no adapter has to be written.
//!
//! # The attachment's seam is a record, not the transport class
//!
//! `WebSocketChannel.stream` is single-subscription: a second `stream.listen` on the same stream
//! throws, so only the transport itself may call `listen`. A second service sharing that socket
//! still needs a way to reach frames it did not ask for, but Dart has no structural class typing
//! and each service's [`emit`] is independent, so `attach{Named}WsDispatcher` can name neither the
//! calling service's own transport class nor a shared base class without this module emitting the
//! same class twice for every pair of services on one crate. A record is structural: every
//! `{Named}WsTransport` exposes the identically-shaped `frames` — a broadcast `Stream` fed by every
//! inbound frame the transport does not itself correlate to a pending `request`, paired with the
//! function that writes a frame back — and `attach{Named}WsDispatcher` takes that record rather
//! than a transport, so it composes across services sharing one socket the same way the
//! `http_rest` Dart client's own structural transport seam does.
//!
//! # A caller reads the outcome, exactly as the `http_rest` Dart client does
//!
//! A reply operation answers `Future<{Named}{Operation}Result>` — [`super::dart_result`]'s own
//! sealed pair, the same one `http_rest` answers — and never throws for a declared error or a
//! fault; a one-way operation still answers `Future<void>` and throws the fault-only
//! `{Named}WsRefusal`, having no reply arm to carry a fault through. The fault type either arm
//! carries (`{Named}FaultFields`) is the same shape [`super::dart_http_client`] already answers
//! with.
//!
//! # A decode failure is a failed-validation fault, not an undeserializable-payload one
//!
//! `http_rest` answers a body that will not decode with `undeserializablePayload` — a status
//! answered a shape its own code did not promise. `ws_rpc` has no status to make that distinction
//! with: a reply that will not decode is a reply that failed the check the transport was always
//! going to make against the declared type, which is `failedValidation` under this crate's own
//! vocabulary instead.
//!
//! # The wire's own fault shape
//!
//! A reply's `error` key carries the operation's declared error verbatim, or
//! `{ isServiceFault: true, fault: <FaultFields> }` in its place — the same convention
//! [`super::client`] and [`super::service`] (`ts_client()`/`ts_service()`) already write for an
//! outbound or a dispatcher-detected fault on a bus-style transport.
//!
//! # Headers ride beside the payload
//!
//! `header_in` rides the frame's own `headers` map (`null` sent for `None`, never left out); a
//! `header_out`/`error_header_out` tuple's head rides `value`/`error`, the rest `headers`.
//!
//! # A handler signals its declared error the same way a caller reads it
//!
//! `{Named}Handlers` answers a reply operation with `Future<Success>` and throws the operation's own
//! declared error to signal it — Dart's idiom for a `Future`, the same one `{Named}WsRefusal` still
//! uses on the calling side. Anything else a handler throws is unexpected and reaches `onFault`
//! instead. Whenever the inbound frame carried an id — a caller waiting on a reply, whether the
//! operation is one-way or not — the attachment answers it: `ok: true, value: null` once a
//! one-way handler returns, a fault reply for anything that goes wrong before or during dispatch,
//! so a pending caller is never left hanging.

use super::dart_http_client::message_type;
use super::result::result_name;
use crate::features::dart::{dart_json_decode, dart_json_encode, dart_typename};
use crate::field_type::get_field_def;
use crate::rename_rule::RenameRule;
use crate::service_schema::parse::{
    HttpShape, OperationDef, OperationInputs, OperationOutcome, ServiceDef, is_unit_type,
    option_inner, tuple_elements,
};
use crate::service_schema::support::fault_fields_typescript_name;
use core::fmt::Write as _;
use syn::Type;

/// The structural seam `{Named}WsTransport.frames` publishes and `attach{Named}WsDispatcher`
/// takes: identically spelled for every service, so an attachment for one service composes over
/// any other service's own transport sharing its connection without naming that transport's type.
const FRAMES_RECORD_TYPE: &str =
    "({Stream<Map<String, dynamic>> inbound, void Function(Map<String, dynamic>) send})";

/// What a client-side decode failure names itself with, bundled under clippy's argument limit.
struct FaultNaming<'names> {
    fn_prefix: &'names str,
    result: &'names str,
    wire: &'names str,
}

/// [`FaultNaming`]'s own twin for the attachment side, naming the service rather than a result.
struct DispatchNaming<'names> {
    fn_prefix: &'names str,
    named: &'names str,
    wire: &'names str,
}

pub fn emit(service: &ServiceDef) -> Vec<String> {
    let named = service.ident.to_string();
    let fn_prefix = RenameRule::CamelCase.apply_to_variant(&named);
    let headered = declares_header_in(service);
    let mut published = vec![heartbeat_class(&named), transport_class(&named, headered)];
    if has_one_way(service) {
        published.push(refusal_class(&named));
    }
    published.push(client_class(service, headered));
    published.push(handlers_class(service));
    published.push(attach_dispatcher_fn(service));
    published.extend(fault_helpers(
        &named,
        &fn_prefix,
        declares_any_header(service),
    ));
    published
}

fn has_one_way(service: &ServiceDef) -> bool {
    service
        .operations
        .iter()
        .any(|operation| matches!(operation.outcome, OperationOutcome::OneWay))
}

/// Whether any operation binds `header_in` — what the transport's `headers` parameter is gated on,
/// so a service with none emits exactly the text it did before headers existed.
fn declares_header_in(service: &ServiceDef) -> bool {
    service
        .operations
        .iter()
        .any(|operation| !HttpShape::of(operation).header_in.is_empty())
}

/// [`declares_header_in`]'s own widening to `header_out`/`error_header_out`, for gating the fault
/// helper's own `field` parameter.
fn declares_any_header(service: &ServiceDef) -> bool {
    service.operations.iter().any(|operation| {
        let shape = HttpShape::of(operation);
        !shape.header_in.is_empty()
            || !shape.header_out.is_empty()
            || !shape.error_header_out.is_empty()
    })
}

// ---------------------------------------------------------------------------------------------
// Liveness: how often a transport pings, and how long it waits for the answering pong.
// ---------------------------------------------------------------------------------------------

fn heartbeat_class(named: &str) -> String {
    format!(
        "/// How often a `{named}` `ws_rpc` transport pings the far side, and how long it waits\n\
         /// for the answering pong before treating the connection as dead.\n\
         /// `{named}WsHeartbeat.off()` turns liveness checking off entirely.\n\
         class {named}WsHeartbeat {{\n  \
         const {named}WsHeartbeat({{\n    \
         this.interval = const Duration(seconds: 30),\n    \
         this.timeout = const Duration(seconds: 10),\n  \
         }});\n  \
         const {named}WsHeartbeat.off()\n      \
         : interval = Duration.zero,\n        \
         timeout = Duration.zero;\n  \
         final Duration interval;\n  \
         final Duration timeout;\n  \
         bool get _off => interval == Duration.zero;\n\
         }}"
    )
}

// ---------------------------------------------------------------------------------------------
// The transport: one `stream.listen`, request/reply correlation, the heartbeat, and the fan-out
// an attachment hangs off.
// ---------------------------------------------------------------------------------------------

fn transport_class(named: &str, headered: bool) -> String {
    format!(
        "{header}{request_and_notify}{rest}",
        header = transport_class_header(named),
        request_and_notify = transport_request_and_notify_methods(named, headered),
        rest = transport_class_rest(named),
    )
}

/// [`transport_class`]'s own opening: the doc, the constructor, and the fields.
fn transport_class_header(named: &str) -> String {
    format!(
        "/// A `{named}` `ws_rpc` transport over the sink and stream a `WebSocketChannel` already\n\
         /// exposes. `heartbeat` left `null` takes the default; pass `{named}WsHeartbeat.off()` to\n\
         /// turn liveness checking off.\n\
         ///\n\
         /// Owns the one `stream.listen` a non-broadcast `Stream` allows: `{named}WsClient` reads a\n\
         /// reply back off `request`, and an attachment reaches every other inbound frame through\n\
         /// `frames` instead of listening a second time.\n\
         class {named}WsTransport {{\n  \
         {named}WsTransport({{\n    \
         required StreamSink<dynamic> sink,\n    \
         required Stream<dynamic> stream,\n    \
         {named}WsHeartbeat? heartbeat,\n  \
         }})  : _sink = sink,\n        \
         _heartbeat = heartbeat ?? const {named}WsHeartbeat() {{\n    \
         _subscription = stream.listen(_onFrame, onDone: _onClose, onError: (_) => _onClose());\n    \
         _schedulePing();\n  \
         }}\n\n  \
         final StreamSink<dynamic> _sink;\n  \
         final {named}WsHeartbeat _heartbeat;\n  \
         final Map<String, Completer<Map<String, dynamic>?>> _pending = {{}};\n  \
         final StreamController<Map<String, dynamic>> _controller =\n      \
         StreamController<Map<String, dynamic>>.broadcast();\n  \
         late final StreamSubscription<dynamic> _subscription;\n  \
         Timer? _pingTimer;\n  \
         Timer? _pongTimer;\n  \
         int _nextId = 1;\n  \
         bool _closed = false;\n\n  \
         "
    )
}

/// [`transport_class`]'s own `request`/`notify`, with a `headers` parameter only where the
/// service declares `header_in` somewhere.
fn transport_request_and_notify_methods(named: &str, headered: bool) -> String {
    if !headered {
        return format!(
            "/// Sends `operation` with `payload` as a `request` frame and answers with the matching\n  \
             /// `reply` frame's own fields, or `null` once the connection closes before one arrives.\n  \
             Future<Map<String, dynamic>?> request(String operation, Object? payload) {{\n    \
             final id = '${{_nextId++}}';\n    \
             final completer = Completer<Map<String, dynamic>?>();\n    \
             _pending[id] = completer;\n    \
             send({{\n      \
             'kind': 'request',\n      \
             'id': id,\n      \
             'service': '{named}',\n      \
             'operation': operation,\n      \
             'payload': payload,\n    \
             }});\n    \
             return completer.future;\n  \
             }}\n\n  \
             /// Sends `operation` with `payload` as a `notify` frame. No reply is expected.\n  \
             Future<void> notify(String operation, Object? payload) async {{\n    \
             send({{'kind': 'notify', 'service': '{named}', 'operation': operation, 'payload': payload}});\n  \
             }}\n\n  \
             "
        );
    }
    format!(
        "/// Sends `operation`, `payload` and `headers` as a `request` frame (`headers` written only\n  \
         /// when non-empty) and answers with the matching `reply` frame, or `null` once closed.\n  \
         Future<Map<String, dynamic>?> request(\n    \
         String operation,\n    \
         Object? payload,\n    \
         Map<String, dynamic> headers,\n  \
         ) {{\n    \
         final id = '${{_nextId++}}';\n    \
         final completer = Completer<Map<String, dynamic>?>();\n    \
         _pending[id] = completer;\n    \
         send({{\n      \
         'kind': 'request',\n      \
         'id': id,\n      \
         'service': '{named}',\n      \
         'operation': operation,\n      \
         'payload': payload,\n      \
         if (headers.isNotEmpty) 'headers': headers,\n    \
         }});\n    \
         return completer.future;\n  \
         }}\n\n  \
         /// Sends `operation`, `payload` and `headers` as a `notify` frame. No reply is expected.\n  \
         Future<void> notify(\n    \
         String operation,\n    \
         Object? payload,\n    \
         Map<String, dynamic> headers,\n  \
         ) async {{\n    \
         send({{\n      \
         'kind': 'notify',\n      \
         'service': '{named}',\n      \
         'operation': operation,\n      \
         'payload': payload,\n      \
         if (headers.isNotEmpty) 'headers': headers,\n    \
         }});\n  \
         }}\n\n  \
         "
    )
}

/// [`transport_class`]'s own close: `send`, the inbound frame reader, the heartbeat, and `close`.
fn transport_class_rest(named: &str) -> String {
    format!(
        "/// Writes one frame straight onto the sink.\n  \
         void send(Map<String, dynamic> frame) {{\n    \
         _sink.add(jsonEncode(frame));\n  \
         }}\n\n  \
         /// The structural seam an attachment dispatches over: every inbound frame this transport\n  \
         /// does not itself correlate to a pending `request` (a ping and a pong included\n  \
         /// nowhere), paired with the function that writes a frame back. Every `{named}WsTransport`\n  \
         /// exposes the same shape, so an attachment for any service sharing this connection\n  \
         /// reaches it here rather than through a transport type of its own.\n  \
         {FRAMES_RECORD_TYPE} get frames => (inbound: _controller.stream, send: send);\n\n  \
         void _onFrame(dynamic raw) {{\n    \
         final Map<String, dynamic> frame;\n    \
         try {{\n      \
         frame = jsonDecode(raw as String) as Map<String, dynamic>;\n    \
         }} catch (_) {{\n      \
         return;\n    \
         }}\n    \
         switch (frame['kind']) {{\n      \
         case 'pong':\n        \
         _pongTimer?.cancel();\n        \
         return;\n      \
         case 'ping':\n        \
         send({{'kind': 'pong'}});\n        \
         return;\n      \
         case 'reply':\n        \
         final id = frame['id'];\n        \
         final completer = id is String ? _pending.remove(id) : null;\n        \
         if (completer != null) {{\n          \
         completer.complete(frame);\n          \
         return;\n        \
         }}\n        \
         _controller.add(frame);\n        \
         return;\n      \
         default:\n        \
         _controller.add(frame);\n    \
         }}\n  \
         }}\n\n  \
         void _onClose() {{\n    \
         if (_closed) return;\n    \
         _closed = true;\n    \
         _pingTimer?.cancel();\n    \
         _pongTimer?.cancel();\n    \
         _subscription.cancel();\n    \
         for (final completer in _pending.values) {{\n      \
         completer.complete(null);\n    \
         }}\n    \
         _pending.clear();\n    \
         _controller.close();\n  \
         }}\n\n  \
         void _schedulePing() {{\n    \
         if (_heartbeat._off) return;\n    \
         _pingTimer = Timer.periodic(_heartbeat.interval, (_) {{\n      \
         send({{'kind': 'ping'}});\n      \
         _pongTimer?.cancel();\n      \
         _pongTimer = Timer(_heartbeat.timeout, _onClose);\n    \
         }});\n  \
         }}\n\n  \
         /// Cancels the subscription and the heartbeat, and settles every pending `request` with\n  \
         /// `null` — a value, not an error — which every waiting `{named}WsClient` method reads\n  \
         /// as the transport-failure fault.\n  \
         void close() => _onClose();\n\
         }}"
    )
}

// ---------------------------------------------------------------------------------------------
// The one exception a client still throws: a one-way method's own fault, having no reply arm to
// carry it through instead. Identical in shape to `dart_http_client`'s own.
// ---------------------------------------------------------------------------------------------

fn refusal_class(named: &str) -> String {
    let fields = fault_fields_typescript_name(named);
    format!(
        "/// What a one-way `{named}` `ws_rpc` method throws when it cannot deliver its `notify`\n\
         /// frame. A one-way operation declares no error, so there is nothing else to throw.\n\
         class {named}WsRefusal implements Exception {{\n  \
         {named}WsRefusal(this.fault);\n  \
         final {fields} fault;\n  \
         @override\n  \
         String toString() =>\n      \
         '{named}WsRefusal: ${{fault.kind}} in `${{fault.operation}}`: ${{fault.detail}}';\n\
         }}"
    )
}

// ---------------------------------------------------------------------------------------------
// The client: one class, one constructor, one method per operation, calling out over the
// transport's own `request`/`notify`.
// ---------------------------------------------------------------------------------------------

fn client_class(service: &ServiceDef, headered: bool) -> String {
    let named = service.ident.to_string();
    let methods = service
        .operations
        .iter()
        .map(|operation| client_method(&named, operation, headered))
        .collect::<Vec<_>>()
        .join("\n\n");
    format!(
        "/// A `{named}` caller over `ws_rpc`.\n\
         class {named}WsClient {{\n  \
         {named}WsClient(this._transport);\n  \
         final {named}WsTransport _transport;\n\n\
         {methods}\n\
         }}"
    )
}

/// A one-way method still answers `Future<void>`; a reply method answers
/// `Future<{Named}{Op}Result>` — [`super::dart_result`]'s own sealed pair — and never throws for a
/// declared error or a fault.
fn return_type(named: &str, operation: &OperationDef) -> String {
    if matches!(operation.outcome, OperationOutcome::OneWay) {
        return "Future<void>".to_owned();
    }
    let result = result_name(named, operation).unwrap();
    format!("Future<{result}>")
}

/// The parameter list a method takes: the message first, then one argument per `header_in`
/// binding — mirrors `dart_http_client`'s own `method_params`.
fn client_method_params(operation: &OperationDef, shape: &HttpShape) -> String {
    let mut params = vec![format!("{} req", message_dart_typename(operation))];
    for header in &shape.header_in {
        params.push(format!("{} {}", dart_type_of(&header.ty), header.parameter));
    }
    params.join(", ")
}

/// One entry per `header_in` binding; unlike `http_rest`, a `None` value is sent as JSON `null`
/// rather than left out — the Rust client encodes every binding unconditionally.
fn header_in_build_stmt(shape: &HttpShape) -> String {
    if shape.header_in.is_empty() {
        return "    const headers = <String, dynamic>{};\n".to_owned();
    }
    let mut entries = String::new();
    for header in &shape.header_in {
        let encoded = dart_json_encode(&header.ty, &header.parameter.to_string(), true);
        let _ = writeln!(entries, "      '{}': {encoded},", header.name);
    }
    format!("    final headers = <String, dynamic>{{\n{entries}    }};\n")
}

fn client_method(named: &str, operation: &OperationDef, headered: bool) -> String {
    let fn_prefix = RenameRule::CamelCase.apply_to_variant(named);
    let shape = HttpShape::of(operation);
    let wire = &operation.wire_name;
    let call = &operation.ts_name;
    let returns = return_type(named, operation);
    let doc = format!("  /// Calls `{wire}` over `ws_rpc`.");
    let sent = message_encode(operation);
    if !headered {
        let params = message_dart_typename(operation);
        return match &operation.outcome {
            OperationOutcome::OneWay => format!(
                "{doc}\n  \
                 {returns} {call}({params} req) async {{\n    \
                 try {{\n      \
                 await _transport.notify('{wire}', {sent});\n    \
                 }} catch (uncarried) {{\n      \
                 throw {named}WsRefusal(_{fn_prefix}WsTransportFailure('{wire}', '$uncarried'));\n    \
                 }}\n  \
                 }}"
            ),
            OperationOutcome::Reply {
                error: _error,
                success: _success,
            } => {
                let result = result_name(named, operation).unwrap();
                let decode = reply_decode_stmt(named, &result, &fn_prefix, wire, operation, &shape);
                format!(
                    "{doc}\n  \
                     {returns} {call}({params} req) async {{\n    \
                     final Map<String, dynamic>? reply;\n    \
                     try {{\n      \
                     reply = await _transport.request('{wire}', {sent});\n    \
                     }} catch (uncarried) {{\n      \
                     return {result}Fault(_{fn_prefix}WsTransportFailure('{wire}', '$uncarried'));\n    \
                     }}\n    \
                     if (reply == null) {{\n      \
                     return {result}Fault(\n        \
                     _{fn_prefix}WsTransportFailure('{wire}', 'the connection closed before a reply arrived'),\n      \
                     );\n    \
                     }}\n\
{decode}\
                     }}"
                )
            }
        };
    }
    let params = client_method_params(operation, &shape);
    let headers_build = header_in_build_stmt(&shape);
    match &operation.outcome {
        OperationOutcome::OneWay => format!(
            "{doc}\n  \
             {returns} {call}({params}) async {{\n\
{headers_build}    \
             try {{\n      \
             await _transport.notify('{wire}', {sent}, headers);\n    \
             }} catch (uncarried) {{\n      \
             throw {named}WsRefusal(_{fn_prefix}WsTransportFailure('{wire}', '$uncarried'));\n    \
             }}\n  \
             }}"
        ),
        OperationOutcome::Reply {
            error: _error,
            success: _success,
        } => {
            let result = result_name(named, operation).unwrap();
            let decode = reply_decode_stmt(named, &result, &fn_prefix, wire, operation, &shape);
            format!(
                "{doc}\n  \
                 {returns} {call}({params}) async {{\n\
{headers_build}    \
                 final Map<String, dynamic>? reply;\n    \
                 try {{\n      \
                 reply = await _transport.request('{wire}', {sent}, headers);\n    \
                 }} catch (uncarried) {{\n      \
                 return {result}Fault(_{fn_prefix}WsTransportFailure('{wire}', '$uncarried'));\n    \
                 }}\n    \
                 if (reply == null) {{\n      \
                 return {result}Fault(\n        \
                 _{fn_prefix}WsTransportFailure('{wire}', 'the connection closed before a reply arrived'),\n      \
                 );\n    \
                 }}\n\
{decode}\
                 }}"
            )
        }
    }
}

/// Reads the reply's own `ok`/`value`/`error` keys: the declared success on `true`, the wire's own
/// fault shape (`error['isServiceFault']`) or the declared error otherwise — mirrors
/// `dart_http_client`'s own `reply_decode_stmt`, over the reply frame rather than a status ladder.
fn reply_decode_stmt(
    named: &str,
    result: &str,
    fn_prefix: &str,
    wire: &str,
    operation: &OperationDef,
    shape: &HttpShape,
) -> String {
    let OperationOutcome::Reply { error, success } = &operation.outcome else {
        return String::new();
    };
    let fields = fault_fields_typescript_name(named);
    let success_block = success_decode_block(result, fn_prefix, wire, success, &shape.header_out);
    let error_block =
        declared_error_decode_stmts(result, fn_prefix, wire, error, &shape.error_header_out);
    format!(
        "    if (reply['ok'] == true) {{\n{success_block}    }}\n    \
         final error = reply['error'];\n    \
         if (error is Map<String, dynamic> && error['isServiceFault'] == true) {{\n      \
         late final {fields} fault;\n      \
         try {{\n        \
         fault = {fields}.fromJson(error['fault'] as Map<String, dynamic>);\n      \
         }} catch (rejected) {{\n        \
         return {result}Fault(_{fn_prefix}WsFailedValidation('{wire}', '$rejected'));\n      \
         }}\n      \
         return {result}Fault(fault);\n    \
         }}\n\
{error_block}"
    )
}

/// One `late final` local per header element, decoded off `headers_expr` (or `null` if absent).
/// `margin` is the leading whitespace, since the two call sites nest at different depths.
fn header_value_decode_stmts(
    naming: &FaultNaming<'_>,
    names: &[String],
    element_types: &[&Type],
    headers_expr: &str,
    local_prefix: &str,
    margin: &str,
) -> (String, Vec<String>) {
    let FaultNaming {
        fn_prefix,
        result,
        wire,
    } = naming;
    let mut stmt = String::new();
    let mut locals = Vec::new();
    for (index, (name, ty)) in names.iter().zip(element_types).enumerate() {
        let local = format!("{local_prefix}{index}");
        let dart_ty = dart_type_of(ty);
        let decode = dart_json_decode(ty, &format!("{headers_expr}?['{name}']"));
        let _ = write!(
            stmt,
            "{margin}late final {dart_ty} {local};\n\
             {margin}try {{\n\
             {margin}  {local} = {decode};\n\
             {margin}}} catch (rejected) {{\n\
             {margin}  return {result}Fault(\n\
             {margin}    _{fn_prefix}WsFailedValidation('{wire}', '$rejected', field: '{name}'),\n\
             {margin}  );\n\
             {margin}}}\n"
        );
        locals.push(local);
    }
    (stmt, locals)
}

/// The response alone, or the response plus every `header_out` element read off the reply's own
/// `headers` — mirrors `dart_http_client`'s own `success_decode_block`.
fn success_decode_block(
    result: &str,
    fn_prefix: &str,
    wire: &str,
    success: &Type,
    header_out: &[String],
) -> String {
    if header_out.is_empty() {
        if is_unit_type(success) {
            return format!("      return {result}Ok();\n");
        }
        let decoded = dart_json_decode(success, "reply['value']");
        return format!(
            "      try {{\n        \
             return {result}Ok({decoded});\n      \
             }} catch (rejected) {{\n        \
             return {result}Fault(_{fn_prefix}WsFailedValidation('{wire}', '$rejected'));\n      \
             }}\n"
        );
    }
    // A `header_out` binding's own arity check guarantees `success` is a tuple of exactly this
    // many elements, so the lookups below never miss.
    let elements: Vec<&Type> = tuple_elements(success).into_iter().flatten().collect();
    let body = elements.first().copied().unwrap_or(success);
    let body_ty = dart_type_of(body);
    let decoded = dart_json_decode(body, "reply['value']");
    let mut stmt = format!(
        "      final replyHeaders = reply['headers'] as Map<String, dynamic>?;\n      \
         late final {body_ty} value;\n      \
         try {{\n        \
         value = {decoded};\n      \
         }} catch (rejected) {{\n        \
         return {result}Fault(_{fn_prefix}WsFailedValidation('{wire}', '$rejected'));\n      \
         }}\n"
    );
    let element_types: Vec<&Type> = elements.iter().skip(1).copied().collect();
    let naming = FaultNaming {
        fn_prefix,
        result,
        wire,
    };
    let (header_stmts, header_idents) = header_value_decode_stmts(
        &naming,
        header_out,
        &element_types,
        "replyHeaders",
        "headerOut",
        "      ",
    );
    stmt.push_str(&header_stmts);
    let _ = writeln!(
        stmt,
        "      return {result}Ok((value, {}));",
        header_idents.join(", ")
    );
    stmt
}

/// [`success_decode_block`]'s own twin for the declared error and `error_header_out`.
fn declared_error_decode_stmts(
    result: &str,
    fn_prefix: &str,
    wire: &str,
    error: &Type,
    error_header_out: &[String],
) -> String {
    if error_header_out.is_empty() {
        let error_ty = dart_type_of(error);
        let decoded = dart_json_decode(error, "error");
        return format!(
            "    late final {error_ty} declared;\n    \
             try {{\n      \
             declared = {decoded};\n    \
             }} catch (rejected) {{\n      \
             return {result}Fault(_{fn_prefix}WsFailedValidation('{wire}', '$rejected'));\n    \
             }}\n    \
             return {result}Operation(declared);\n"
        );
    }
    let elements: Vec<&Type> = tuple_elements(error).into_iter().flatten().collect();
    let head = elements.first().copied().unwrap_or(error);
    let head_ty = dart_type_of(head);
    let decoded = dart_json_decode(head, "error");
    let mut stmt = format!(
        "    final replyHeaders = reply['headers'] as Map<String, dynamic>?;\n    \
         late final {head_ty} declaredHead;\n    \
         try {{\n      \
         declaredHead = {decoded};\n    \
         }} catch (rejected) {{\n      \
         return {result}Fault(_{fn_prefix}WsFailedValidation('{wire}', '$rejected'));\n    \
         }}\n"
    );
    let element_types: Vec<&Type> = elements.iter().skip(1).copied().collect();
    let naming = FaultNaming {
        fn_prefix,
        result,
        wire,
    };
    let (header_stmts, header_idents) = header_value_decode_stmts(
        &naming,
        error_header_out,
        &element_types,
        "replyHeaders",
        "errorHeaderOut",
        "    ",
    );
    stmt.push_str(&header_stmts);
    let _ = writeln!(
        stmt,
        "    return {result}Operation((declaredHead, {}));",
        header_idents.join(", ")
    );
    stmt
}

// ---------------------------------------------------------------------------------------------
// Handlers: what an app implementing this service answers inbound frames with, and the
// attachment that dispatches them.
// ---------------------------------------------------------------------------------------------

/// A handler's own signature: context, message, one argument per `header_in` binding, its
/// declared success out — a reply operation signals its declared error by throwing it.
fn handler_signature(operation: &OperationDef, shape: &HttpShape) -> String {
    let req_ty = message_dart_typename(operation);
    let mut params = vec!["Ctx ctx".to_owned(), format!("{req_ty} req")];
    for header in &shape.header_in {
        params.push(format!("{} {}", dart_type_of(&header.ty), header.parameter));
    }
    let joined = params.join(", ");
    match &operation.outcome {
        OperationOutcome::OneWay => format!("Future<void> Function({joined})"),
        OperationOutcome::Reply {
            success,
            error: _error,
        } => {
            let ret = if is_unit_type(success) {
                "void".to_owned()
            } else {
                dart_type_of(success)
            };
            format!("Future<{ret}> Function({joined})")
        }
    }
}

fn handlers_class(service: &ServiceDef) -> String {
    let named = service.ident.to_string();
    let mut params = String::new();
    let mut fields = String::new();
    for operation in &service.operations {
        let shape = HttpShape::of(operation);
        let call = &operation.ts_name;
        let signature = handler_signature(operation, &shape);
        let _ = writeln!(params, "    required this.{call},");
        let _ = writeln!(fields, "  final {signature} {call};");
    }
    format!(
        "/// What a `{named}` `ws_rpc` attachment dispatches an inbound frame to: one handler per\n\
         /// declared operation.\n\
         class {named}Handlers<Ctx> {{\n  \
         {named}Handlers({{\n\
{params}  \
         }});\n\n\
{fields}\
         }}"
    )
}

/// Writes a reply frame back over `frames.send`, guarded on `replyId != null`. `headers_expr`,
/// where given, is written under `headers` only when non-empty.
fn reply_write_stmt(
    named: &str,
    key: &str,
    ok: &str,
    value_expr: &str,
    headers_expr: Option<&str>,
) -> String {
    let headers_line = headers_expr.map_or_else(String::new, |expr| {
        format!("              if ({expr}.isNotEmpty) 'headers': {expr},\n")
    });
    format!(
        "          if (replyId != null) {{\n            \
         frames.send({{\n              \
         'kind': 'reply',\n              \
         'id': replyId,\n              \
         'service': '{named}',\n              \
         'ok': {ok},\n              \
         '{key}': {value_expr},\n\
{headers_line}            \
         }});\n          \
         }}\n"
    )
}

/// Splits a header-tuple `record` into its head and a `Map` of the rest under their own header
/// names, a `null` element omitted — or, with no declared headers, encodes `ty` whole.
fn split_reply_record(
    record: &str,
    head_local: &str,
    headers_local: &str,
    ty: &Type,
    header_names: &[String],
) -> (String, String, Option<String>) {
    if header_names.is_empty() {
        return (String::new(), dart_json_encode(ty, record, true), None);
    }
    // A `header_out`/`error_header_out` binding's own arity check guarantees `ty` is a tuple of
    // exactly this many elements, so the lookups below never miss.
    let elements: Vec<&Type> = tuple_elements(ty).into_iter().flatten().collect();
    let body = elements.first().copied().unwrap_or(ty);
    let element_types: Vec<&Type> = elements.iter().skip(1).copied().collect();
    let idents: Vec<String> = (0..header_names.len())
        .map(|index| format!("{headers_local}{index}"))
        .collect();
    let mut stmt = format!(
        "            final ({head_local}, {}) = {record};\n            \
         final {headers_local} = <String, dynamic>{{}};\n",
        idents.join(", ")
    );
    for ((name, element_ty), ident) in header_names.iter().zip(&element_types).zip(&idents) {
        stmt.push_str(&header_out_push_stmt(
            headers_local,
            name,
            ident,
            element_ty,
        ));
    }
    let encoded = dart_json_encode(body, head_local, true);
    (stmt, encoded, Some(headers_local.to_owned()))
}

/// One push into a header map: unconditional, or guarded on non-`null` for an `Option` element.
fn header_out_push_stmt(local: &str, name: &str, ident: &str, ty: &Type) -> String {
    option_inner(ty).map_or_else(
        || {
            format!(
                "            {local}['{name}'] = {};\n",
                dart_json_encode(ty, ident, true)
            )
        },
        |inner| {
            format!(
                "            if ({ident} != null) {{\n              \
                 {local}['{name}'] = {};\n            \
                 }}\n",
                dart_json_encode(inner, ident, true)
            )
        },
    )
}

fn attach_dispatcher_fn(service: &ServiceDef) -> String {
    let named = service.ident.to_string();
    let fn_prefix = RenameRule::CamelCase.apply_to_variant(&named);
    let fields = fault_fields_typescript_name(&named);
    let arms = service
        .operations
        .iter()
        .map(|operation| dispatch_arm(&named, &fn_prefix, operation))
        .collect::<Vec<_>>()
        .join("\n");
    let mut written = format!(
        "/// Attaches `handlers` to `frames`, dispatching every inbound `{named}` frame, decoded\n\
         /// through each declared type's own codec, to its own handler. A frame this cannot\n\
         /// dispatch, or a handler raising anything but its own declared error, reaches `onFault`\n\
         /// rather than vanishing; a request left waiting on either is answered with the fault\n\
         /// instead of hanging. Answers with the function that detaches it.\n\
         void Function() attach{named}WsDispatcher<Ctx>(\n  \
         {FRAMES_RECORD_TYPE} frames,\n  \
         Ctx ctx,\n  \
         {named}Handlers<Ctx> handlers, {{\n  \
         required void Function({fields}) onFault,\n\
         }}) {{\n  \
         final subscription = frames.inbound.listen((frame) async {{\n    \
         if (frame['service'] != '{named}') return;\n    \
         final rawId = frame['id'];\n    \
         final replyId = rawId is String ? rawId : null;\n    \
         switch (frame['operation']) {{\n\
{arms}\n      \
         default:\n        \
         {{\n          \
         final fault = _{fn_prefix}WsUnknownOperation(\n            \
         '${{frame['operation']}}',\n            \
         'this service answers to no operation by that name',\n          \
         );\n          \
         onFault(fault);\n"
    );
    written.push_str(&reply_write_stmt(
        &named,
        "error",
        "false",
        "{'isServiceFault': true, 'fault': fault.toJson()}",
        None,
    ));
    written.push_str(
        "          }\n      \
         }\n    \
         });\n  \
         return () {\n    \
         subscription.cancel();\n  \
         };\n\
         }",
    );
    written
}

/// Decodes each `header_in` binding off the frame's own `headers`, faulting under the header's
/// own name on a miss. Answers the statements and the bound locals, for the handler call.
fn header_in_read_stmts(
    named: &str,
    fn_prefix: &str,
    wire: &str,
    shape: &HttpShape,
) -> (String, Vec<String>) {
    if shape.header_in.is_empty() {
        return (String::new(), Vec::new());
    }
    let mut stmt =
        "          final headersIn = frame['headers'] as Map<String, dynamic>?;\n".to_owned();
    let mut idents = Vec::new();
    for header in &shape.header_in {
        let name = &header.name;
        let parameter = header.parameter.to_string();
        let dart_ty = dart_type_of(&header.ty);
        let decode = dart_json_decode(&header.ty, &format!("headersIn?['{name}']"));
        let _ = write!(
            stmt,
            "          final {dart_ty} {parameter};\n          \
             try {{\n            \
             {parameter} = {decode};\n          \
             }} catch (rejected) {{\n            \
             final fault = _{fn_prefix}WsFailedValidation(\n              \
             '{wire}',\n              \
             '$rejected',\n              \
             field: '{name}',\n            \
             );\n            \
             onFault(fault);\n"
        );
        stmt.push_str(&reply_write_stmt(
            named,
            "error",
            "false",
            "{'isServiceFault': true, 'fault': fault.toJson()}",
            None,
        ));
        stmt.push_str("            return;\n          }\n");
        idents.push(parameter);
    }
    (stmt, idents)
}

/// The `, local` arguments a handler call appends after `decoded`.
fn header_call_args(header_locals: &[String]) -> String {
    header_locals.iter().fold(String::new(), |mut args, local| {
        let _ = write!(args, ", {local}");
        args
    })
}

fn dispatch_arm(named: &str, fn_prefix: &str, operation: &OperationDef) -> String {
    let shape = HttpShape::of(operation);
    let wire = &operation.wire_name;
    let call = &operation.ts_name;
    let req_ty = message_dart_typename(operation);
    let received = message_decode(operation, "frame['payload']");
    let mut arm = format!(
        "      case '{wire}':\n        \
         {{\n          \
         final {req_ty} decoded;\n          \
         try {{\n            \
         decoded = {received};\n          \
         }} catch (rejected) {{\n            \
         final fault = _{fn_prefix}WsFailedValidation('{wire}', '$rejected');\n            \
         onFault(fault);\n"
    );
    arm.push_str(&reply_write_stmt(
        named,
        "error",
        "false",
        "{'isServiceFault': true, 'fault': fault.toJson()}",
        None,
    ));
    arm.push_str("            return;\n          }\n");
    let (header_reads, header_locals) = header_in_read_stmts(named, fn_prefix, wire, &shape);
    arm.push_str(&header_reads);
    let header_args = header_call_args(&header_locals);
    match &operation.outcome {
        OperationOutcome::OneWay => {
            arm.push_str(&one_way_dispatch_arm(
                named,
                fn_prefix,
                wire,
                call,
                &header_args,
            ));
        }
        OperationOutcome::Reply { error, success } => {
            let naming = DispatchNaming {
                fn_prefix,
                named,
                wire,
            };
            arm.push_str(&reply_dispatch_arm(
                &naming,
                call,
                &header_args,
                &shape,
                success,
                error,
            ));
        }
    }
    arm
}

/// [`dispatch_arm`]'s own `OneWay` arm.
fn one_way_dispatch_arm(
    named: &str,
    fn_prefix: &str,
    wire: &str,
    call: &str,
    header_args: &str,
) -> String {
    let mut arm = format!(
        "          try {{\n            \
         await handlers.{call}(ctx, decoded{header_args});\n"
    );
    arm.push_str(&reply_write_stmt(named, "value", "true", "null", None));
    let _ = write!(
        arm,
        "          }} catch (unexpected) {{\n            \
         final fault = _{fn_prefix}WsHandlerPanic('{wire}', '$unexpected');\n            \
         onFault(fault);\n"
    );
    arm.push_str(&reply_write_stmt(
        named,
        "error",
        "false",
        "{'isServiceFault': true, 'fault': fault.toJson()}",
        None,
    ));
    arm.push_str("          }\n          return;\n        }\n");
    arm
}

/// [`dispatch_arm`]'s own `Reply` arm: splits a header-tuple success or declared error apart.
fn reply_dispatch_arm(
    naming: &DispatchNaming<'_>,
    call: &str,
    header_args: &str,
    shape: &HttpShape,
    success: &Type,
    error: &Type,
) -> String {
    let DispatchNaming {
        fn_prefix,
        named,
        wire,
    } = naming;
    let error_ty = dart_type_of(error);
    let unit = is_unit_type(success);
    let call_stmt = if unit {
        format!("await handlers.{call}(ctx, decoded{header_args});\n")
    } else {
        format!("final answered = await handlers.{call}(ctx, decoded{header_args});\n")
    };
    let mut arm = format!(
        "          try {{\n            \
         {call_stmt}"
    );
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
    arm.push_str(&value_pre);
    arm.push_str(&reply_write_stmt(
        named,
        "value",
        "true",
        &value_expr,
        value_headers.as_deref(),
    ));
    let _ = writeln!(arm, "          }} on {error_ty} catch (declared) {{");
    let (error_pre, error_expr, error_headers) = split_reply_record(
        "declared",
        "declaredHead",
        "errorHeadersOut",
        error,
        &shape.error_header_out,
    );
    arm.push_str(&error_pre);
    arm.push_str(&reply_write_stmt(
        named,
        "error",
        "false",
        &error_expr,
        error_headers.as_deref(),
    ));
    let _ = write!(
        arm,
        "          }} catch (unexpected) {{\n            \
         final fault = _{fn_prefix}WsHandlerPanic('{wire}', '$unexpected');\n            \
         onFault(fault);\n"
    );
    arm.push_str(&reply_write_stmt(
        named,
        "error",
        "false",
        "{'isServiceFault': true, 'fault': fault.toJson()}",
        None,
    ));
    arm.push_str("          }\n          return;\n        }\n");
    arm
}

// ---------------------------------------------------------------------------------------------
// The faults every method and every dispatch arm reaches for.
// ---------------------------------------------------------------------------------------------

fn fault_helpers(named: &str, fn_prefix: &str, headered: bool) -> Vec<String> {
    let failed_validation_doc = format!(
        "The fault a `{named}` `ws_rpc` reply answers with when it will not become the \
         operation's own declared type through that type's own codec."
    );
    vec![
        fault_helper(
            named,
            fn_prefix,
            "WsTransportFailure",
            "transportFailure",
            &format!(
                "The fault a `{named}` `ws_rpc` client answers with when the transport could not \
                 carry a call: the frame never went out, or the reply never came back."
            ),
        ),
        if headered {
            failed_validation_fault_helper(named, fn_prefix)
        } else {
            fault_helper(
                named,
                fn_prefix,
                "WsFailedValidation",
                "failedValidation",
                &failed_validation_doc,
            )
        },
        fault_helper(
            named,
            fn_prefix,
            "WsUnknownOperation",
            "unknownOperation",
            &format!(
                "The fault a `{named}` `ws_rpc` attachment answers with when an inbound frame \
                 names an operation nothing on this service declares."
            ),
        ),
        fault_helper(
            named,
            fn_prefix,
            "WsHandlerPanic",
            "handlerPanic",
            &format!(
                "The fault a `{named}` `ws_rpc` attachment answers with when a handler raises \
                 anything other than its own declared error."
            ),
        ),
    ]
}

fn fault_helper(named: &str, fn_prefix: &str, suffix: &str, kind: &str, doc: &str) -> String {
    let fields = fault_fields_typescript_name(named);
    format!(
        "/// {doc}\n\
         {fields} _{fn_prefix}{suffix}(String operation, String detail) => {fields}(\n  \
         detail: detail,\n  \
         kind: {named}FaultKind.{kind},\n  \
         operation: operation,\n\
         );"
    )
}

/// [`fault_helper`]'s own twin, emitted only where the service declares a header: `field` names a
/// failed header element's own decode, `null` everywhere else.
fn failed_validation_fault_helper(named: &str, fn_prefix: &str) -> String {
    let fields = fault_fields_typescript_name(named);
    format!(
        "/// The fault a `{named}` `ws_rpc` reply answers with when it will not become the\n\
         /// operation's own declared type through that type's own codec.\n\
         {fields} _{fn_prefix}WsFailedValidation(\n  \
         String operation,\n  \
         String detail, {{\n  \
         String? field,\n\
         }}) => {fields}(\n  \
         detail: detail,\n  \
         field: field,\n  \
         kind: {named}FaultKind.failedValidation,\n  \
         operation: operation,\n\
         );"
    )
}

// ---------------------------------------------------------------------------------------------
// Small, Dart-flavored value rendering, duplicated from `dart_http_client` rather than shared
// with it: this module needs none of its header/query/body-kind machinery, only a type's name.
// ---------------------------------------------------------------------------------------------

/// The message's Dart type: the type the operation named, or the one the macro declared for an
/// operation that named none — mirrors `dart_http_client`'s own `message_dart_typename`.
/// The message `req` encoded for a frame's payload.
fn message_encode(operation: &OperationDef) -> String {
    message_type(operation).map_or_else(
        || "req.toJson()".to_owned(),
        |ty| dart_json_encode(&ty, "req", true),
    )
}

/// The message decoded out of a frame's dynamically typed `payload`.
fn message_decode(operation: &OperationDef, payload: &str) -> String {
    let req_ty = message_dart_typename(operation);
    message_type(operation).map_or_else(
        || format!("{req_ty}.fromJson({payload} as Map<String, dynamic>)"),
        |ty| dart_json_decode(&ty, payload),
    )
}

fn message_dart_typename(operation: &OperationDef) -> String {
    match &operation.inputs {
        OperationInputs::Named(declared) => dart_type_of(declared),
        OperationInputs::Empty | OperationInputs::Generated(_) => {
            operation.generated_message_ident().map_or_else(
                || "dynamic".to_owned(),
                |ident| {
                    let named: Type = syn::parse_quote! { #ident };
                    dart_type_of(&named)
                },
            )
        }
    }
}

/// `ty`'s own Dart type name, read through the same `FieldDef` walk every field's type goes
/// through — mirrors `dart_http_client`'s own `dart_type_of`.
fn dart_type_of(ty: &Type) -> String {
    dart_typename(&get_field_def("value", ty, ""))
}
