//! The Kotlin `ws_rpc` client: a transport that owns the socket, a per-operation client that
//! calls out over it, and a dispatcher attachment for a service the app implements — one module
//! holding both halves, mirroring [`super::dart_ws_client`] over Kotlin's own coroutine and
//! `kotlinx.serialization` idioms rather than Dart's `Future`/`Stream` pair.
//!
//! # The transport owns the socket
//!
//! `{Service}WsSocket` exposes exactly one `onMessage` and one `onClose` slot, and the transport's
//! constructor is the only place either is set. A second service sharing the connection cannot
//! set them a second time, so it reaches every frame the transport does not itself correlate to a
//! pending `request` through `{Service}WsFrames` instead — a structural record of the raw text to
//! answer over (`send`), the uncorrelated frames (`inbound`), and the transport's own
//! `CoroutineScope`. Handing out that scope, rather than a fresh one per attachment, is what lets
//! closing the transport tear every dispatcher reading `frames` down with it, shared ones
//! included: cancelling a `CoroutineScope` cancels every coroutine launched on it.
//!
//! # A caller reads the outcome, exactly as the `http_rest` Kotlin client does
//!
//! A reply operation answers `{Service}{Operation}Result` — the same sealed type
//! [`super::kotlin_http_client`] already publishes for it — and never throws for a declared error
//! or a fault; a one-way operation still answers plainly and throws the fault-only
//! `{Service}WsRefusal`, named apart from `kotlin_http_client()`'s own bare `{Service}Refusal` so
//! the two coexist in one file.
//!
//! # A handler answers the sealed result; it does not throw the declared error
//!
//! `{Service}Handlers` answers a reply operation with the same `{Service}{Operation}Result` the
//! client reads, narrowed on `Ok`/`Declared`/`Fault` in the dispatch arm rather than caught off a
//! thrown type — Kotlin has no throwable a plain `@Serializable` declared error could be without
//! this crate inventing a wrapper type nothing else needs. Anything a handler throws regardless is
//! unexpected and reaches `onFault` as a `handler-panic` fault instead. Whenever the inbound frame
//! carried an id — a caller waiting on a reply, whether the operation is one-way or not — the
//! attachment answers it, so a pending caller is never left hanging.
//!
//! # A decode failure is a failed-validation fault, not an undeserializable-payload one
//!
//! `ws_rpc` has no status to distinguish an unreadable reply from a rejected one, mirroring
//! `dart_ws_client`'s own reasoning: a reply that will not decode is answered as `failedValidation`
//! under this crate's own vocabulary.
//!
//! # The wire's own fault shape
//!
//! A reply's `error` key carries the operation's declared error verbatim, or
//! `{ "isServiceFault": true, "fault": <fault fields> }` in its place — the convention every other
//! surface in this crate already writes for an outbound or a dispatcher-detected fault.

use super::result::result_name;
use crate::features::kotlin::kotlin_typename;
use crate::field_type::get_field_def;
use crate::rename_rule::RenameRule;
use crate::service_schema::parse::{
    HttpShape, OperationDef, OperationInputs, OperationOutcome, ServiceDef, is_unit_type,
    option_inner, tuple_elements,
};
use crate::service_schema::support::fault_fields_typescript_name;
use core::fmt::Write as _;
use syn::Type;

/// One `header_out`/`error_header_out` entry: its local identifier (`headerOut0`, ...), its wire
/// name, and its declared type.
struct HeaderElement {
    kotlin_prop: String,
    ty: Type,
    wire_name: String,
}

pub fn emit(service: &ServiceDef) -> Vec<String> {
    let named = service.ident.to_string();
    let fn_prefix = RenameRule::CamelCase.apply_to_variant(&named);
    let has_header_in = declares_header_in(service);
    let needs_field = has_header_in || declares_header_out_or_error(service);
    let mut published = vec![
        socket_interface(&named),
        options_class(&named),
        frames_class(&named),
    ];
    if has_one_way(service) {
        published.push(refusal_class(&named));
    }
    published.push(transport_class(&named, has_header_in));
    published.push(client_class(service, &named, &fn_prefix, has_header_in));
    published.push(handlers_interface(service, &named));
    published.push(attachment_class(&named));
    published.push(attach_dispatcher_fn(&named, &fn_prefix, has_header_in));
    published.push(dispatch_fn(service, &named, &fn_prefix, has_header_in));
    published.extend(fault_helpers(&named, &fn_prefix, needs_field));
    published
}

fn has_one_way(service: &ServiceDef) -> bool {
    service
        .operations
        .iter()
        .any(|operation| matches!(operation.outcome, OperationOutcome::OneWay))
}

/// Whether any operation binds a `header_in` value — gates the transport's `headers` channel.
fn declares_header_in(service: &ServiceDef) -> bool {
    service
        .operations
        .iter()
        .any(|operation| !HttpShape::of(operation).header_in.is_empty())
}

/// Whether any operation declares `header_out` or `error_header_out` — the other half of what
/// earns the failed-validation helper its `field` parameter.
fn declares_header_out_or_error(service: &ServiceDef) -> bool {
    service.operations.iter().any(|operation| {
        let shape = HttpShape::of(operation);
        !shape.header_out.is_empty() || !shape.error_header_out.is_empty()
    })
}

// ---------------------------------------------------------------------------------------------
// The socket seam, the heartbeat options, and the structural record an attachment dispatches
// over.
// ---------------------------------------------------------------------------------------------

fn socket_interface(named: &str) -> String {
    format!(
        "/// The seam a `{named}` `ws_rpc` transport owns: the only place `onMessage` and\n\
         /// `onClose` are ever set, so a real socket satisfies this once and the transport takes\n\
         /// it from there.\n\
         interface {named}WsSocket {{\n  \
         fun send(text: String)\n  \
         fun close()\n  \
         var onMessage: ((String) -> Unit)?\n  \
         var onClose: (() -> Unit)?\n\
         }}"
    )
}

fn options_class(named: &str) -> String {
    format!(
        "/// How often a `{named}` `ws_rpc` transport pings the far side, and how long it waits\n\
         /// for the answering pong before closing the connection. `heartbeat = null` turns\n\
         /// probing off entirely.\n\
         data class {named}WsOptions(\n  \
         val heartbeat: Heartbeat? = Heartbeat(intervalMs = 30_000, timeoutMs = 10_000),\n\
         ) {{\n  \
         data class Heartbeat(val intervalMs: Long, val timeoutMs: Long)\n\
         }}"
    )
}

