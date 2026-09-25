//! The Swift `ws_rpc` client: a socket seam over which one actor correlates requests to their
//! replies, probes liveness on a heartbeat, checks every reply against the operation's own
//! declared type, and settles every waiting call with a `transport-failure` fault when the socket
//! closes — the TypeScript client's feature set, item for item, read at [`super::ws_client`].
//!
//! # One actor, not a transport plus a client
//!
//! The transport is an `actor` rather than a class guarded by a lock: a lock compiles but leaves a
//! forgotten `lock()` to break silently, while an actor has the compiler enforce the isolation.
//! There is also no seam left to cut between "the transport" and "the client": the reply's own
//! success or error type is known only to the
//! per-operation method, so the actor that owns the correlation map is the same actor that decodes
//! the reply. [`transport_actor`] is that one actor, carrying one public method per operation with
//! the REST client's own signatures; [`client_alias`] publishes `{Named}WsClient` as another name
//! for it, so a caller constructs it the way it constructs `{Named}HttpClient`.
//!
//! # Ordering: one stream, not one task per frame
//!
//! A socket's `onMessage` callback fires once per inbound frame, synchronously, in the order
//! frames arrived — but handing each one to the actor through its own freshly spawned `Task`
//! gives Swift no reason to run those tasks in that same order. [`transport_actor`] instead feeds
//! every inbound frame into one `AsyncStream`, fed by `onMessage` alone, and drains it with one
//! `for await` loop bound to the actor: `AsyncStream` delivers what was `yield`ed in the order it
//! was `yield`ed, and a single consuming loop processes one frame to completion before the next is
//! read, so arrival order is preserved end to end without depending on how the runtime happens to
//! schedule unstructured tasks.
//!
//! # The shared failure and refusal types
//!
//! `{Named}{Operation}Failure` and `{Named}Refusal` are declared once, by
//! [`super::swift_http_client`], and named here rather than redeclared — the same operation
//! answers the same failure whichever transport carried the call. What this module still owns is
//! the fault *it* can report on its own: a reply that will not decode, or a call that never
//! finished because the socket closed, built through [`transport_failure_helper`] and
//! [`failed_validation_helper`] and named with a `Ws` infix so a bundle carrying both clients
//! never declares two functions under one name.

use crate::features::swift::swift_reference_type;
use crate::field_type::get_field_def;
use crate::rename_rule::RenameRule;
use crate::service_schema::parse::{
    HttpShape, OperationDef, OperationInputs, OperationOutcome, ServiceDef, is_unit_type,
    option_inner, tuple_elements,
};
use core::fmt::Write as _;
use syn::Type;

use super::swift_type::swift_typename_of;

pub fn emit(service: &ServiceDef) -> Vec<String> {
    let named = service.ident.to_string();
    let prefix = RenameRule::CamelCase.apply_to_variant(&named);
    let has_header_in = declares_header_in(service);
    let mut published = vec![
        socket_type(&named),
        options_type(&named),
        support_types(&named, has_header_in),
    ];
    if has_header_in {
        published.push(header_in_type(&named));
    }
    published.push(transport_failure_helper(&named, &prefix));
    published.push(failed_validation_helper(&named, &prefix));
    let mut aux = Vec::new();
    published.push(transport_actor(
        service,
        &named,
        &prefix,
        has_header_in,
        &mut aux,
    ));
    published.extend(aux);
    published.push(client_alias(&named));
    published
}

/// Whether the service declares an operation carrying at least one `header_in` binding —
/// mirrors `swift_http_client`'s own `declares_header_in`.
fn declares_header_in(service: &ServiceDef) -> bool {
    service
        .operations
        .iter()
        .any(|operation| !HttpShape::of(operation).header_in.is_empty())
}

/// `{Named}{PascalOperation}Failure`, the failure arm every reply method answers with —
/// [`super::swift_http_client`]'s own name, read again here rather than redeclared. `None` for a
/// one-way operation, mirroring [`super::result::result_name`].
fn failure_name(named: &str, operation: &OperationDef) -> Option<String> {
    match &operation.outcome {
        OperationOutcome::OneWay => None,
        OperationOutcome::Reply {
            error: _error,
            success: _success,
        } => Some(format!(
            "{named}{}Failure",
            RenameRule::PascalCase.apply_to_field(&operation.ident.to_string())
        )),
    }
}

