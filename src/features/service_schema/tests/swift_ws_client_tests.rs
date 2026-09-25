//! The Swift `ws_rpc` client, read off the emitted text.
//!
//! No consuming build compiles this text against real message types here, so these are the same
//! kind of structural read `dart_ws_client_tests` gives its own emitted source: a substring that
//! must appear, and a name that must not. The text has also been compiled and driven under a real
//! Swift toolchain, against an in-memory socket, separately from this file's own assertions.

use super::{
    SWIFT_UNIT_SUCCESS_SERVICE, SWIFT_WS_HEADERS_SERVICE, SWIFT_WS_SERVICE, swift_ws_client_of,
};

/// The body of one declaration, from its own start marker through the closing brace that ends
/// it at column zero — mirrors `dart_ws_client_tests`'s own `body_from`, adjusted for Swift's
/// brace-per-declaration rather than blank-line-separated statements.
fn body_from<'written>(written: &'written str, marker: &str) -> &'written str {
    let start = written.find(marker);
    assert!(start.is_some(), "no `{marker}` in: {written}");
    let rest = &written[start.unwrap()..];
    let end = rest.find("\n}").map_or(rest.len(), |at| at + 2);
    &rest[..end]
}

#[test]
fn the_socket_seam_has_four_members_and_names_no_networking_type() {
    let written = swift_ws_client_of(SWIFT_WS_SERVICE);
    assert!(
        written
            .contains("public protocol ConversationClientServiceWsSocket: AnyObject, Sendable {"),
        "got: {written}"
    );
    let seam = body_from(
        &written,
        "public protocol ConversationClientServiceWsSocket",
    );
    for member in [
        "func send(_ text: String)",
        "func close()",
        "var onMessage: (@Sendable (String) -> Void)? { get set }",
        "var onClose: (@Sendable () -> Void)? { get set }",
    ] {
        assert!(seam.contains(member), "got: {seam}");
    }
    for named in [
        "URLSessionWebSocketTask",
        "URLSession",
        "NWConnection",
        "import ",
    ] {
        assert!(
            !written.contains(named),
            "the seam speaks only in send/close/callback terms; the networking library that \
             finally carries the call is an adapter's business. Got: {written}"
        );
    }
}

#[test]
fn the_options_default_to_thirty_and_ten_seconds_and_nil_turns_probing_off() {
    let written = swift_ws_client_of(SWIFT_WS_SERVICE);
    assert!(
        written.contains("public struct ConversationClientServiceWsOptions: Sendable {"),
        "got: {written}"
    );
    assert!(
        written.contains("public var heartbeat: Heartbeat?")
            && written.contains("public var intervalMs: Int")
            && written.contains("public var timeoutMs: Int"),
        "got: {written}"
    );
    assert!(
        written.contains(
            "public init(heartbeat: Heartbeat? = Heartbeat(intervalMs: 30_000, timeoutMs: 10_000)) {"
        ),
        "the default ping interval is 30 seconds and the default pong timeout is 10; a caller \
         passing `nil` turns probing off. Got: {written}"
    );
}

#[test]
fn exactly_one_transport_actor_is_emitted() {
    let written = swift_ws_client_of(SWIFT_WS_SERVICE);
    assert_eq!(
        written
            .matches("public actor ConversationClientServiceWsTransport {")
            .count(),
        1,
        "one actor serves every operation on the service. Got: {written}"
    );
}

#[test]
fn inbound_frames_are_fed_through_one_ordered_async_stream_rather_than_a_task_per_frame() {
    let written = swift_ws_client_of(SWIFT_WS_SERVICE);
    let init = body_from(
        &written,
        "public init(socket: any ConversationClientServiceWsSocket",
    );
    assert!(
        init.contains("AsyncStream<String>.makeStream()")
            && init.contains("outbound.yield(text)")
            && init.contains("for await text in inbound {"),
        "every inbound frame is decoded off one stream, in the order it was yielded, rather than \
         through a freshly spawned `Task` per frame with no ordering guarantee between them. \
         Got: {init}"
    );
    assert!(
        !init.contains("Task { [weak self] in await self?.handleMessage"),
        "handleMessage is reached only from the stream's own consuming loop, never from a \
         one-off `Task` per frame. Got: {init}"
    );
}

#[test]
fn the_transport_answers_an_inbound_ping_and_treats_a_pong_as_rearming() {
    let written = swift_ws_client_of(SWIFT_WS_SERVICE);
    let handler = body_from(
        &written,
        "private func handleMessage(_ text: String) async {",
    );
    assert!(
        handler.contains("case \"ping\":")
            && handler.contains("socket.send(\"{\\\"kind\\\":\\\"pong\\\"}\")"),
        "got: {handler}"
    );
    assert!(
        handler.contains("case \"pong\":")
            && handler.contains("pongDeadlineTask?.cancel()")
            && handler.contains("schedulePing()"),
        "got: {handler}"
    );
}