fn frames_class(named: &str) -> String {
    format!(
        "/// The structural seam a `{named}` `ws_rpc` transport publishes and\n\
         /// `attach{named}WsDispatcher` takes: every inbound frame the transport does not itself\n\
         /// correlate to a pending `request` (a ping and a pong included nowhere), the raw-text\n\
         /// sender to answer over, and the transport's own scope — so closing the transport\n\
         /// cancels this attachment, and every one it shared the socket with, along with it.\n\
         data class {named}WsFrames(\n  \
         val inbound: SharedFlow<String>,\n  \
         val send: (String) -> Unit,\n  \
         val scope: CoroutineScope,\n\
         )"
    )
}

// ---------------------------------------------------------------------------------------------
// The one exception a client still throws: a one-way method's own fault, having no reply arm to
// carry it through instead. Named apart from `kotlin_http_client`'s own bare `{named}Refusal` so
// both coexist in one bundle.
// ---------------------------------------------------------------------------------------------

fn refusal_class(named: &str) -> String {
    let fields = fault_fields_typescript_name(named);
    format!(
        "/// What a one-way `{named}` `ws_rpc` method throws when it cannot deliver its `notify`\n\
         /// frame. A one-way operation declares no error, so there is nothing else to throw.\n\
         class {named}WsRefusal(val fault: {fields}) : Exception(fault.detail)"
    )
}

// ---------------------------------------------------------------------------------------------
// The transport: correlates a `request` to its reply, answers an inbound ping, runs the
// heartbeat, and hands every other frame to `frames`.
// ---------------------------------------------------------------------------------------------

fn transport_class(named: &str, has_headers: bool) -> String {
    format!(
        "{header}{request}{notify_and_send}{on_frame}{heartbeat}{close}\n\
         }}",
        header = transport_header_stmt(named),
        request = transport_request_stmt(named, has_headers),
        notify_and_send = transport_notify_and_send_stmt(named, has_headers),
        on_frame = transport_on_frame_stmt(),
        heartbeat = transport_heartbeat_stmt(named),
        close = transport_close_stmt(named),
    )
}

/// The class's own doc, its constructor, the fields every other piece reaches for, `init`, and the
/// `frames` seam an attachment dispatches over.
fn transport_header_stmt(named: &str) -> String {
    format!(
        "/// A `{named}` `ws_rpc` transport that owns `socket` under the preferred ownership\n\
         /// shape: correlates a `request` to its reply through a `CompletableDeferred`, answers\n\
         /// an inbound ping with one pong, and runs its own heartbeat on a scope it owns and\n\
         /// hands out through `frames` — closing this transport cancels that scope, and with it\n\
         /// every dispatcher reading `frames`, shared ones included.\n\
         ///\n\
         /// `request` runs on its caller's coroutine, `onFrame` on whatever thread the socket\n\
         /// delivers on, and the heartbeat and `close` on the transport's own scope — so every\n\
         /// read and write of `pending`, `nextId` and `pongJob` is guarded by `synchronized(this)`,\n\
         /// a call to the socket or a pending deferred never made while holding it.\n\
         class {named}WsTransport(\n  \
         private val socket: {named}WsSocket,\n  \
         private val options: {named}WsOptions = {named}WsOptions(),\n\
         ) {{\n  \
         private val scope = CoroutineScope(SupervisorJob() + Dispatchers.Default)\n  \
         private val pending = mutableMapOf<String, CompletableDeferred<JsonObject?>>()\n  \
         private val uncorrelated = MutableSharedFlow<String>(extraBufferCapacity = 64)\n  \
         private var nextId = 1\n  \
         private var pongJob: Job? = null\n  \
         private var closed = false\n\n  \
         init {{\n    \
         socket.onMessage = {{ text -> onFrame(text) }}\n    \
         socket.onClose = {{ close() }}\n    \
         scheduleHeartbeat()\n  \
         }}\n\n  \
         /// The structural seam an attachment dispatches over. Every `{named}WsTransport`\n  \
         /// exposes the same shape, so an attachment for any service sharing this socket reaches\n  \
         /// it here rather than through a transport type of its own.\n  \
         val frames: {named}WsFrames\n    \
         get() = {named}WsFrames(uncorrelated.asSharedFlow(), socket::send, scope)\n\n  "
    )
}

/// Assigns the next id and registers the pending deferred under one lock, then sends the request
/// frame and awaits the reply outside it.
fn transport_request_stmt(named: &str, has_headers: bool) -> String {
    if has_headers {
        return format!(
            "/// Sends `operation` with `payload` and `headers` as a `request` frame and answers\n  \
             /// with the matching `reply` frame's own fields, or `null` once the connection closes\n  \
             /// before one arrives.\n  \
             suspend fun request(operation: String, payload: JsonElement?, headers: JsonObject): JsonObject? {{\n    \
             val deferred = CompletableDeferred<JsonObject?>()\n    \
             val id = synchronized(this) {{\n      \
             val assigned = (nextId++).toString()\n      \
             pending[assigned] = deferred\n      \
             assigned\n    \
             }}\n    \
             send(\n      \
             buildJsonObject {{\n        \
             put(\"kind\", \"request\")\n        \
             put(\"id\", id)\n        \
             put(\"service\", \"{named}\")\n        \
             put(\"operation\", operation)\n        \
             put(\"payload\", payload ?: JsonNull)\n        \
             if (headers.isNotEmpty()) put(\"headers\", headers)\n      \
             }},\n    \
             )\n    \
             return deferred.await()\n  \
             }}\n\n  "
        );
    }
    format!(
        "/// Sends `operation` with `payload` as a `request` frame and answers with the matching\n  \
         /// `reply` frame's own fields, or `null` once the connection closes before one arrives.\n  \
         suspend fun request(operation: String, payload: JsonElement?): JsonObject? {{\n    \
         val deferred = CompletableDeferred<JsonObject?>()\n    \
         val id = synchronized(this) {{\n      \
         val assigned = (nextId++).toString()\n      \
         pending[assigned] = deferred\n      \
         assigned\n    \
         }}\n    \
         send(\n      \
         buildJsonObject {{\n        \
         put(\"kind\", \"request\")\n        \
         put(\"id\", id)\n        \
         put(\"service\", \"{named}\")\n        \
         put(\"operation\", operation)\n        \
         put(\"payload\", payload ?: JsonNull)\n      \
         }},\n    \
         )\n    \
         return deferred.await()\n  \
         }}\n\n  "
    )
}