/// `{Named}Refusal`, the error every one-way method throws — also [`super::swift_http_client`]'s
/// own name.
fn refusal_name(named: &str) -> String {
    format!("{named}Refusal")
}

/// `{Named}Fault`, the shared fault type both the REST and the `ws_rpc` client answer a defect
/// with — [`super::swift_http_client`]'s own binding over the service's generated
/// `{Named}FaultFields`.
fn fault_name(named: &str) -> String {
    format!("{named}Fault")
}

// ---------------------------------------------------------------------------------------------
// The socket seam and the heartbeat options.
// ---------------------------------------------------------------------------------------------

/// The socket seam: four members, none of them naming a networking type, so an app's own
/// `URLSessionWebSocketTask` wrapper — or anything else that can hand text in and take text out —
/// satisfies it without an adapter.
fn socket_type(named: &str) -> String {
    format!(
        "/// What binds a `{named}` `ws_rpc` transport to a socket, in plain terms: send text \
         out,\n\
         /// close the connection, and hand every inbound text frame and the eventual close back \
         through\n\
         /// the two callbacks. Names no networking type.\n\
         public protocol {named}WsSocket: AnyObject, Sendable {{\n  \
         func send(_ text: String)\n  \
         func close()\n  \
         var onMessage: (@Sendable (String) -> Void)? {{ get set }}\n  \
         var onClose: (@Sendable () -> Void)? {{ get set }}\n\
         }}"
    )
}

/// How often the transport probes the socket, and how long it waits for the answer; `heartbeat`
/// left `nil` turns probing off. Defaults to a 30 second interval and a 10 second timeout.
fn options_type(named: &str) -> String {
    format!(
        "/// How often a `{named}` `ws_rpc` transport probes the socket, and how long it waits \
         for\n\
         /// the answering pong. `heartbeat` left `nil` turns probing off.\n\
         public struct {named}WsOptions: Sendable {{\n  \
         public struct Heartbeat: Sendable {{\n    \
         public var intervalMs: Int\n    \
         public var timeoutMs: Int\n    \
         public init(intervalMs: Int, timeoutMs: Int) {{\n      \
         self.intervalMs = intervalMs\n      \
         self.timeoutMs = timeoutMs\n    \
         }}\n  \
         }}\n  \
         public var heartbeat: Heartbeat?\n  \
         public init(heartbeat: Heartbeat? = Heartbeat(intervalMs: 30_000, timeoutMs: 10_000)) \
         {{\n    \
         self.heartbeat = heartbeat\n  \
         }}\n\
         }}"
    )
}

// ---------------------------------------------------------------------------------------------
// The frame and envelope shapes the actor encodes and decodes through — declared once per
// service, reused by every operation method.
// ---------------------------------------------------------------------------------------------

/// The outbound frame shapes and inbound decode envelopes every operation method shares, plus a
/// bare probe that reads `kind`, `id` and `service` off a frame before its `value`/`error` shape
/// is known. The request and notify frames carry an optional `headers` field only where the
/// service declares `header_in` — left off the wire when `nil`.
fn support_types(named: &str, has_header_in: bool) -> String {
    let fault = fault_name(named);
    let headers_field = if has_header_in {
        format!("\n  let headers: [String: {named}WsHeaderIn]?")
    } else {
        String::new()
    };
    format!(
        "struct {named}WsRequestFrame<Payload: Encodable>: Encodable {{\n  \
         let kind = \"request\"\n  \
         let id: String\n  \
         let service: String\n  \
         let operation: String\n  \
         let payload: Payload{headers_field}\n\
         }}\n\n\
         struct {named}WsNotifyFrame<Payload: Encodable>: Encodable {{\n  \
         let kind = \"notify\"\n  \
         let service: String\n  \
         let operation: String\n  \
         let payload: Payload{headers_field}\n\
         }}\n\n\
         struct {named}WsFrameProbe: Decodable {{\n  \
         let kind: String\n  \
         let id: String?\n  \
         let service: String?\n\
         }}\n\n\
         struct {named}WsOkProbe: Decodable {{\n  \
         let ok: Bool\n\
         }}\n\n\
         struct {named}WsValueEnvelope<Success: Decodable>: Decodable {{\n  \
         let value: Success\n\
         }}\n\n\
         struct {named}WsDeclaredEnvelope<Declared: Decodable>: Decodable {{\n  \
         let error: Declared\n\
         }}\n\n\
         struct {named}WsFaultEnvelope: Decodable {{\n  \
         struct Marker: Decodable {{\n    \
         let isServiceFault: Bool?\n    \
         let fault: {fault}?\n  \
         }}\n  \
         let error: Marker\n\
         }}"
    )
}