#[test]
fn a_missed_pong_closes_the_socket_and_settles_every_waiting_call() {
    let written = swift_ws_client_of(SWIFT_WS_SERVICE);
    let ping = body_from(&written, "private func sendPing() {");
    assert!(
        ping.contains("socket.send(\"{\\\"kind\\\":\\\"ping\\\"}\")")
            && ping.contains("pongDeadlineTask = Task"),
        "got: {ping}"
    );
    let missed = body_from(&written, "private func pongMissed() {");
    assert!(
        missed.contains("socket.close()") && missed.contains("handleClose()"),
        "got: {missed}"
    );
    let close = body_from(&written, "private func handleClose() {");
    assert!(
        close.contains("continuation.resume(returning: nil)"),
        "every request still waiting resumes with `nil`, which each waiting method reads as the \
         transport-failure fault. Got: {close}"
    );
}

#[test]
fn a_request_frame_and_a_notify_frame_are_written_with_the_service_name() {
    let written = swift_ws_client_of(SWIFT_WS_SERVICE);
    assert!(
        written.contains(
            "struct ConversationClientServiceWsRequestFrame<Payload: Encodable>: Encodable {"
        ) && written.contains("let kind = \"request\""),
        "got: {written}"
    );
    assert!(
        written.contains(
            "struct ConversationClientServiceWsNotifyFrame<Payload: Encodable>: Encodable {"
        ) && written.contains("let kind = \"notify\""),
        "got: {written}"
    );
    let correlate = body_from(&written, "private func correlate<Payload: Encodable>(");
    assert!(
        correlate.contains("service: \"ConversationClientService\""),
        "got: {correlate}"
    );
}

#[test]
fn a_reply_operation_answers_result_and_decodes_success_declared_error_or_fault() {
    let written = swift_ws_client_of(SWIFT_WS_SERVICE);
    assert!(
        written.contains(
            "public func window(_ req: WindowRequest) async -> Result<WindowPage, \
             ConversationClientServiceWindowFailure> {"
        ),
        "the reply method's return type names the failure `swift_http_client()` declares, rather \
         than redeclaring one. Got: {written}"
    );
    let method = body_from(&written, "public func window(_ req: WindowRequest)");
    assert!(
        method.contains("raw = try await correlate(operation: \"window\", payload: req)"),
        "got: {method}"
    );
    assert!(
        method.contains(
            "let decoded = try JSONDecoder().decode(ConversationClientServiceWsValueEnvelope<WindowPage>.self, from: raw)"
        ) && method.contains("return .success(decoded.value)"),
        "a successful reply decodes through the generic value envelope into the success arm. \
         Got: {method}"
    );
    assert!(
        method.contains(
            "let declared = try JSONDecoder().decode(ConversationClientServiceWsDeclaredEnvelope<WindowError>.self, from: raw)"
        ) && method.contains("return .failure(.declared(declared.error))"),
        "got: {method}"
    );
    assert!(
        method.contains("probed.error.isServiceFault == true")
            && method.contains("return .failure(.fault(fault))"),
        "got: {method}"
    );
}

#[test]
fn a_reply_still_waiting_when_the_socket_closes_answers_a_transport_failure_fault() {
    let written = swift_ws_client_of(SWIFT_WS_SERVICE);
    let method = body_from(&written, "public func window(_ req: WindowRequest)");
    assert!(
        method.contains("guard let raw else {")
            && method.contains(
                "conversationClientServiceWsTransportFailure(\"window\", \"the socket closed \
                 before the reply arrived\")"
            ),
        "got: {method}"
    );
}

#[test]
fn a_reply_decode_failure_is_a_failed_validation_fault() {
    let written = swift_ws_client_of(SWIFT_WS_SERVICE);
    let method = body_from(&written, "public func window(_ req: WindowRequest)");
    assert!(
        method
            .matches("conversationClientServiceWsFailedValidation(\"window\",")
            .count()
            >= 3,
        "an outbound encode failure, a bad success value and a bad declared error all decode to \
         `failedValidation`. Got: {method}"
    );
}

#[test]
fn a_one_way_operation_is_async_throws_and_throws_only_the_shared_refusal() {
    let written = swift_ws_client_of(SWIFT_WS_SERVICE);
    assert!(
        written.contains("public func purgeConversation(_ req: ConversationId) async throws {"),
        "got: {written}"
    );
    let method = body_from(
        &written,
        "public func purgeConversation(_ req: ConversationId)",
    );
    assert!(
        method.contains("try sendNotify(operation: \"purge-conversation\", payload: req)")
            && method.contains(
                "throw ConversationClientServiceRefusal(fault: \
                 conversationClientServiceWsFailedValidation(\"purge-conversation\", \"\\(error)\"))"
            ),
        "a one-way method throws the shared refusal, naming no error type of its own. \
         Got: {method}"
    );
}