fn transport_notify_and_send_stmt(named: &str, has_headers: bool) -> String {
    if has_headers {
        return format!(
            "/// Sends `operation` with `payload` and `headers` as a `notify` frame. No reply is\n  \
             /// expected.\n  \
             fun notify(operation: String, payload: JsonElement?, headers: JsonObject) {{\n    \
             send(\n      \
             buildJsonObject {{\n        \
             put(\"kind\", \"notify\")\n        \
             put(\"service\", \"{named}\")\n        \
             put(\"operation\", operation)\n        \
             put(\"payload\", payload ?: JsonNull)\n        \
             if (headers.isNotEmpty()) put(\"headers\", headers)\n      \
             }},\n    \
             )\n  \
             }}\n\n  \
             private fun send(frame: JsonObject) {{\n    \
             socket.send(frame.toString())\n  \
             }}\n\n  "
        );
    }
    format!(
        "/// Sends `operation` with `payload` as a `notify` frame. No reply is expected.\n  \
         fun notify(operation: String, payload: JsonElement?) {{\n    \
         send(\n      \
         buildJsonObject {{\n        \
         put(\"kind\", \"notify\")\n        \
         put(\"service\", \"{named}\")\n        \
         put(\"operation\", operation)\n        \
         put(\"payload\", payload ?: JsonNull)\n      \
         }},\n    \
         )\n  \
         }}\n\n  \
         private fun send(frame: JsonObject) {{\n    \
         socket.send(frame.toString())\n  \
         }}\n\n  "
    )
}

/// Reads a frame off the socket: a pong cancels the outstanding watcher under one lock, a ping
/// draws one pong, and a reply either settles a pending deferred (removed under one lock) or,
/// unmatched, reaches `frames.inbound` beside every frame this service does not itself correlate.
fn transport_on_frame_stmt() -> String {
    "private fun onFrame(text: String) {\n    \
     val frame = try {\n      \
     Json.parseToJsonElement(text).jsonObject\n    \
     } catch (malformed: Throwable) {\n      \
     return\n    \
     }\n    \
     when ((frame[\"kind\"] as? JsonPrimitive)?.contentOrNull) {\n      \
     \"pong\" -> {\n        \
     val watcher = synchronized(this) {\n          \
     val current = pongJob\n          \
     pongJob = null\n          \
     current\n        \
     }\n        \
     watcher?.cancel()\n      \
     }\n      \
     \"ping\" -> send(buildJsonObject { put(\"kind\", \"pong\") })\n      \
     \"reply\" -> {\n        \
     val id = (frame[\"id\"] as? JsonPrimitive)?.contentOrNull\n        \
     val deferred = id?.let { key -> synchronized(this) { pending.remove(key) } }\n        \
     if (deferred != null) deferred.complete(frame) else uncorrelated.tryEmit(text)\n      \
     }\n      \
     else -> uncorrelated.tryEmit(text)\n    \
     }\n  \
     }\n\n  "
        .to_owned()
}

/// Arms the pong watcher under one lock before sending the ping — never after, which would let a
/// pong that answers faster than the watcher is assigned find nothing to cancel — and cancels
/// whatever watcher the previous round left outside the lock.
fn transport_heartbeat_stmt(named: &str) -> String {
    format!(
        "private fun scheduleHeartbeat() {{\n    \
         val heartbeat = options.heartbeat ?: return\n    \
         scope.launch {{\n      \
         while (isActive) {{\n        \
         delay(heartbeat.intervalMs)\n        \
         val previous = synchronized(this@{named}WsTransport) {{\n          \
         val current = pongJob\n          \
         pongJob = scope.launch {{\n            \
         delay(heartbeat.timeoutMs)\n            \
         close()\n          \
         }}\n          \
         current\n        \
         }}\n        \
         previous?.cancel()\n        \
         send(buildJsonObject {{ put(\"kind\", \"ping\") }})\n      \
         }}\n    \
         }}\n  \
         }}\n\n  "
    )
}

fn transport_close_stmt(named: &str) -> String {
    format!(
        "/// Closes the socket and settles every pending `request` with `null` — a value, not an\n  \
         /// error — which every waiting `{named}WsClient` method reads as the transport-failure\n  \
         /// fault, then cancels the scope `frames` hands out, tearing every dispatcher reading it\n  \
         /// down along with it.\n  \
         fun close() {{\n    \
         if (closed) return\n    \
         closed = true\n    \
         socket.close()\n    \
         val watcher = synchronized(this) {{ pongJob }}\n    \
         watcher?.cancel()\n    \
         val settled = synchronized(this) {{\n      \
         val waiting = pending.values.toList()\n      \
         pending.clear()\n      \
         waiting\n    \
         }}\n    \
         for (deferred in settled) deferred.complete(null)\n    \
         scope.cancel()\n  \
         }}"
    )
}

// ---------------------------------------------------------------------------------------------
// The client: one class, one constructor, one method per operation, calling out over the
// transport's own `request`/`notify`.
// ---------------------------------------------------------------------------------------------

fn client_class(service: &ServiceDef, named: &str, fn_prefix: &str, has_headers: bool) -> String {
    let methods = service
        .operations
        .iter()
        .map(|operation| client_method(named, fn_prefix, operation, has_headers))
        .collect::<Vec<_>>()
        .join("\n\n");
    format!(
        "/// A `{named}` caller over `ws_rpc`.\n\
         class {named}WsClient(private val transport: {named}WsTransport) {{\n\
         {methods}\n\
         }}"
    )
}

/// `req` plus one parameter per `header_in` binding.
fn client_method_params(shape: &HttpShape, req_ty: &str) -> String {
    let mut params = vec![format!("req: {req_ty}")];
    for header in &shape.header_in {
        params.push(format!(
            "{}: {}",
            kotlin_property(&header.parameter.to_string()),
            kotlin_type_of(&header.ty)
        ));
    }
    params.join(", ")
}