/// One outgoing `header_in` value, type-erased so operations with different header types share
/// one dictionary; an `Optional` writes JSON `null` for `nil` rather than omitting the key.
fn header_in_type(named: &str) -> String {
    format!(
        "struct {named}WsHeaderIn: Encodable {{\n  \
         private let write: (Encoder) throws -> Void\n\n  \
         init<Value: Encodable>(_ value: Value) {{\n    \
         write = {{ encoder in try value.encode(to: encoder) }}\n  \
         }}\n\n  \
         func encode(to encoder: Encoder) throws {{\n    \
         try write(encoder)\n  \
         }}\n\
         }}"
    )
}

// ---------------------------------------------------------------------------------------------
// The two faults this transport can report on its own: a reply that would not become the
// operation's own declared shape, and a call that never finished because the socket closed.
// ---------------------------------------------------------------------------------------------

fn transport_failure_helper(named: &str, prefix: &str) -> String {
    let fault = fault_name(named);
    format!(
        "/// The fault a `{named}` `ws_rpc` call answers with when the transport could not carry \
         it:\n\
         /// the frame never went out, or the reply never came back before the socket closed.\n\
         func {prefix}WsTransportFailure(_ operation: String, _ detail: String) -> {fault} {{\n  \
         {fault}(detail: detail, field: nil, kind: .transportFailure, operation: operation)\n\
         }}"
    )
}

fn failed_validation_helper(named: &str, prefix: &str) -> String {
    let fault = fault_name(named);
    format!(
        "/// The fault a `{named}` `ws_rpc` call answers with when a reply will not become the \
         operation's\n\
         /// own declared success or error, or when a payload will not become the wire shape \
         going out.\n\
         func {prefix}WsFailedValidation(_ operation: String, _ detail: String) -> {fault} {{\n  \
         {fault}(detail: detail, field: nil, kind: .failedValidation, operation: operation)\n\
         }}"
    )
}

// ---------------------------------------------------------------------------------------------
// The transport actor.
// ---------------------------------------------------------------------------------------------

fn transport_actor(
    service: &ServiceDef,
    named: &str,
    prefix: &str,
    has_header_in: bool,
    aux: &mut Vec<String>,
) -> String {
    let methods = service
        .operations
        .iter()
        .map(|operation| operation_method(named, prefix, operation, has_header_in, aux))
        .collect::<Vec<_>>()
        .join("\n\n");
    format!(
        "{header}{init}{close}{correlate}{notify}{handle_message}{heartbeat}{handle_close}\n\n\
         {methods}\n\
         }}",
        header = actor_header(named),
        init = actor_init(named),
        close = actor_close_method(),
        correlate = actor_correlate(named, has_header_in),
        notify = actor_send_notify(named, has_header_in),
        handle_message = actor_handle_message(named),
        heartbeat = actor_heartbeat_machinery(),
        handle_close = actor_handle_close(),
    )
}

fn actor_header(named: &str) -> String {
    format!(
        "/// A `{named}` transport over one `ws_rpc` socket, correlating requests to their \
         replies,\n\
         /// probing liveness on its own heartbeat, and settling every call still waiting with a\n\
         /// `transport-failure` fault once the socket closes. One method per operation, with the\n\
         /// REST client's own signatures.\n\
         public actor {named}WsTransport {{\n  \
         private let socket: any {named}WsSocket\n  \
         private let heartbeat: {named}WsOptions.Heartbeat?\n  \
         private var next = 0\n  \
         private var pending: [String: CheckedContinuation<Data?, Never>] = [:]\n  \
         private var pingTask: Task<Void, Never>?\n  \
         private var pongDeadlineTask: Task<Void, Never>?\n  \
         private var inboundContinuation: AsyncStream<String>.Continuation?\n  \
         private var closed = false\n\n"
    )
}

