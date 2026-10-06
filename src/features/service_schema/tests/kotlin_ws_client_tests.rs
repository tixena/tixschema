//! The `ws_rpc` Kotlin client, read off the emitted text — twin of `dart_ws_client_tests.rs`.
//!
//! No Kotlin toolchain is reachable in `cargo test`, so nothing here compiles the emitted source;
//! these tests read structure, the same way `dart_ws_client_tests.rs` reads Dart's own output. A
//! live kotlinc compile-and-run pass covers the heartbeat, correlation, dispatch and sharing
//! scenarios separately, outside this crate's own test suite.

use super::{
    KOTLIN_UNIT_SUCCESS_HTTP_SERVICE, KOTLIN_WS_SERVICE, MIXED_SERVICE, RESERVED_WORD_SERVICE,
    kotlin_ws_client_of,
};

/// The body of one method, one dispatch arm, or one function, from its own start marker through
/// the closing brace of whatever follows — mirrors `dart_ws_client_tests`'s own `body_from`.
fn body_from<'written>(written: &'written str, marker: &str) -> &'written str {
    let start = written.find(marker);
    assert!(start.is_some(), "no `{marker}` in: {written}");
    let rest = &written[start.unwrap()..];
    let end = rest.find("\n\n").unwrap_or(rest.len());
    &rest[..end]
}

#[test]
fn a_successful_reply_is_checked_against_the_declared_success() {
    let written = kotlin_ws_client_of(MIXED_SERVICE);
    let body = body_from(&written, "suspend fun getAvailableBalance(");
    assert!(
        body.contains("Json.decodeFromJsonElement(serializer<AvailableBalanceResponse>(),"),
        "a success the transport handed back is answered as an `AvailableBalanceResponse`, so it is \
         decoded as one first. Got: {body}"
    );
}

#[test]
fn a_failed_reply_is_checked_against_the_declared_error() {
    let written = kotlin_ws_client_of(MIXED_SERVICE);
    let body = body_from(&written, "suspend fun getAvailableBalance(");
    assert!(
        body.contains("Json.decodeFromJsonElement(serializer<BalanceError>(),"),
        "a failure the transport handed back is answered as a `BalanceError`, so it is decoded \
         as one first. Got: {body}"
    );
}

#[test]
fn a_declared_error_crosses_with_every_field_it_carries() {
    let written = kotlin_ws_client_of(MIXED_SERVICE);
    assert!(
        written.contains(
            "put(\"error\", Json.encodeToJsonElement(serializer<BalanceError>(), answered.error))"
        ),
        "the dispatcher writes the declared error through its own serializer, whole. \
         Got: {written}"
    );
    assert!(
        !written.contains("errorMessage"),
        "no field of a declared error is picked out by name. Got: {written}"
    );
}

#[test]
fn a_reply_handler_answers_the_sealed_result_rather_than_throwing_a_declared_error() {
    let written = kotlin_ws_client_of(KOTLIN_WS_SERVICE);
    assert!(
        written.contains(
            "suspend fun window(req: WindowRequest): ConversationClientServiceWindowResult"
        ),
        "got: {written}"
    );
    assert!(
        !written.contains("throws WindowError") && !written.contains(": WindowError)"),
        "a handler never throws its own declared error under the decided sealed-result shape. \
         Got: {written}"
    );
    assert!(
        written.contains("suspend fun purgeConversation(req: ConversationId)"),
        "a one-way handler still answers plainly. Got: {written}"
    );
}

#[test]
fn the_socket_seam_owns_message_and_close_and_names_no_socket_library() {
    let written = kotlin_ws_client_of(KOTLIN_WS_SERVICE);
    assert!(
        written.contains(
            "interface ConversationClientServiceWsSocket {\n  \
             fun send(text: String)\n  \
             fun close()\n  \
             var onMessage: ((String) -> Unit)?\n  \
             var onClose: (() -> Unit)?\n\
             }"
        ),
        "got: {written}"
    );
    for named in ["OkHttp", "Ktor", "okhttp3", "import "] {
        assert!(
            !written.contains(named),
            "the seam speaks only in plain terms; the library that finally carries the call is \
             an adapter's business, never this crate's. Got: {written}"
        );
    }
}