/// One entry per `header_in` binding, each JSON-encoded; `None` sent as JSON `null` rather than
/// omitted, mirroring the Rust client's own unconditional `outbound_headers`.
fn client_headers_build_stmt(shape: &HttpShape) -> String {
    if shape.header_in.is_empty() {
        return "    val headers = buildJsonObject {}\n".to_owned();
    }
    let mut stmt = String::from("    val headers = buildJsonObject {\n");
    for header in &shape.header_in {
        let prop = kotlin_property(&header.parameter.to_string());
        let name = &header.name;
        if let Some(inner) = option_inner(&header.ty) {
            let inner_ty = kotlin_type_of(inner);
            let _ = writeln!(
                stmt,
                "      put(\"{name}\", {prop}?.let {{ \
                 Json.encodeToJsonElement(serializer<{inner_ty}>(), it) }} ?: JsonNull)"
            );
        } else {
            let ty = kotlin_type_of(&header.ty);
            let _ = writeln!(
                stmt,
                "      put(\"{name}\", Json.encodeToJsonElement(serializer<{ty}>(), {prop}))"
            );
        }
    }
    stmt.push_str("    }\n");
    stmt
}

fn client_method(
    named: &str,
    fn_prefix: &str,
    operation: &OperationDef,
    has_headers: bool,
) -> String {
    let wire = &operation.wire_name;
    let call = &operation.ts_name;
    let req_ty = message_kotlin_typename(operation);
    let shape = HttpShape::of(operation);
    let params = client_method_params(&shape, &req_ty);
    let doc = format!("  /// Calls `{wire}` over `ws_rpc`.");
    let (headers_build, headers_arg) = if has_headers {
        (client_headers_build_stmt(&shape), ", headers")
    } else {
        (String::new(), "")
    };
    match &operation.outcome {
        OperationOutcome::OneWay => format!(
            "{doc}\n  \
             suspend fun {call}({params}) {{\n\
{headers_build}    \
             try {{\n      \
             transport.notify(\"{wire}\", Json.encodeToJsonElement(serializer<{req_ty}>(), req){headers_arg})\n    \
             }} catch (uncarried: Throwable) {{\n      \
             throw {named}WsRefusal({fn_prefix}WsTransportFailure(\"{wire}\", uncarried.toString()))\n    \
             }}\n  \
             }}"
        ),
        OperationOutcome::Reply { error, success } => {
            let result = result_name(named, operation).unwrap();
            let decode = reply_decode_stmt(named, fn_prefix, wire, &result, &shape, error, success);
            format!(
                "{doc}\n  \
                 suspend fun {call}({params}): {result} {{\n\
{headers_build}    \
                 val reply = try {{\n      \
                 transport.request(\"{wire}\", Json.encodeToJsonElement(serializer<{req_ty}>(), req){headers_arg})\n    \
                 }} catch (uncarried: Throwable) {{\n      \
                 return {result}.Fault({fn_prefix}WsTransportFailure(\"{wire}\", uncarried.toString()))\n    \
                 }}\n    \
                 if (reply == null) {{\n      \
                 return {result}.Fault(\n        \
                 {fn_prefix}WsTransportFailure(\"{wire}\", \"the connection closed before a reply arrived\"),\n      \
                 )\n    \
                 }}\n\
{decode}\
                 }}"
            )
        }
    }
}

/// Reads the reply's own `ok`/`value`/`error` keys: the declared success on `true`, the wire's
/// own fault shape (`error.isServiceFault`) or the declared error otherwise.
fn reply_decode_stmt(
    named: &str,
    fn_prefix: &str,
    wire: &str,
    result: &str,
    shape: &HttpShape,
    error: &Type,
    success: &Type,
) -> String {
    let fields = fault_fields_typescript_name(named);
    let success_block = success_decode_block(fn_prefix, wire, result, shape, success);
    let error_block = error_decode_block(fn_prefix, wire, result, shape, error);
    format!(
        "    val ok = (reply[\"ok\"] as? JsonPrimitive)?.booleanOrNull ?: false\n    \
         if (ok) {{\n{success_block}    }}\n    \
         val error = reply[\"error\"]\n    \
         val isServiceFault = (error as? JsonObject)?.get(\"isServiceFault\")\n      \
         ?.let {{ (it as? JsonPrimitive)?.booleanOrNull }} ?: false\n    \
         if (isServiceFault) {{\n      \
         return try {{\n        \
         {result}.Fault(\n          \
         Json.decodeFromJsonElement(serializer<{fields}>(), error.getValue(\"fault\")),\n        \
         )\n      \
         }} catch (rejected: Throwable) {{\n        \
         {result}.Fault({fn_prefix}WsFailedValidation(\"{wire}\", rejected.toString()))\n      \
         }}\n    \
         }}\n\
{error_block}"
    )
}

fn success_decode_block(
    fn_prefix: &str,
    wire: &str,
    result: &str,
    shape: &HttpShape,
    success: &Type,
) -> String {
    if is_unit_type(success) {
        return format!("      return {result}.Ok\n");
    }
    if shape.header_out.is_empty() {
        let success_ty = kotlin_type_of(success);
        return format!(
            "      return try {{\n        \
             {result}.Ok(Json.decodeFromJsonElement(serializer<{success_ty}>(), reply.getValue(\"value\")))\n      \
             }} catch (rejected: Throwable) {{\n        \
             {result}.Fault({fn_prefix}WsFailedValidation(\"{wire}\", rejected.toString()))\n      \
             }}\n"
        );
    }
    let extra = header_elements(&shape.header_out, success, "headerOut");
    let body_ty = kotlin_type_of(body_type(shape.header_out.len(), success));
    let mut stmt = format!(
        "      val headers = (reply[\"headers\"] as? JsonObject) ?: buildJsonObject {{}}\n      \
         val value = try {{\n        \
         Json.decodeFromJsonElement(serializer<{body_ty}>(), reply.getValue(\"value\"))\n      \
         }} catch (rejected: Throwable) {{\n        \
         return {result}.Fault({fn_prefix}WsFailedValidation(\"{wire}\", rejected.toString()))\n      \
         }}\n"
    );
    let (header_stmts, idents) =
        header_out_decode_stmts(result, fn_prefix, wire, "headers", "      ", &extra);
    stmt.push_str(&header_stmts);
    let _ = writeln!(
        stmt,
        "      return {result}.Ok({result}Value(value, {}))",
        idents.join(", ")
    );
    stmt
}