/// Wires the socket to one `AsyncStream` so inbound frames decode in arrival order (a `Task` per
/// frame gives no such guarantee). `self` is captured weakly in both spawned tasks so neither
/// keeps the actor alive past its own last strong reference.
fn actor_init(named: &str) -> String {
    format!(
        "  public init(socket: any {named}WsSocket, options: {named}WsOptions = .init()) {{\n    \
         self.socket = socket\n    \
         self.heartbeat = options.heartbeat\n    \
         let (inbound, outbound) = AsyncStream<String>.makeStream()\n    \
         self.inboundContinuation = outbound\n    \
         socket.onMessage = {{ text in outbound.yield(text) }}\n    \
         socket.onClose = {{ outbound.finish() }}\n    \
         Task {{ [weak self] in\n      \
         for await text in inbound {{\n        \
         await self?.handleMessage(text)\n      \
         }}\n      \
         await self?.handleClose()\n    \
         }}\n    \
         Task {{ [weak self] in await self?.schedulePing() }}\n  \
         }}\n\n"
    )
}

fn actor_close_method() -> String {
    "  public func close() {\n    \
     socket.close()\n    \
     handleClose()\n  \
     }\n\n"
        .to_owned()
}

/// Sends a `request` frame and awaits the matching `reply`'s raw bytes, or `nil` once the socket
/// closes first. Registering the pending continuation and writing the frame happen inside the
/// same non-suspending closure, so no reply for this id can be read before it is recorded. Takes
/// `headers` only where the service declares `header_in`.
fn actor_correlate(named: &str, has_header_in: bool) -> String {
    let (headers_param, headers_arg) = if has_header_in {
        (
            format!("\n    headers: [String: {named}WsHeaderIn]?,"),
            ", headers: headers",
        )
    } else {
        (String::new(), "")
    };
    format!(
        "  private func correlate<Payload: Encodable>(\n    \
         operation: String,\n    \
         payload: Payload,{headers_param}\n  \
         ) async throws -> Data? {{\n    \
         let id = String(next)\n    \
         next += 1\n    \
         let frame = {named}WsRequestFrame(id: id, service: \"{named}\", operation: operation, \
         payload: payload{headers_arg})\n    \
         let data = try JSONEncoder().encode(frame)\n    \
         if closed {{\n      \
         return nil\n    \
         }}\n    \
         let text = String(decoding: data, as: UTF8.self)\n    \
         return await withCheckedContinuation {{ (continuation: CheckedContinuation<Data?, \
         Never>) in\n      \
         pending[id] = continuation\n      \
         socket.send(text)\n    \
         }}\n  \
         }}\n\n"
    )
}

fn actor_send_notify(named: &str, has_header_in: bool) -> String {
    let (headers_param, headers_arg) = if has_header_in {
        (
            format!(", headers: [String: {named}WsHeaderIn]?"),
            ", headers: headers",
        )
    } else {
        (String::new(), "")
    };
    format!(
        "  private func sendNotify<Payload: Encodable>(operation: String, payload: \
         Payload{headers_param}) throws {{\n    \
         let frame = {named}WsNotifyFrame(service: \"{named}\", operation: operation, payload: \
         payload{headers_arg})\n    \
         let data = try JSONEncoder().encode(frame)\n    \
         let text = String(decoding: data, as: UTF8.self)\n    \
         socket.send(text)\n  \
         }}\n\n"
    )
}

/// Dispatches one inbound frame by its `kind`: heartbeat, a matching `reply` resumed for the
/// waiting method to decode, or dropped where nothing here is waiting on it.
fn actor_handle_message(named: &str) -> String {
    format!(
        "  private func handleMessage(_ text: String) async {{\n    \
         guard let data = text.data(using: .utf8) else {{ return }}\n    \
         guard let probe = try? JSONDecoder().decode({named}WsFrameProbe.self, from: data) else \
         {{ return }}\n    \
         switch probe.kind {{\n    \
         case \"ping\":\n      \
         socket.send(\"{{\\\"kind\\\":\\\"pong\\\"}}\")\n    \
         case \"pong\":\n      \
         pongDeadlineTask?.cancel()\n      \
         pongDeadlineTask = nil\n      \
         schedulePing()\n    \
         case \"reply\":\n      \
         guard probe.service == \"{named}\", let id = probe.id, let waiting = \
         pending.removeValue(forKey: id) else {{ return }}\n      \
         waiting.resume(returning: data)\n    \
         default:\n      \
         return\n    \
         }}\n  \
         }}\n\n"
    )
}