#[test]
fn the_heartbeat_options_carry_the_declared_defaults_and_a_nullable_off_switch() {
    let written = kotlin_ws_client_of(KOTLIN_WS_SERVICE);
    assert!(
        written.contains(
            "data class ConversationClientServiceWsOptions(\n  \
             val heartbeat: Heartbeat? = Heartbeat(intervalMs = 30_000, timeoutMs = 10_000),\n\
             ) {\n  \
             data class Heartbeat(val intervalMs: Long, val timeoutMs: Long)\n\
             }"
        ),
        "the default ping interval is 30 seconds and the default pong timeout is 10, and a null \
         heartbeat turns probing off. Got: {written}"
    );
}

#[test]
fn the_frames_record_carries_inbound_send_and_the_transports_own_scope() {
    let written = kotlin_ws_client_of(KOTLIN_WS_SERVICE);
    assert!(
        written.contains(
            "data class ConversationClientServiceWsFrames(\n  \
             val inbound: SharedFlow<String>,\n  \
             val send: (String) -> Unit,\n  \
             val scope: CoroutineScope,\n\
             )"
        ),
        "got: {written}"
    );
}

#[test]
fn exactly_one_transport_class_is_emitted_and_it_owns_the_socket() {
    let written = kotlin_ws_client_of(KOTLIN_WS_SERVICE);
    assert_eq!(
        written
            .matches("class ConversationClientServiceWsTransport(")
            .count(),
        1,
        "one transport class serves every operation on the service. Got: {written}"
    );
    assert!(
        written.contains("socket.onMessage = { text -> onFrame(text) }")
            && written.contains("socket.onClose = { close() }"),
        "the transport is the only thing that sets `onMessage`/`onClose`. Got: {written}"
    );
    assert!(
        written.contains(
            "val frames: ConversationClientServiceWsFrames\n    \
             get() = ConversationClientServiceWsFrames(uncorrelated.asSharedFlow(), \
             socket::send, scope)"
        ),
        "every transport publishes the same structural seam over its own scope. Got: {written}"
    );
}

#[test]
fn request_ids_start_at_one_and_correlate_through_a_completable_deferred() {
    let written = kotlin_ws_client_of(KOTLIN_WS_SERVICE);
    assert!(written.contains("private var nextId = 1"), "got: {written}");
    assert!(
        written.contains(
            "private val pending = mutableMapOf<String, CompletableDeferred<JsonObject?>>()"
        ),
        "a pending deferred's own value is nullable so the transport can settle it with `null` \
         on close, a value rather than an error. Got: {written}"
    );
    let request_body = body_from(&written, "suspend fun request(");
    assert!(
        request_body.contains("val id = synchronized(this) {")
            && request_body.contains("val assigned = (nextId++).toString()")
            && request_body.contains("pending[assigned] = deferred")
            && request_body.contains("return deferred.await()"),
        "the id is assigned and the deferred registered under one lock, so a concurrent `request` \
         never reads a stale `nextId` or races the same pending entry. Got: {request_body}"
    );
}

#[test]
fn closing_settles_every_pending_call_and_cancels_the_scope_frames_hands_out() {
    let written = kotlin_ws_client_of(KOTLIN_WS_SERVICE);
    let close_body = body_from(&written, "fun close() {");
    assert!(
        close_body.contains("socket.close()")
            && close_body.contains("deferred.complete(null)")
            && close_body.contains("scope.cancel()"),
        "closing settles every pending call with a value, `null`, rather than an error, and \
         cancels the scope `frames` hands out, tearing every dispatcher reading it down along \
         with it. Got: {close_body}"
    );
}

#[test]
fn a_missed_pong_closes_and_an_inbound_ping_draws_exactly_one_pong() {
    let written = kotlin_ws_client_of(KOTLIN_WS_SERVICE);
    let heartbeat_body = body_from(&written, "private fun scheduleHeartbeat() {");
    assert!(
        heartbeat_body.contains("delay(heartbeat.intervalMs)")
            && heartbeat_body.contains("put(\"kind\", \"ping\")")
            && heartbeat_body.contains("delay(heartbeat.timeoutMs)")
            && heartbeat_body.contains("close()"),
        "a missed pong closes the transport. Got: {heartbeat_body}"
    );
    let frame_body = body_from(&written, "private fun onFrame(text: String) {");
    assert!(
        frame_body.contains("\"ping\" -> send(buildJsonObject { put(\"kind\", \"pong\") })"),
        "an inbound ping draws exactly one pong. Got: {frame_body}"
    );
    assert!(
        frame_body.contains("\"pong\" -> {")
            && frame_body.contains("val watcher = synchronized(this) {")
            && frame_body.contains("pongJob = null")
            && frame_body.contains("watcher?.cancel()"),
        "an inbound pong re-arms the heartbeat, reading and clearing `pongJob` under one lock \
         and cancelling the watcher outside it. Got: {frame_body}"
    );
}