fn error_decode_block(
    fn_prefix: &str,
    wire: &str,
    result: &str,
    shape: &HttpShape,
    error: &Type,
) -> String {
    if shape.error_header_out.is_empty() {
        let error_ty = kotlin_type_of(error);
        return format!(
            "    return try {{\n      \
             {result}.Declared(Json.decodeFromJsonElement(serializer<{error_ty}>(), error ?: JsonNull))\n    \
             }} catch (rejected: Throwable) {{\n      \
             {result}.Fault({fn_prefix}WsFailedValidation(\"{wire}\", rejected.toString()))\n    \
             }}\n"
        );
    }
    let extra = header_elements(&shape.error_header_out, error, "errorHeaderOut");
    let head_ty = kotlin_type_of(body_type(shape.error_header_out.len(), error));
    let mut stmt = format!(
        "    val declaredHeaders = (reply[\"headers\"] as? JsonObject) ?: buildJsonObject {{}}\n    \
         val declaredHead = try {{\n      \
         Json.decodeFromJsonElement(serializer<{head_ty}>(), error ?: JsonNull)\n    \
         }} catch (rejected: Throwable) {{\n      \
         return {result}.Fault({fn_prefix}WsFailedValidation(\"{wire}\", rejected.toString()))\n    \
         }}\n"
    );
    let (header_stmts, idents) =
        header_out_decode_stmts(result, fn_prefix, wire, "declaredHeaders", "    ", &extra);
    stmt.push_str(&header_stmts);
    let _ = writeln!(
        stmt,
        "    return {result}.Declared({result}Declared(declaredHead, {}))",
        idents.join(", ")
    );
    stmt
}

/// Decodes each of `extra` off `headers_var`: an `Option<T>` element reads a missing header as
/// `null`; a required one, missing or undecodable, answers `{result}.Fault` naming the header.
fn header_out_decode_stmts(
    result: &str,
    fn_prefix: &str,
    wire: &str,
    headers_var: &str,
    margin: &str,
    extra: &[HeaderElement],
) -> (String, Vec<String>) {
    let mut stmt = String::new();
    let mut idents = Vec::new();
    for field in extra {
        let raw_ident = format!("{}Raw", field.kotlin_prop);
        let header_name = &field.wire_name;
        let prop = &field.kotlin_prop;
        if let Some(inner) = option_inner(&field.ty) {
            let inner_ty = kotlin_type_of(inner);
            let _ = write!(
                stmt,
                "{margin}val {raw_ident} = {headers_var}[\"{header_name}\"]\n\
                 {margin}val {prop} = {raw_ident}?.let {{ raw ->\n\
                 {margin}  try {{\n\
                 {margin}    Json.decodeFromJsonElement(serializer<{inner_ty}>(), raw)\n\
                 {margin}  }} catch (rejected: Throwable) {{\n\
                 {margin}    return {result}.Fault(\n\
                 {margin}      {fn_prefix}WsFailedValidation(\"{wire}\", rejected.toString(), field = \"{header_name}\"),\n\
                 {margin}    )\n\
                 {margin}  }}\n\
                 {margin}}}\n",
            );
        } else {
            let ty = kotlin_type_of(&field.ty);
            let _ = write!(
                stmt,
                "{margin}val {raw_ident} = {headers_var}[\"{header_name}\"]\n\
                 {margin}  ?: return {result}.Fault(\n\
                 {margin}    {fn_prefix}WsFailedValidation(\n\
                 {margin}      \"{wire}\",\n\
                 {margin}      \"a declared response header was missing\",\n\
                 {margin}      field = \"{header_name}\",\n\
                 {margin}    ),\n\
                 {margin}  )\n\
                 {margin}val {prop} = try {{\n\
                 {margin}  Json.decodeFromJsonElement(serializer<{ty}>(), {raw_ident})\n\
                 {margin}}} catch (rejected: Throwable) {{\n\
                 {margin}  return {result}.Fault(\n\
                 {margin}    {fn_prefix}WsFailedValidation(\"{wire}\", rejected.toString(), field = \"{header_name}\"),\n\
                 {margin}  )\n\
                 {margin}}}\n",
            );
        }
        idents.push(field.kotlin_prop.clone());
    }
    (stmt, idents)
}

// ---------------------------------------------------------------------------------------------
// Handlers: what an app implementing this service answers inbound frames with, and the
// attachment that dispatches them.
// ---------------------------------------------------------------------------------------------

/// `req` plus one parameter per `header_in` binding, mirroring [`client_method_params`].
fn handler_params(operation: &OperationDef, shape: &HttpShape) -> String {
    let req_ty = message_kotlin_typename(operation);
    let mut params = vec![format!("req: {req_ty}")];
    for header in &shape.header_in {
        params.push(format!(
            "{}: {}",
            kotlin_property(&header.parameter.to_string()),
            kotlin_type_of(&header.ty)
        ));
    }
    params.join(", ")
}

fn handler_member(named: &str, operation: &OperationDef) -> String {
    let call = &operation.ts_name;
    let shape = HttpShape::of(operation);
    let params = handler_params(operation, &shape);
    result_name(named, operation).map_or_else(
        || format!("  suspend fun {call}({params})"),
        |result| format!("  suspend fun {call}({params}): {result}"),
    )
}

fn handlers_interface(service: &ServiceDef, named: &str) -> String {
    let members = service
        .operations
        .iter()
        .map(|operation| handler_member(named, operation))
        .collect::<Vec<_>>()
        .join("\n");
    format!(
        "/// What a `{named}` `ws_rpc` attachment dispatches an inbound frame to: one handler per\n\
         /// declared operation. A reply operation answers the same sealed result the client\n\
         /// reads rather than throwing its declared error; anything a handler throws regardless\n\
         /// reaches `onFault` instead.\n\
         interface {named}Handlers {{\n\
         {members}\n\
         }}"
    )
}

fn attachment_class(named: &str) -> String {
    format!(
        "/// One socket's `{named}` dispatcher: the scope it and every service it shared the\n\
         /// socket with run on, and the hook that shares that socket.\n\
         class {named}WsAttachment internal constructor(\n  \
         val scope: CoroutineScope,\n  \
         private val send: (String) -> Unit,\n  \
         private val job: Job,\n\
         ) {{\n  \
         private val shared = mutableListOf<() -> Unit>()\n\n  \
         /// Detaches every service this attachment shared the socket with, then this one.\n  \
         fun detach() {{\n    \
         shared.toList().forEach {{ it() }}\n    \
         shared.clear()\n    \
         job.cancel()\n  \
         }}\n\n  \
         /// Hands the guarded sender and this attachment's scope to another service's\n  \
         /// dispatcher. The detach it returns also runs when this attachment detaches. Throws\n  \
         /// once this attachment has detached.\n  \
         fun share(attach: (send: (String) -> Unit, scope: CoroutineScope) -> () -> Unit): () -> Unit {{\n    \
         check(job.isActive) {{ \"the socket is closed\" }}\n    \
         val detachShared = attach(send, scope)\n    \
         shared += detachShared\n    \
         return detachShared\n  \
         }}\n\
         }}"
    )
}