#[test]
fn nothing_here_redeclares_the_shared_failure_or_refusal_types() {
    let written = swift_ws_client_of(SWIFT_WS_SERVICE);
    for redeclared in [
        "enum ConversationClientServiceWindowFailure",
        "struct ConversationClientServiceRefusal",
        "struct ConversationClientServiceFaultFields",
    ] {
        assert!(
            !written.contains(redeclared),
            "the failure, the refusal and the fault fields are declared once, by \
             `swift_http_client()`; this module only names them. Got: {written}"
        );
    }
}

#[test]
fn the_fault_helpers_carry_a_ws_infix_so_a_bundle_with_both_clients_declares_each_name_once() {
    let written = swift_ws_client_of(SWIFT_WS_SERVICE);
    assert!(
        written.contains(
            "func conversationClientServiceWsTransportFailure(_ operation: String, _ detail: \
             String) -> ConversationClientServiceFault {"
        ) && written.contains("kind: .transportFailure, operation: operation)"),
        "got: {written}"
    );
    assert!(
        written.contains(
            "func conversationClientServiceWsFailedValidation(_ operation: String, _ detail: \
             String) -> ConversationClientServiceFault {"
        ),
        "got: {written}"
    );
}

#[test]
fn the_client_alias_names_the_same_actor_under_the_name_a_caller_constructs() {
    let written = swift_ws_client_of(SWIFT_WS_SERVICE);
    assert!(
        written.contains(
            "public typealias ConversationClientServiceWsClient = ConversationClientServiceWsTransport"
        ),
        "got: {written}"
    );
}

#[test]
fn a_unit_success_answers_the_void_result_with_no_value_decode() {
    let written = swift_ws_client_of(SWIFT_UNIT_SUCCESS_SERVICE);
    assert!(
        written.contains(
            "public func ping(_ req: PingRequest) async -> Result<Void, PingClientServicePingFailure> {"
        ),
        "got: {written}"
    );
    let method = body_from(&written, "public func ping(_ req: PingRequest)");
    assert!(
        method.contains("if ok {\n        return .success(())\n    }"),
        "got: {method}"
    );
    assert!(
        !written.contains("PingClientServiceWsValueEnvelope<Void>"),
        "`Void` is not `Decodable`; a unit success is never decoded through the value envelope. \
         Got: {written}"
    );
}

#[test]
fn a_headerless_service_gets_no_headers_field_on_either_frame() {
    let written = swift_ws_client_of(SWIFT_WS_SERVICE);
    assert!(
        !written.contains("headers"),
        "a service declaring no `header_in` anywhere must emit exactly the same text as before — \
         no `headers` field, no `WsHeaderIn` carrier. Got: {written}"
    );
}

#[test]
fn a_header_in_binding_becomes_an_extra_parameter_and_a_built_dictionary() {
    let written = swift_ws_client_of(SWIFT_WS_HEADERS_SERVICE);
    assert!(
        written.contains("public func mark(_ req: String, tenant: String) async throws {"),
        "got: {written}"
    );
    let method = body_from(&written, "public func mark(_ req: String, tenant: String)");
    assert!(
        method.contains(
            "let headers: [String: StampClientServiceWsHeaderIn]? = [\n      \"x-tenant\": \
             StampClientServiceWsHeaderIn(tenant),\n    ]"
        ) && method.contains("try sendNotify(operation: \"mark\", payload: req, headers: headers)"),
        "got: {method}"
    );
}

#[test]
fn an_optional_header_in_value_is_wrapped_and_sent_unconditionally_rather_than_unwrapped() {
    let written = swift_ws_client_of(SWIFT_WS_HEADERS_SERVICE);
    let method = body_from(
        &written,
        "public func stamp(_ req: String, tenant: String, trace: UInt32?)",
    );
    assert!(
        method.contains("\"x-trace\": StampClientServiceWsHeaderIn(trace),"),
        "an `Option` header_in value is handed to the carrier whole — its own `Encodable` \
         conformance writes `null` for `nil` rather than the value being unwrapped and skipped. \
         Got: {method}"
    );
}

#[test]
fn the_shared_actor_methods_carry_a_headers_parameter_passed_through_from_every_call_site() {
    let written = swift_ws_client_of(SWIFT_WS_HEADERS_SERVICE);
    let correlate = body_from(&written, "private func correlate<Payload: Encodable>(");
    assert!(
        correlate.contains("headers: [String: StampClientServiceWsHeaderIn]?,")
            && correlate.contains("payload: payload, headers: headers)"),
        "got: {correlate}"
    );
    let notify = body_from(&written, "private func sendNotify<Payload: Encodable>(");
    assert!(
        notify.contains("headers: [String: StampClientServiceWsHeaderIn]?")
            && notify.contains("payload: payload, headers: headers)"),
        "got: {notify}"
    );
}