fn actor_heartbeat_machinery() -> String {
    "  private func schedulePing() {\n    \
     guard let heartbeat else { return }\n    \
     pingTask?.cancel()\n    \
     let intervalNanoseconds = UInt64(heartbeat.intervalMs) * 1_000_000\n    \
     pingTask = Task { [weak self] in\n      \
     try? await Task.sleep(nanoseconds: intervalNanoseconds)\n      \
     guard !Task.isCancelled else { return }\n      \
     await self?.sendPing()\n    \
     }\n  \
     }\n\n  \
     private func sendPing() {\n    \
     guard !closed, let heartbeat else { return }\n    \
     socket.send(\"{\\\"kind\\\":\\\"ping\\\"}\")\n    \
     pongDeadlineTask?.cancel()\n    \
     let timeoutNanoseconds = UInt64(heartbeat.timeoutMs) * 1_000_000\n    \
     pongDeadlineTask = Task { [weak self] in\n      \
     try? await Task.sleep(nanoseconds: timeoutNanoseconds)\n      \
     guard !Task.isCancelled else { return }\n      \
     await self?.pongMissed()\n    \
     }\n  \
     }\n\n  \
     private func pongMissed() {\n    \
     guard !closed else { return }\n    \
     socket.close()\n    \
     handleClose()\n  \
     }\n\n"
        .to_owned()
}

/// Tears the transport down once. Resumes every request still waiting with `nil` — a value, not
/// a thrown error — which each waiting method reads as the transport-failure fault.
fn actor_handle_close() -> String {
    "  private func handleClose() {\n    \
     guard !closed else { return }\n    \
     closed = true\n    \
     pingTask?.cancel()\n    \
     pongDeadlineTask?.cancel()\n    \
     inboundContinuation?.finish()\n    \
     pingTask = nil\n    \
     pongDeadlineTask = nil\n    \
     let waiting = pending\n    \
     pending.removeAll()\n    \
     for continuation in waiting.values {\n      \
     continuation.resume(returning: nil)\n    \
     }\n  \
     }"
    .to_owned()
}

// ---------------------------------------------------------------------------------------------
// One method per operation.
// ---------------------------------------------------------------------------------------------

fn operation_method(
    named: &str,
    prefix: &str,
    operation: &OperationDef,
    has_header_in: bool,
    aux: &mut Vec<String>,
) -> String {
    match &operation.outcome {
        OperationOutcome::OneWay => one_way_method(named, prefix, operation, has_header_in, aux),
        OperationOutcome::Reply { error, success } => {
            reply_method(named, prefix, operation, error, success, has_header_in, aux)
        }
    }
}

/// One argument per `header_in` binding, after the message — the raw Rust identifier, never
/// re-cased, mirrors `swift_http_client`'s own `method_params`.
fn header_in_params(param_ty: &str, shape: &HttpShape) -> String {
    let mut params = format!("_ req: {param_ty}");
    for header in &shape.header_in {
        let _ = write!(
            params,
            ", {}: {}",
            header.parameter,
            swift_typename_of(&header.ty)
        );
    }
    params
}

/// The outgoing header dictionary, one entry per `header_in` binding, sent unconditionally so an
/// `Option` holding `nil` crosses as JSON `null` rather than being left out.
fn header_in_build_stmt(named: &str, shape: &HttpShape) -> String {
    if shape.header_in.is_empty() {
        return format!("    let headers: [String: {named}WsHeaderIn]? = nil\n");
    }
    let mut entries = String::new();
    for header in &shape.header_in {
        let _ = writeln!(
            entries,
            "      \"{}\": {named}WsHeaderIn({}),",
            header.name, header.parameter
        );
    }
    format!("    let headers: [String: {named}WsHeaderIn]? = [\n{entries}    ]\n")
}