#[test]
fn an_uncorrelated_reply_and_a_frame_for_another_service_reach_the_uncorrelated_flow() {
    let written = kotlin_ws_client_of(KOTLIN_WS_SERVICE);
    let frame_body = body_from(&written, "private fun onFrame(text: String) {");
    assert!(
        frame_body.contains(
            "if (deferred != null) deferred.complete(frame) else uncorrelated.tryEmit(text)"
        ) && frame_body.contains("else -> uncorrelated.tryEmit(text)"),
        "a reply this transport did not ask for, and any other frame — including one naming \
         another service — is handed to `frames.inbound` rather than dropped. Got: {frame_body}"
    );
}

#[test]
fn a_reply_operation_answers_the_sealed_result_and_decodes_through_the_generated_codec() {
    let written = kotlin_ws_client_of(KOTLIN_WS_SERVICE);
    assert!(
        written.contains(
            "suspend fun window(req: WindowRequest): ConversationClientServiceWindowResult {"
        ),
        "got: {written}"
    );
    let method = body_from(&written, "suspend fun window(");
    assert!(
        method.contains(
            "transport.request(\"window\", Json.encodeToJsonElement(serializer<WindowRequest>(), req))"
        ),
        "got: {method}"
    );
    assert!(
        method.contains("if (ok) {")
            && method.contains(
                "ConversationClientServiceWindowResult.Ok(Json.decodeFromJsonElement(serializer<WindowPage>(), reply.getValue(\"value\")))"
            ),
        "a successful reply decodes through the generated codec into the sealed result's own \
         `Ok` member. Got: {method}"
    );
}

#[test]
fn a_reply_operation_answers_the_declared_error_or_a_fault_behind_is_service_fault() {
    let written = kotlin_ws_client_of(KOTLIN_WS_SERVICE);
    let method = body_from(&written, "suspend fun window(");
    assert!(
        method.contains(
            "return ConversationClientServiceWindowResult.Fault(conversationClientServiceWsTransportFailure(\"window\", uncarried.toString()))"
        ),
        "a transport failure is a fault, never the declared error. Got: {method}"
    );
    assert!(
        method.contains("if (reply == null) {")
            && method.contains(
                "conversationClientServiceWsTransportFailure(\"window\", \"the connection closed before a reply arrived\"),"
            ),
        "a request still waiting when the socket closes answers the same transport-failure \
         fault, read off the `null` the transport settles its deferred with. Got: {method}"
    );
    assert!(
        method.contains("val isServiceFault = (error as? JsonObject)?.get(\"isServiceFault\")")
            && method.contains(
                "ConversationClientServiceWindowResult.Fault(\n          Json.decodeFromJsonElement(serializer<ConversationClientServiceFaultFields>(), error.getValue(\"fault\")),"
            ),
        "a wire fault behind `isServiceFault` is read back through the generated fault codec. \
         Got: {method}"
    );
    assert!(
        method.contains(
            "ConversationClientServiceWindowResult.Declared(Json.decodeFromJsonElement(serializer<WindowError>(), error ?: JsonNull))"
        ),
        "otherwise the wire's `error` is the operation's own declared error. Got: {method}"
    );
}

#[test]
fn a_reply_decode_failure_is_a_failed_validation_fault_not_an_undeserializable_payload_one() {
    let written = kotlin_ws_client_of(KOTLIN_WS_SERVICE);
    let method = body_from(&written, "suspend fun window(");
    assert!(
        method
            .matches("conversationClientServiceWsFailedValidation(")
            .count()
            >= 2,
        "both a bad success value and a bad declared error decode to a failed-validation fault. \
         Got: {method}"
    );
    assert!(
        !written.contains("UndeserializablePayload")
            && !written.contains("undeserializablePayload"),
        "`ws_rpc` has no status ladder, so this crate's own decision names every reply decode \
         failure a failed-validation fault instead. Got: {written}"
    );
}

#[test]
fn a_one_way_operation_answers_plainly_and_throws_a_ws_refusal_named_apart_from_the_http_one() {
    let written = kotlin_ws_client_of(KOTLIN_WS_SERVICE);
    assert!(
        written.contains("suspend fun purgeConversation(req: ConversationId) {"),
        "got: {written}"
    );
    let method = body_from(&written, "suspend fun purgeConversation(");
    assert!(
        method.contains(
            "transport.notify(\"purge-conversation\", Json.encodeToJsonElement(serializer<ConversationId>(), req))"
        ) && method.contains(
            "throw ConversationClientServiceWsRefusal(conversationClientServiceWsTransportFailure(\"purge-conversation\", uncarried.toString()))"
        ),
        "a one-way method throws the fault-only ws refusal, named apart from \
         `kotlin_http_client()`'s own bare refusal so both coexist in one file. Got: {method}"
    );
    assert!(
        written.contains("class ConversationClientServiceWsRefusal(val fault: ConversationClientServiceFaultFields) : Exception(fault.detail)"),
        "got: {written}"
    );
}