fn attach_dispatcher_fn(named: &str, fn_prefix: &str, has_headers: bool) -> String {
    let fields = fault_fields_typescript_name(named);
    let headers_arg = if has_headers {
        ", frame[\"headers\"]"
    } else {
        ""
    };
    format!(
        "/// Attaches `handlers` to `frames`, dispatching every inbound `{named}` frame, decoded\n\
         /// through the generated codec, to its own handler on `frames`' own scope — so closing\n\
         /// the transport this attachment reads from ends it too. A frame naming another\n\
         /// service, or carrying no operation at all, is left on `frames.inbound` for that\n\
         /// service's own attachment; one naming an operation this service does not declare\n\
         /// reaches `onFault` instead. Every frame that carried an id is answered, one-way\n\
         /// operations included, so a caller waiting on a reply is never left hanging.\n\
         fun attach{named}WsDispatcher(\n  \
         frames: {named}WsFrames,\n  \
         handlers: {named}Handlers,\n  \
         onFault: ({fields}) -> Unit,\n\
         ): {named}WsAttachment {{\n  \
         val job = frames.scope.launch {{\n    \
         frames.inbound.collect {{ text ->\n      \
         val frame = runCatching {{ Json.parseToJsonElement(text).jsonObject }}.getOrNull() ?: return@collect\n      \
         if ((frame[\"service\"] as? JsonPrimitive)?.contentOrNull != \"{named}\") return@collect\n      \
         val id = (frame[\"id\"] as? JsonPrimitive)?.contentOrNull\n      \
         val operation = (frame[\"operation\"] as? JsonPrimitive)?.contentOrNull ?: return@collect\n      \
         launch {{\n        \
         {fn_prefix}WsDispatch(id, operation, frame[\"payload\"]{headers_arg}, handlers, frames.send, onFault)\n      \
         }}\n    \
         }}\n  \
         }}\n  \
         return {named}WsAttachment(frames.scope, frames.send, job)\n\
         }}"
    )
}

/// The reply frame `{ok}` writes back over `send`, correlated to the inbound frame's own `id`.
fn reply_frame_expr(named: &str, ok: &str, key: &str, value_expr: &str) -> String {
    format!(
        "buildJsonObject {{ put(\"kind\", \"reply\"); put(\"id\", id); put(\"service\", \"{named}\"); \
         put(\"ok\", {ok}); put(\"{key}\", {value_expr}) }}.toString()"
    )
}

fn fault_envelope_expr(named: &str, fault_expr: &str) -> String {
    let fields = fault_fields_typescript_name(named);
    format!(
        "buildJsonObject {{ put(\"isServiceFault\", true); put(\"fault\", \
         Json.encodeToJsonElement(serializer<{fields}>(), {fault_expr})) }}"
    )
}

/// Answers a waiting request with the fault envelope — guarded on `id != null`, which is `null`
/// only for a `notify` frame.
fn fault_reply_stmt(named: &str, fault_expr: &str) -> String {
    let envelope = fault_envelope_expr(named, fault_expr);
    let frame = reply_frame_expr(named, "false", "error", &envelope);
    format!("        if (id != null) send({frame})\n")
}

fn dispatch_fn(service: &ServiceDef, named: &str, fn_prefix: &str, has_headers: bool) -> String {
    let fields = fault_fields_typescript_name(named);
    let headers_param = if has_headers {
        "  headers: JsonElement?,\n"
    } else {
        ""
    };
    let headers_prelude = if has_headers {
        "  val incomingHeaders = (headers as? JsonObject) ?: buildJsonObject {}\n"
    } else {
        ""
    };
    let arms = service
        .operations
        .iter()
        .map(|operation| dispatch_arm(named, fn_prefix, operation))
        .collect::<Vec<_>>()
        .join("\n");
    let unknown_fault = format!(
        "{fn_prefix}WsUnknownOperation(operation, \"this service answers to no operation by that name\")"
    );
    format!(
        "private suspend fun {fn_prefix}WsDispatch(\n  \
         id: String?,\n  \
         operation: String,\n  \
         payload: JsonElement?,\n\
{headers_param}  \
         handlers: {named}Handlers,\n  \
         send: (String) -> Unit,\n  \
         onFault: ({fields}) -> Unit,\n\
         ) {{\n\
{headers_prelude}  \
         when (operation) {{\n\
{arms}\n    \
         else -> {{\n      \
         val fault = {unknown_fault}\n      \
         onFault(fault)\n\
{unknown_reply}    \
         }}\n  \
         }}\n\
         }}",
        unknown_reply = fault_reply_stmt(named, "fault"),
    )
}

/// Reads each `header_in` binding off `incomingHeaders`, refusing the same way a malformed
/// payload does — mirrors the Rust dispatcher's own `header_in_reads`.
fn header_in_read_stmt(
    named: &str,
    fn_prefix: &str,
    wire: &str,
    shape: &HttpShape,
) -> (String, Vec<String>) {
    let mut stmt = String::new();
    let mut idents = Vec::new();
    for header in &shape.header_in {
        let prop = kotlin_property(&header.parameter.to_string());
        let name = &header.name;
        let raw_ident = format!("{prop}Raw");
        if let Some(inner) = option_inner(&header.ty) {
            let inner_ty = kotlin_type_of(inner);
            let decode_fault = fault_reply_stmt(named, "fault");
            let _ = write!(
                stmt,
                "      val {raw_ident} = incomingHeaders[\"{name}\"]\n      \
                 val {prop} = if ({raw_ident} == null || {raw_ident} is JsonNull) {{\n        \
                 null\n      \
                 }} else {{\n        \
                 try {{\n          \
                 Json.decodeFromJsonElement(serializer<{inner_ty}>(), {raw_ident})\n        \
                 }} catch (rejected: Throwable) {{\n          \
                 val fault = {fn_prefix}WsFailedValidation(\"{wire}\", rejected.toString(), field = \"{name}\")\n          \
                 onFault(fault)\n\
{decode_fault}          \
                 return\n        \
                 }}\n      \
                 }}\n",
            );
        } else {
            let ty = kotlin_type_of(&header.ty);
            let missing_fault = fault_reply_stmt(named, "fault");
            let decode_fault = fault_reply_stmt(named, "fault");
            let _ = write!(
                stmt,
                "      val {raw_ident} = incomingHeaders[\"{name}\"]\n      \
                 if ({raw_ident} == null) {{\n        \
                 val fault = {fn_prefix}WsFailedValidation(\"{wire}\", \"a required header was not carried\", field = \"{name}\")\n        \
                 onFault(fault)\n\
{missing_fault}        \
                 return\n      \
                 }}\n      \
                 val {prop} = try {{\n        \
                 Json.decodeFromJsonElement(serializer<{ty}>(), {raw_ident})\n      \
                 }} catch (rejected: Throwable) {{\n        \
                 val fault = {fn_prefix}WsFailedValidation(\"{wire}\", rejected.toString(), field = \"{name}\")\n        \
                 onFault(fault)\n\
{decode_fault}        \
                 return\n      \
                 }}\n",
            );
        }
        idents.push(prop);
    }
    (stmt, idents)
}