fn one_way_method(
    named: &str,
    prefix: &str,
    operation: &OperationDef,
    has_header_in: bool,
    aux: &mut Vec<String>,
) -> String {
    let call = &operation.ts_name;
    let wire = &operation.wire_name;
    let (param_ty, param_aux) = message_swift_type(operation);
    aux.extend(param_aux);
    let refusal = refusal_name(named);
    let shape = HttpShape::of(operation);
    let params = header_in_params(&param_ty, &shape);
    let (headers_build, headers_arg) = if has_header_in {
        (header_in_build_stmt(named, &shape), ", headers: headers")
    } else {
        (String::new(), "")
    };
    format!(
        "  /// Calls `{wire}` over `ws_rpc`.\n  \
         public func {call}({params}) async throws {{\n\
{headers_build}    \
         do {{\n      \
         try sendNotify(operation: \"{wire}\", payload: req{headers_arg})\n    \
         }} catch {{\n      \
         throw {refusal}(fault: {prefix}WsFailedValidation(\"{wire}\", \"\\(error)\"))\n    \
         }}\n  \
         }}"
    )
}

/// One operation's own reply-headers structure: an absent optional element decodes as `nil`, a
/// missing required one throws, caught by the reply method's own outer `catch` into a fault.
fn header_read_struct(
    type_name: &str,
    names: &[String],
    elements: &[&Type],
    body_elements: usize,
    ident_prefix: &str,
) -> (String, Vec<String>) {
    let entries: Vec<(String, &String, &Type)> = names
        .iter()
        .zip(elements.iter().skip(body_elements))
        .enumerate()
        .map(|(index, (name, ty))| (format!("{ident_prefix}{index}"), name, *ty))
        .collect();
    let has_required = entries.iter().any(|(_, _, ty)| option_inner(ty).is_none());
    let mut properties = String::new();
    let mut coding_keys = String::new();
    let mut assigns = String::new();
    for (ident, name, ty) in &entries {
        let full_ty = swift_typename_of(ty);
        let _ = writeln!(properties, "  let {ident}: {full_ty}");
        let _ = writeln!(coding_keys, "    case {ident} = \"{name}\"");
        if let Some(inner) = option_inner(ty) {
            let inner_ty = swift_typename_of(inner);
            let accessor = if has_required { "headers" } else { "headers?" };
            let _ = writeln!(
                assigns,
                "    {ident} = try {accessor}.decodeIfPresent({inner_ty}.self, forKey: .{ident})"
            );
        } else {
            let _ = writeln!(
                assigns,
                "    {ident} = try headers.decode({full_ty}.self, forKey: .{ident})"
            );
        }
    }
    let guard_block = if has_required {
        "    guard let headers else {\n      \
         throw DecodingError.keyNotFound(\n        \
         TopKeys.headers,\n        \
         DecodingError.Context(\n          \
         codingPath: decoder.codingPath,\n          \
         debugDescription: \"a declared response header was missing\"\n        \
         )\n      \
         )\n    \
         }\n"
    } else {
        ""
    };
    let idents = entries.into_iter().map(|(ident, _, _)| ident).collect();
    let text = format!(
        "struct {type_name}: Decodable {{\n\
{properties}\n  \
         private enum TopKeys: String, CodingKey {{\n    \
         case headers\n  \
         }}\n\n  \
         private enum HeaderKeys: String, CodingKey {{\n\
{coding_keys}  \
         }}\n\n  \
         init(from decoder: Decoder) throws {{\n    \
         let top = try decoder.container(keyedBy: TopKeys.self)\n    \
         let headers: KeyedDecodingContainer<HeaderKeys>?\n    \
         if top.contains(.headers) {{\n      \
         headers = try top.nestedContainer(keyedBy: HeaderKeys.self, forKey: .headers)\n    \
         }} else {{\n      \
         headers = nil\n    \
         }}\n\
{guard_block}\
{assigns}  \
         }}\n\
         }}"
    );
    (text, idents)
}