#[test]
fn the_client_class_holds_one_constructor_and_every_operation_as_a_method() {
    let written = kotlin_ws_client_of(KOTLIN_WS_SERVICE);
    assert!(
        written.contains(
            "class ConversationClientServiceWsClient(private val transport: ConversationClientServiceWsTransport) {"
        ),
        "got: {written}"
    );
    for call in ["window", "purgeConversation"] {
        assert!(
            written.contains(&format!("suspend fun {call}(")),
            "operation `{call}` should have a method on the client. Got: {written}"
        );
    }
}

#[test]
fn the_handlers_interface_carries_one_member_per_operation_with_no_context_parameter() {
    let written = kotlin_ws_client_of(KOTLIN_WS_SERVICE);
    assert!(
        written.contains("interface ConversationClientServiceHandlers {"),
        "got: {written}"
    );
    assert!(
        written.contains(
            "  suspend fun window(req: WindowRequest): ConversationClientServiceWindowResult"
        ) && written.contains("  suspend fun purgeConversation(req: ConversationId)"),
        "a handler takes only the operation's own request; no `Ctx` parameter is declared. \
         Got: {written}"
    );
}

#[test]
fn the_attachment_launches_on_frames_own_scope_and_exposes_detach_and_share() {
    let written = kotlin_ws_client_of(KOTLIN_WS_SERVICE);
    assert!(
        written.contains(
            "fun attachConversationClientServiceWsDispatcher(\n  \
             frames: ConversationClientServiceWsFrames,\n  \
             handlers: ConversationClientServiceHandlers,\n  \
             onFault: (ConversationClientServiceFaultFields) -> Unit,\n\
             ): ConversationClientServiceWsAttachment {"
        ),
        "got: {written}"
    );
    assert!(
        written.contains("val job = frames.scope.launch {"),
        "the attachment launches on the transport's own scope rather than a fresh one, so \
         closing the transport ends this attachment too. Got: {written}"
    );
    assert!(
        written.contains(
            "return ConversationClientServiceWsAttachment(frames.scope, frames.send, job)"
        ),
        "got: {written}"
    );
    assert!(
        written.contains("class ConversationClientServiceWsAttachment internal constructor(")
            && written.contains("fun detach() {")
            && written.contains("job.cancel()")
            && written.contains(
                "fun share(attach: (send: (String) -> Unit, scope: CoroutineScope) -> () -> Unit): () -> Unit {"
            )
            && written.contains("check(job.isActive) { \"the socket is closed\" }"),
        "`share` throws once this attachment has detached, and `detach` cancels its own job. \
         Got: {written}"
    );
}

#[test]
fn a_frame_naming_another_service_or_carrying_no_operation_is_left_for_that_service() {
    let written = kotlin_ws_client_of(KOTLIN_WS_SERVICE);
    let attach_body = body_from(&written, "fun attachConversationClientServiceWsDispatcher(");
    assert!(
        attach_body.contains(
            "if ((frame[\"service\"] as? JsonPrimitive)?.contentOrNull != \"ConversationClientService\") return@collect"
        ) && attach_body.contains(
            "val operation = (frame[\"operation\"] as? JsonPrimitive)?.contentOrNull ?: return@collect"
        ),
        "got: {attach_body}"
    );
}

#[test]
fn an_unknown_operation_reports_and_replies_with_the_fault() {
    let written = kotlin_ws_client_of(KOTLIN_WS_SERVICE);
    assert!(
        written.contains("private suspend fun conversationClientServiceWsDispatch(")
            && written.contains("when (operation) {")
            && written.contains("\"window\" -> {")
            && written.contains("\"purge-conversation\" -> {"),
        "got: {written}"
    );
    let default_arm = body_from(&written, "else -> {");
    assert!(
        default_arm.contains("conversationClientServiceWsUnknownOperation(operation,")
            && default_arm.contains("onFault(fault)")
            && default_arm.contains("if (id != null) send(")
            && default_arm.contains("\"isServiceFault\", true"),
        "an operation nothing on the service answers to reaches `onFault`, and a request left \
         waiting on it is answered with the same fault rather than left hanging. \
         Got: {default_arm}"
    );
}