#[test]
fn a_header_out_tuple_reads_the_body_and_a_dedicated_header_structure_back() {
    let written = swift_ws_client_of(SWIFT_WS_HEADERS_SERVICE);
    assert!(
        written.contains(
            "public func stamp(_ req: String, tenant: String, trace: UInt32?) async -> \
             Result<(StampReceipt, String, UInt32?), StampClientServiceStampFailure> {"
        ),
        "the success tuple carries the body and every declared `header_out` element, exactly as \
         `swift_http_client()`'s own return type does. Got: {written}"
    );
    let method = body_from(
        &written,
        "public func stamp(_ req: String, tenant: String, trace: UInt32?)",
    );
    assert!(
        method.contains(
            "let decoded = try JSONDecoder().decode(StampClientServiceWsValueEnvelope<StampReceipt>.self, from: raw)"
        ) && method.contains(
            "let headerValues = try JSONDecoder().decode(StampClientServiceWsStampSuccessHeaders.self, from: raw)"
        ) && method.contains(
            "return .success((decoded.value, headerValues.headerOut0, headerValues.headerOut1))"
        ),
        "the body decodes through the plain value envelope; the headers decode separately \
         through their own structure and are rejoined by hand. Got: {method}"
    );
}

#[test]
fn the_success_headers_structure_defaults_an_absent_optional_element_and_throws_on_a_missing_required_one()
 {
    let written = swift_ws_client_of(SWIFT_WS_HEADERS_SERVICE);
    assert!(
        written.contains("struct StampClientServiceWsStampSuccessHeaders: Decodable {"),
        "got: {written}"
    );
    let body = body_from(
        &written,
        "struct StampClientServiceWsStampSuccessHeaders: Decodable {",
    );
    assert!(
        body.contains("let headerOut0: String") && body.contains("let headerOut1: UInt32?"),
        "got: {body}"
    );
    assert!(
        body.contains("case headerOut0 = \"etag\"") && body.contains("case headerOut1 = \"x-age\""),
        "each element decodes under the declared header name, not its synthesized identifier. \
         Got: {body}"
    );
    assert!(
        body.contains("guard let headers else {")
            && body.contains("throw DecodingError.keyNotFound("),
        "at least one required element means a wholly-absent `headers` object is itself a fault. \
         Got: {body}"
    );
    assert!(
        body.contains("headerOut0 = try headers.decode(String.self, forKey: .headerOut0)")
            && body.contains(
                "headerOut1 = try headers.decodeIfPresent(UInt32.self, forKey: .headerOut1)"
            ),
        "the required element throws through `decode` when its own key is missing; the optional \
         one defaults to `nil` through `decodeIfPresent`. Got: {body}"
    );
}

#[test]
fn an_all_optional_error_headers_structure_never_requires_the_headers_object_at_all() {
    let written = swift_ws_client_of(SWIFT_WS_HEADERS_SERVICE);
    let body = body_from(
        &written,
        "struct StampClientServiceWsStampErrorHeaders: Decodable {",
    );
    assert!(
        !body.contains("guard let headers else {"),
        "every declared element is optional, so a wholly-absent `headers` object is not itself a \
         fault — only reachable per element. Got: {body}"
    );
    assert!(
        body.contains(
            "errorHeaderOut0 = try headers?.decodeIfPresent(String.self, forKey: .errorHeaderOut0)"
        ),
        "got: {body}"
    );
}

#[test]
fn a_declared_error_with_error_header_out_reads_the_head_and_its_own_headers_back() {
    let written = swift_ws_client_of(SWIFT_WS_HEADERS_SERVICE);
    let method = body_from(
        &written,
        "public func stamp(_ req: String, tenant: String, trace: UInt32?)",
    );
    assert!(
        method.contains(
            "let declared = try JSONDecoder().decode(StampClientServiceWsDeclaredEnvelope<StampError>.self, from: raw)"
        ) && method.contains(
            "let errorHeaderValues = try JSONDecoder().decode(StampClientServiceWsStampErrorHeaders.self, from: raw)"
        ) && method.contains(
            "return .failure(.declared((declared.error, errorHeaderValues.errorHeaderOut0)))"
        ),
        "got: {method}"
    );
}

#[test]
fn no_unkeyed_tuple_codec_type_is_generated_for_a_header_bearing_reply() {
    let written = swift_ws_client_of(SWIFT_WS_HEADERS_SERVICE);
    for named in ["SuccessTuple", "ErrorTuple"] {
        assert!(
            !written.contains(named),
            "a `ws_rpc` reply decodes its body and its headers separately; nothing here needs an \
             unkeyed tuple codec. Got: {written}"
        );
    }
}