/// The `val replyHeaders = ...` statement a reply arm writes before sending: one `put` per
/// element of `extra`, read off `accessor_root`, a `null` `Option<T>` element omitted.
fn header_write_stmt(extra: &[HeaderElement], accessor_root: &str) -> String {
    let mut stmt = String::from("            val replyHeaders = buildJsonObject {\n");
    for field in extra {
        let accessor = format!("{accessor_root}.{}", field.kotlin_prop);
        let name = &field.wire_name;
        if let Some(inner) = option_inner(&field.ty) {
            let inner_ty = kotlin_type_of(inner);
            let _ = writeln!(
                stmt,
                "              {accessor}?.let {{ put(\"{name}\", \
                 Json.encodeToJsonElement(serializer<{inner_ty}>(), it)) }}"
            );
        } else {
            let ty = kotlin_type_of(&field.ty);
            let _ = writeln!(
                stmt,
                "              put(\"{name}\", Json.encodeToJsonElement(serializer<{ty}>(), {accessor}))"
            );
        }
    }
    stmt.push_str("            }\n");
    stmt
}

/// The `Ok` reply's `replyHeaders` statement and body expression. Called only where `header_out`
/// was declared, so `success` is a tuple.
fn ok_reply_parts(shape: &HttpShape, success: &Type) -> (String, String) {
    let extra = header_elements(&shape.header_out, success, "headerOut");
    let body_ty = kotlin_type_of(body_type(shape.header_out.len(), success));
    let body_expr =
        format!("Json.encodeToJsonElement(serializer<{body_ty}>(), answered.value.value)");
    (header_write_stmt(&extra, "answered.value"), body_expr)
}

/// [`ok_reply_parts`]'s own twin on the declared-error side.
fn declared_reply_parts(shape: &HttpShape, error: &Type) -> (String, String) {
    let extra = header_elements(&shape.error_header_out, error, "errorHeaderOut");
    let head_ty = kotlin_type_of(body_type(shape.error_header_out.len(), error));
    let body_expr =
        format!("Json.encodeToJsonElement(serializer<{head_ty}>(), answered.error.error)");
    (header_write_stmt(&extra, "answered.error"), body_expr)
}

/// The `is {result}.Ok -> ...` arm: the original one-line `send` where the operation declared no
/// `header_out`, else a block that writes `replyHeaders` beside the body.
fn ok_arm(named: &str, result: &str, shape: &HttpShape, success: &Type) -> String {
    if shape.header_out.is_empty() {
        let ok_value = if is_unit_type(success) {
            "JsonNull".to_owned()
        } else {
            let success_ty = kotlin_type_of(success);
            format!("Json.encodeToJsonElement(serializer<{success_ty}>(), answered.value)")
        };
        let ok_frame = reply_frame_expr(named, "true", "value", &ok_value);
        return format!("          is {result}.Ok -> send({ok_frame})\n");
    }
    let (headers_stmt, body_expr) = ok_reply_parts(shape, success);
    format!(
        "          is {result}.Ok -> {{\n\
{headers_stmt}            \
         send(\n              \
         buildJsonObject {{\n                \
         put(\"kind\", \"reply\")\n                \
         put(\"id\", id)\n                \
         put(\"service\", \"{named}\")\n                \
         put(\"ok\", true)\n                \
         put(\"value\", {body_expr})\n                \
         if (replyHeaders.isNotEmpty()) put(\"headers\", replyHeaders)\n              \
         }}.toString(),\n            \
         )\n          \
         }}\n"
    )
}

/// [`ok_arm`]'s own twin on the declared-error side.
fn declared_arm(named: &str, result: &str, shape: &HttpShape, error: &Type) -> String {
    if shape.error_header_out.is_empty() {
        let error_ty = kotlin_type_of(error);
        let declared_frame = reply_frame_expr(
            named,
            "false",
            "error",
            &format!("Json.encodeToJsonElement(serializer<{error_ty}>(), answered.error)"),
        );
        return format!("          is {result}.Declared -> send({declared_frame})\n");
    }
    let (headers_stmt, body_expr) = declared_reply_parts(shape, error);
    format!(
        "          is {result}.Declared -> {{\n\
{headers_stmt}            \
         send(\n              \
         buildJsonObject {{\n                \
         put(\"kind\", \"reply\")\n                \
         put(\"id\", id)\n                \
         put(\"service\", \"{named}\")\n                \
         put(\"ok\", false)\n                \
         put(\"error\", {body_expr})\n                \
         if (replyHeaders.isNotEmpty()) put(\"headers\", replyHeaders)\n              \
         }}.toString(),\n            \
         )\n          \
         }}\n"
    )
}