/// The reply's own success block: the plain envelope decode with no `header_out`, or the body
/// plus every header element read back and rejoined into the declared tuple.
fn success_block(
    named: &str,
    prefix: &str,
    wire: &str,
    operation: &OperationDef,
    success: &Type,
    shape: &HttpShape,
    aux: &mut Vec<String>,
) -> (String, String) {
    if !shape.header_out.is_empty() {
        let success_ty = swift_typename_of(success);
        let elements: Vec<&Type> = tuple_elements(success).into_iter().flatten().collect();
        let body_ty = elements
            .first()
            .map_or_else(|| swift_typename_of(success), |ty| swift_typename_of(ty));
        let headers_ty = format!(
            "{named}Ws{}SuccessHeaders",
            RenameRule::PascalCase.apply_to_field(&operation.ident.to_string())
        );
        let (headers_struct, header_idents) =
            header_read_struct(&headers_ty, &shape.header_out, &elements, 1, "headerOut");
        aux.push(headers_struct);
        let mut tuple_parts = vec!["decoded.value".to_owned()];
        tuple_parts.extend(
            header_idents
                .iter()
                .map(|ident| format!("headerValues.{ident}")),
        );
        let tuple_expr = tuple_parts.join(", ");
        let block = format!(
            "        do {{\n          \
             let decoded = try JSONDecoder().decode({named}WsValueEnvelope<{body_ty}>.self, \
             from: raw)\n          \
             let headerValues = try JSONDecoder().decode({headers_ty}.self, from: raw)\n          \
             return .success(({tuple_expr}))\n        \
             }} catch {{\n          \
             return .failure(.fault({prefix}WsFailedValidation(\"{wire}\", \"\\(error)\")))\n        \
             }}\n"
        );
        return (success_ty, block);
    }
    if is_unit_type(success) {
        return (
            "Void".to_owned(),
            "        return .success(())\n".to_owned(),
        );
    }
    let field_hint = format!(
        "{named}{}Success",
        RenameRule::PascalCase.apply_to_field(&operation.ident.to_string())
    );
    let (success_ty, success_aux) =
        swift_reference_type(&get_field_def("value", success, ""), &field_hint);
    aux.extend(success_aux);
    let block = format!(
        "        do {{\n          \
         let decoded = try JSONDecoder().decode({named}WsValueEnvelope<{success_ty}>.self, \
         from: raw)\n          \
         return .success(decoded.value)\n        \
         }} catch {{\n          \
         return .failure(.fault({prefix}WsFailedValidation(\"{wire}\", \"\\(error)\")))\n        \
         }}\n"
    );
    (success_ty, block)
}

/// The reply's own declared-error block: the plain envelope decode with no `error_header_out`, or
/// the head plus every header element read back and rejoined into the declared tuple.
fn declared_error_block(
    named: &str,
    prefix: &str,
    wire: &str,
    operation: &OperationDef,
    error: &Type,
    shape: &HttpShape,
    aux: &mut Vec<String>,
) -> String {
    if shape.error_header_out.is_empty() {
        let error_hint = format!(
            "{named}{}Error",
            RenameRule::PascalCase.apply_to_field(&operation.ident.to_string())
        );
        let (error_ty, error_aux) =
            swift_reference_type(&get_field_def("error", error, ""), &error_hint);
        aux.extend(error_aux);
        return format!(
            "    do {{\n      \
             let declared = try JSONDecoder().decode({named}WsDeclaredEnvelope<{error_ty}>.self, \
             from: raw)\n      \
             return .failure(.declared(declared.error))\n    \
             }} catch {{\n      \
             return .failure(.fault({prefix}WsFailedValidation(\"{wire}\", \"\\(error)\")))\n    \
             }}\n"
        );
    }
    let elements: Vec<&Type> = tuple_elements(error).into_iter().flatten().collect();
    let head_ty = elements
        .first()
        .map_or_else(|| swift_typename_of(error), |ty| swift_typename_of(ty));
    let headers_ty = format!(
        "{named}Ws{}ErrorHeaders",
        RenameRule::PascalCase.apply_to_field(&operation.ident.to_string())
    );
    let (headers_struct, header_idents) = header_read_struct(
        &headers_ty,
        &shape.error_header_out,
        &elements,
        1,
        "errorHeaderOut",
    );
    aux.push(headers_struct);
    let mut tuple_parts = vec!["declared.error".to_owned()];
    tuple_parts.extend(
        header_idents
            .iter()
            .map(|ident| format!("errorHeaderValues.{ident}")),
    );
    let tuple_expr = tuple_parts.join(", ");
    format!(
        "    do {{\n      \
         let declared = try JSONDecoder().decode({named}WsDeclaredEnvelope<{head_ty}>.self, \
         from: raw)\n      \
         let errorHeaderValues = try JSONDecoder().decode({headers_ty}.self, from: raw)\n      \
         return .failure(.declared(({tuple_expr})))\n    \
         }} catch {{\n      \
         return .failure(.fault({prefix}WsFailedValidation(\"{wire}\", \"\\(error)\")))\n    \
         }}\n"
    )
}