#[test]
fn a_reply_dispatch_arm_narrows_on_the_sealed_result_for_every_outcome() {
    let written = kotlin_ws_client_of(KOTLIN_WS_SERVICE);
    let arm = body_from(&written, "\"window\" -> {");
    assert!(
        arm.contains(
            "Json.decodeFromJsonElement(serializer<WindowRequest>(), payload ?: JsonNull)"
        ) && arm.contains("val answered = try {")
            && arm.contains("handlers.window(decoded)")
            && arm.contains("is ConversationClientServiceWindowResult.Ok -> send(")
            && arm.contains("is ConversationClientServiceWindowResult.Declared -> send(")
            && arm.contains("is ConversationClientServiceWindowResult.Fault -> send("),
        "got: {arm}"
    );
    assert!(
        !arm.contains("catch (declared:") && !arm.contains("} on WindowError"),
        "a declared error is read off the sealed result, never caught off a thrown type. \
         Got: {arm}"
    );
}

#[test]
fn a_one_way_dispatch_arm_acknowledges_a_request_frame_with_a_null_value() {
    let written = kotlin_ws_client_of(KOTLIN_WS_SERVICE);
    let arm = body_from(&written, "\"purge-conversation\" -> {");
    assert!(
        arm.contains("handlers.purgeConversation(decoded)")
            && arm.contains(
                "if (id != null) send(buildJsonObject { put(\"kind\", \"reply\"); put(\"id\", id); \
                 put(\"service\", \"ConversationClientService\"); put(\"ok\", true); \
                 put(\"value\", JsonNull) }.toString())"
            ),
        "a one-way operation reached over a request frame is still answered once its handler \
         returns, `ok: true` with a `null` value, rather than left hanging. Got: {arm}"
    );
}

#[test]
fn a_handler_throw_becomes_a_handler_panic_fault_and_reaches_on_fault() {
    let written = kotlin_ws_client_of(KOTLIN_WS_SERVICE);
    for arm_marker in ["\"window\" -> {", "\"purge-conversation\" -> {"] {
        let arm = body_from(&written, arm_marker);
        assert!(
            arm.contains("catch (unexpected: Throwable) {")
                && arm.contains("conversationClientServiceWsHandlerPanic(")
                && arm.contains("onFault(fault)"),
            "anything a handler throws reaches `onFault` as a handler-panic fault. \
             Got: {arm}"
        );
    }
}

#[test]
fn the_fault_helpers_reuse_the_generated_fault_fields_and_kind() {
    let written = kotlin_ws_client_of(KOTLIN_WS_SERVICE);
    for (helper, kind) in [
        (
            "conversationClientServiceWsTransportFailure",
            "ConversationClientServiceFaultKind.TransportFailure",
        ),
        (
            "conversationClientServiceWsFailedValidation",
            "ConversationClientServiceFaultKind.FailedValidation",
        ),
        (
            "conversationClientServiceWsUnknownOperation",
            "ConversationClientServiceFaultKind.UnknownOperation",
        ),
        (
            "conversationClientServiceWsHandlerPanic",
            "ConversationClientServiceFaultKind.HandlerPanic",
        ),
    ] {
        assert!(
            written.contains(&format!(
                "private fun {helper}(operation: String, detail: String): ConversationClientServiceFaultFields ="
            )) && written.contains(kind),
            "helper `{helper}` should report `{kind}`. Got: {written}"
        );
    }
}

#[test]
fn a_unit_success_answers_the_field_less_ok_member() {
    let written = kotlin_ws_client_of(KOTLIN_UNIT_SUCCESS_HTTP_SERVICE);
    let method = body_from(&written, "suspend fun ping(");
    assert!(
        method.contains("if (ok) {") && method.contains("return PingClientServicePingResult.Ok"),
        "got: {method}"
    );
    assert!(
        !method.contains("Result.Ok(Json.decodeFromJsonElement"),
        "a unit success reads no `value` key off the reply. Got: {method}"
    );
}

#[test]
fn a_hard_keyword_is_written_between_backticks_in_the_method_and_the_handler() {
    let written = kotlin_ws_client_of(RESERVED_WORD_SERVICE);
    for escaped in [
        "suspend fun `for`(req: ForRequest, default: String?)",
        "suspend fun `object`(req: Thing)",
        "handlers.`for`(decoded, default)",
    ] {
        assert!(written.contains(escaped), "no `{escaped}` in: {written}");
    }
}