fn dispatch_arm(named: &str, fn_prefix: &str, operation: &OperationDef) -> String {
    let wire = &operation.wire_name;
    let req_ty = message_kotlin_typename(operation);
    let shape = HttpShape::of(operation);
    let decode_fault = fault_reply_stmt(named, "fault");
    let (header_reads, header_idents) = header_in_read_stmt(named, fn_prefix, wire, &shape);
    let mut arm = format!(
        "    \"{wire}\" -> {{\n      \
         val decoded = try {{\n        \
         Json.decodeFromJsonElement(serializer<{req_ty}>(), payload ?: JsonNull)\n      \
         }} catch (rejected: Throwable) {{\n        \
         val fault = {fn_prefix}WsFailedValidation(\"{wire}\", rejected.toString())\n        \
         onFault(fault)\n\
{decode_fault}        \
         return\n      \
         }}\n\
{header_reads}"
    );
    let mut call_arg_list = vec!["decoded".to_owned()];
    call_arg_list.extend(header_idents);
    let call_args = call_arg_list.join(", ");
    match &operation.outcome {
        OperationOutcome::OneWay => {
            let call = &operation.ts_name;
            let panic_fault = fault_reply_stmt(named, "fault");
            let ack = reply_frame_expr(named, "true", "value", "JsonNull");
            let _ = write!(
                arm,
                "      try {{\n        \
                 handlers.{call}({call_args})\n      \
                 }} catch (unexpected: Throwable) {{\n        \
                 val fault = {fn_prefix}WsHandlerPanic(\"{wire}\", unexpected.toString())\n        \
                 onFault(fault)\n\
{panic_fault}        \
                 return\n      \
                 }}\n      \
                 if (id != null) send({ack})\n    \
                 }}\n"
            );
        }
        OperationOutcome::Reply { error, success } => {
            let call = &operation.ts_name;
            let result = result_name(named, operation).unwrap();
            let panic_fault = fault_reply_stmt(named, "fault");
            let ok_line = ok_arm(named, &result, &shape, success);
            let declared_line = declared_arm(named, &result, &shape, error);
            let fault_envelope = fault_envelope_expr(named, "answered.fault");
            let fault_frame = reply_frame_expr(named, "false", "error", &fault_envelope);
            let _ = write!(
                arm,
                "      val answered = try {{\n        \
                 handlers.{call}({call_args})\n      \
                 }} catch (unexpected: Throwable) {{\n        \
                 val fault = {fn_prefix}WsHandlerPanic(\"{wire}\", unexpected.toString())\n        \
                 onFault(fault)\n\
{panic_fault}        \
                 return\n      \
                 }}\n      \
                 if (id != null) {{\n        \
                 when (answered) {{\n\
{ok_line}{declared_line}          \
                 is {result}.Fault -> send({fault_frame})\n        \
                 }}\n      \
                 }}\n    \
                 }}\n"
            );
        }
    }
    arm
}

// ---------------------------------------------------------------------------------------------
// The faults every method and every dispatch arm reaches for.
// ---------------------------------------------------------------------------------------------

fn fault_helpers(named: &str, fn_prefix: &str, needs_field: bool) -> Vec<String> {
    vec![
        fault_helper(
            named,
            fn_prefix,
            "WsTransportFailure",
            "TransportFailure",
            &format!(
                "The fault a `{named}` `ws_rpc` client answers with when the transport could not \
                 carry a call: the frame never went out, or the reply never came back."
            ),
        ),
        fault_helper_validation(named, fn_prefix, needs_field),
        fault_helper(
            named,
            fn_prefix,
            "WsUnknownOperation",
            "UnknownOperation",
            &format!(
                "The fault a `{named}` `ws_rpc` attachment answers with when an inbound frame \
                 names an operation nothing on this service declares."
            ),
        ),
        fault_helper(
            named,
            fn_prefix,
            "WsHandlerPanic",
            "HandlerPanic",
            &format!(
                "The fault a `{named}` `ws_rpc` attachment answers with when a handler raises \
                 anything at all rather than answering the sealed result."
            ),
        ),
    ]
}

fn fault_helper(named: &str, fn_prefix: &str, suffix: &str, kind: &str, doc: &str) -> String {
    let fields = fault_fields_typescript_name(named);
    format!(
        "/// {doc}\n\
         private fun {fn_prefix}{suffix}(operation: String, detail: String): {fields} = {fields}(\n  \
         detail = detail,\n  \
         kind = {named}FaultKind.{kind},\n  \
         operation = operation,\n\
         )"
    )
}

/// `field` names the header a header-shaped failure is about, and is only worth declaring where
/// a header read or write uses it.
fn fault_helper_validation(named: &str, fn_prefix: &str, needs_field: bool) -> String {
    let fields = fault_fields_typescript_name(named);
    let doc = format!(
        "The fault a `{named}` `ws_rpc` reply answers with when it will not become the \
         operation's own declared type through the generated codec."
    );
    if !needs_field {
        return fault_helper(
            named,
            fn_prefix,
            "WsFailedValidation",
            "FailedValidation",
            &doc,
        );
    }
    format!(
        "/// {doc}\n\
         private fun {fn_prefix}WsFailedValidation(\n  \
         operation: String,\n  \
         detail: String,\n  \
         field: String? = null,\n\
         ): {fields} = {fields}(\n  \
         detail = detail,\n  \
         field = field,\n  \
         kind = {named}FaultKind.FailedValidation,\n  \
         operation = operation,\n\
         )"
    )
}

// ---------------------------------------------------------------------------------------------
// Small, Kotlin-flavored value rendering, duplicated from `kotlin_http_client` rather than shared
// with it: this module needs none of its HTTP-shaped machinery.
// ---------------------------------------------------------------------------------------------

fn kotlin_property(raw: &str) -> String {
    RenameRule::CamelCase.apply_to_field(raw)
}

/// The message's Kotlin type: the type the operation named, or the one the macro declared for an
/// operation that named none.
fn message_kotlin_typename(operation: &OperationDef) -> String {
    match &operation.inputs {
        OperationInputs::Named(declared) => kotlin_type_of(declared),
        OperationInputs::Empty | OperationInputs::Generated(_) => {
            operation.generated_message_ident().map_or_else(
                || "Unit".to_owned(),
                |ident| {
                    let named: Type = syn::parse_quote! { #ident };
                    kotlin_type_of(&named)
                },
            )
        }
    }
}

fn kotlin_type_of(ty: &Type) -> String {
    kotlin_typename(&get_field_def("value", ty, ""))
}

/// The type a header tuple carries in its body slot: the tuple's first element, or `ty` itself
/// where no headers were declared.
fn body_type(headers: usize, ty: &Type) -> &Type {
    if headers == 0 {
        return ty;
    }
    tuple_elements(ty)
        .and_then(|elements| elements.first())
        .unwrap_or(ty)
}

/// `names`' own declared header elements, matched against `whole`'s trailing tuple elements.
fn header_elements(names: &[String], whole: &Type, ident_prefix: &str) -> Vec<HeaderElement> {
    let elements: Vec<&Type> = tuple_elements(whole).into_iter().flatten().collect();
    names
        .iter()
        .zip(elements.iter().skip(1))
        .enumerate()
        .map(|(index, (name, ty))| HeaderElement {
            kotlin_prop: format!("{ident_prefix}{index}"),
            ty: (*ty).clone(),
            wire_name: name.clone(),
        })
        .collect()
}