fn reply_method(
    named: &str,
    prefix: &str,
    operation: &OperationDef,
    error: &Type,
    success: &Type,
    has_header_in: bool,
    aux: &mut Vec<String>,
) -> String {
    let call = &operation.ts_name;
    let wire = &operation.wire_name;
    let (param_ty, param_aux) = message_swift_type(operation);
    aux.extend(param_aux);
    let failure = failure_name(named, operation).unwrap();
    let shape = HttpShape::of(operation);
    let params = header_in_params(&param_ty, &shape);
    let (headers_build, headers_arg) = if has_header_in {
        (header_in_build_stmt(named, &shape), ", headers: headers")
    } else {
        (String::new(), "")
    };
    let (success_ty, success_block) =
        success_block(named, prefix, wire, operation, success, &shape, aux);
    let error_block = declared_error_block(named, prefix, wire, operation, error, &shape, aux);
    format!(
        "  /// Calls `{wire}` over `ws_rpc`.\n  \
         public func {call}({params}) async -> Result<{success_ty}, {failure}> {{\n\
{headers_build}    \
         let raw: Data?\n    \
         do {{\n      \
         raw = try await correlate(operation: \"{wire}\", payload: req{headers_arg})\n    \
         }} catch {{\n      \
         return .failure(.fault({prefix}WsFailedValidation(\"{wire}\", \"\\(error)\")))\n    \
         }}\n    \
         guard let raw else {{\n      \
         return .failure(.fault({prefix}WsTransportFailure(\"{wire}\", \"the socket closed \
         before the reply arrived\")))\n    \
         }}\n    \
         let ok: Bool\n    \
         do {{\n      \
         ok = try JSONDecoder().decode({named}WsOkProbe.self, from: raw).ok\n    \
         }} catch {{\n      \
         return .failure(.fault({prefix}WsFailedValidation(\"{wire}\", \"\\(error)\")))\n    \
         }}\n    \
         if ok {{\n\
         {success_block}    \
         }}\n    \
         if let probed = try? JSONDecoder().decode({named}WsFaultEnvelope.self, from: raw),\n       \
         probed.error.isServiceFault == true,\n       \
         let fault = probed.error.fault {{\n      \
         return .failure(.fault(fault))\n    \
         }}\n\
{error_block}  \
         }}"
    )
}

/// [`crate::features::swift::swift_reference_type`] for the operation's own message: the type an
/// operation's one argument already is, or the type the macro declared for an operation that
/// named none — mirrors `dart_ws_client`'s own `message_dart_typename`.
fn message_swift_type(operation: &OperationDef) -> (String, Vec<String>) {
    match &operation.inputs {
        OperationInputs::Named(declared) => {
            swift_reference_type(&get_field_def("req", declared, ""), "Req")
        }
        OperationInputs::Empty | OperationInputs::Generated(_) => {
            operation.generated_message_ident().map_or_else(
                || ("String".to_owned(), Vec::new()),
                |ident| {
                    let named: Type = syn::parse_quote! { #ident };
                    swift_reference_type(&get_field_def("req", &named, ""), "Req")
                },
            )
        }
    }
}

// ---------------------------------------------------------------------------------------------
// The client callers construct.
// ---------------------------------------------------------------------------------------------

fn client_alias(named: &str) -> String {
    format!(
        "/// A `{named}` caller over `ws_rpc` — the same actor \
         `{named}WsTransport` is, under the\n\
         /// name a caller constructs, mirroring `{named}HttpClient`'s own method signatures.\n\
         public typealias {named}WsClient = {named}WsTransport"
    )
}
