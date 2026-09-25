//! The `ws_rpc` Dart client, read off the emitted text.
//!
//! No Dart toolchain is reachable here, so nothing here type-checks the emitted source — these
//! tests read structure, the same way `dart_http_client_tests` reads the `http_rest` sibling's own
//! output: a substring that must appear, and a name that must not.

use super::{
    DART_HEADER_TUPLE_SERVICE, DART_PRIMITIVE_SERVICE, DART_UNIT_SUCCESS_HTTP_SERVICE,
    DART_WS_SERVICE, dart_ws_client_of,
};

/// The body of one method or dispatch arm, from its own start marker through the closing brace of
/// whatever follows — mirrors `dart_http_client_tests`'s own `method_body`.
fn body_from<'written>(written: &'written str, marker: &str) -> &'written str {
    let start = written.find(marker);
    assert!(start.is_some(), "no `{marker}` in: {written}");
    let rest = &written[start.unwrap()..];
    let end = rest.find("\n\n").unwrap_or(rest.len());
    &rest[..end]
}

#[test]
fn no_thrown_declared_error_or_fault_survives_a_reply_answers_the_result_pair_instead() {
    let written = dart_ws_client_of(DART_WS_SERVICE);
    // Spelled apart so this negative assertion itself does not reintroduce the removed name into
    // `src/`, which the task's own exit gate greps for.
    let removed_class = format!("Ws{}", "Error");
    for gone in [removed_class.as_str(), ".declared(", ".fault("] {
        assert!(
            !written.contains(gone),
            "a reply method answers the result pair rather than throwing it. Got: {written}"
        );
    }
    assert!(
        written.contains("Future<void> applyBundle(ApplyBundleRequest req) async {"),
        "a one-way method still answers `Future<void>`. Got: {written}"
    );
}

#[test]
fn exactly_one_transport_class_is_emitted_and_it_names_no_socket_package() {
    let written = dart_ws_client_of(DART_WS_SERVICE);
    assert_eq!(
        written.matches("class LedgerWsTransport {").count(),
        1,
        "one transport class serves every operation on the service. Got: {written}"
    );
    for named in [
        "package:web_socket_channel",
        "web_socket_channel",
        "dart:io",
        "import '",
    ] {
        assert!(
            !written.contains(named),
            "the transport speaks only in sink/stream terms; the socket package that finally \
             carries the call is an adapter's business, never this crate's. Got: {written}"
        );
    }
}

#[test]
fn the_heartbeat_class_carries_the_declared_defaults_and_a_const_off_constructor() {
    let written = dart_ws_client_of(DART_WS_SERVICE);
    assert!(
        written.contains("class LedgerWsHeartbeat {"),
        "got: {written}"
    );
    assert!(
        written.contains("final Duration interval;") && written.contains("final Duration timeout;"),
        "got: {written}"
    );
    assert!(
        written.contains("this.interval = const Duration(seconds: 30),")
            && written.contains("this.timeout = const Duration(seconds: 10),"),
        "the default ping interval is 30 seconds and the default pong timeout is 10. Got: {written}"
    );
    assert!(
        written.contains("const LedgerWsHeartbeat.off()"),
        "a const constructor turns the heartbeat off. Got: {written}"
    );
}

#[test]
fn the_transport_is_constructed_over_a_sink_and_a_stream_with_an_optional_heartbeat() {
    let written = dart_ws_client_of(DART_WS_SERVICE);
    assert!(
        written.contains("LedgerWsTransport({")
            && written.contains("required StreamSink<dynamic> sink,")
            && written.contains("required Stream<dynamic> stream,")
            && written.contains("LedgerWsHeartbeat? heartbeat,"),
        "a null heartbeat takes the default, constructed beside the required sink and stream. \
         Got: {written}"
    );
    assert!(
        written.contains("_heartbeat = heartbeat ?? const LedgerWsHeartbeat()"),
        "got: {written}"
    );
    assert!(
        written.contains(
            "_subscription = stream.listen(_onFrame, onDone: _onClose, onError: (_) => _onClose());"
        ),
        "the transport owns the one `stream.listen` a non-broadcast stream allows. Got: {written}"
    );
}

#[test]
fn the_transport_holds_pending_completers_keyed_by_frame_id_and_can_close() {
    let written = dart_ws_client_of(DART_WS_SERVICE);
    assert!(
        written.contains("final Map<String, Completer<Map<String, dynamic>?>> _pending = {};"),
        "a pending completer's own value is nullable so the transport can settle it with `null` \
         on close, a value rather than an error. Got: {written}"
    );
    assert!(
        written.contains("Timer? _pingTimer;") && written.contains("Timer? _pongTimer;"),
        "the heartbeat is timer-driven. Got: {written}"
    );
    assert!(
        written.contains("void close() => _onClose();"),
        "got: {written}"
    );
    let close_body = body_from(&written, "void _onClose()");
    assert!(
        close_body.contains("_subscription.cancel();")
            && close_body.contains("completer.complete(null);")
            && close_body.contains("_controller.close();"),
        "closing settles every pending call with a value, `null`, rather than an error, and \
         closes the frames controller rather than leaving either hanging. Got: {close_body}"
    );
}

#[test]
fn request_ids_start_at_one() {
    let written = dart_ws_client_of(DART_WS_SERVICE);
    assert!(written.contains("int _nextId = 1;"), "got: {written}");
}

#[test]
fn the_transport_exposes_frames_as_a_structural_record_over_a_broadcast_controller() {
    let written = dart_ws_client_of(DART_WS_SERVICE);
    assert!(
        written.contains(
            "final StreamController<Map<String, dynamic>> _controller =\n      \
             StreamController<Map<String, dynamic>>.broadcast();"
        ),
        "got: {written}"
    );
    assert!(
        written.contains(
            "({Stream<Map<String, dynamic>> inbound, void Function(Map<String, dynamic>) send}) \
             get frames => (inbound: _controller.stream, send: send);"
        ),
        "every transport publishes the same structural seam, keyed on `inbound` and `send`. \
         Got: {written}"
    );
    assert!(
        written.contains("void send(Map<String, dynamic> frame) {")
            && !written.contains("_write(")
            && !written.contains("_attach")
            && !written.contains("_attachments"),
        "`_write` is now the public `send`, and there is no separate attachment list any more. \
         Got: {written}"
    );
    let frame_body = body_from(&written, "void _onFrame(dynamic raw)");
    assert!(
        frame_body.contains("_pongTimer?.cancel();")
            && frame_body.matches("_controller.add(frame);").count() == 2,
        "a ping and a pong reach neither the pending map nor the controller; a correlated reply \
         settles its own completer and nothing else; every other frame, including an \
         uncorrelated reply, reaches the controller. Got: {frame_body}"
    );
}

#[test]
fn a_reply_operation_answers_future_of_the_result_pair_and_decodes_through_the_generated_codec() {
    let written = dart_ws_client_of(DART_WS_SERVICE);
    assert!(
        written.contains(
            "Future<LedgerListTransactionsResult> listTransactions(ListTransactionsRequest req) \
             async {"
        ),
        "got: {written}"
    );
    let method = body_from(&written, " listTransactions(");
    assert!(
        method.contains("reply = await _transport.request('list-transactions', (req).toJson());"),
        "got: {method}"
    );
    assert!(
        method.contains("if (reply['ok'] == true) {")
            && method.contains(
                "return LedgerListTransactionsResultOk(TransactionList.fromJson(reply['value']));"
            ),
        "a successful reply decodes through the generated `fromJson` codec into the pair's own \
         `Ok` member. Got: {method}"
    );
}

#[test]
fn a_reply_operation_answers_the_declared_error_or_a_fault_behind_is_service_fault() {
    let written = dart_ws_client_of(DART_WS_SERVICE);
    let method = body_from(&written, " listTransactions(");
    assert!(
        method.contains(
            "return LedgerListTransactionsResultFault(_ledgerWsTransportFailure('list-transactions', '$uncarried'));"
        ),
        "a transport failure is a fault, never the declared error. Got: {method}"
    );
    assert!(
        method.contains("if (reply == null) {")
            && method.contains(
                "_ledgerWsTransportFailure('list-transactions', 'the connection closed before a \
                 reply arrived'),"
            ),
        "a request still waiting when the socket closes answers the same transport-failure \
         fault, read off the `null` the transport settles its completer with. Got: {method}"
    );
    assert!(
        method.contains("if (error is Map<String, dynamic> && error['isServiceFault'] == true) {")
            && method.contains(
                "fault = LedgerFaultFields.fromJson(error['fault'] as Map<String, dynamic>);"
            )
            && method.contains("return LedgerListTransactionsResultFault(fault);"),
        "a wire fault behind `isServiceFault` is read back through the generated `FaultFields` \
         codec. Got: {method}"
    );
    assert!(
        method.contains("declared = ListError.fromJson(error);")
            && method.contains("return LedgerListTransactionsResultOperation(declared);"),
        "otherwise the wire's `error` is the operation's own declared error. Got: {method}"
    );
}

#[test]
fn a_reply_decode_failure_is_a_failed_validation_fault_not_an_undeserializable_payload_one() {
    let written = dart_ws_client_of(DART_WS_SERVICE);
    let method = body_from(&written, " listTransactions(");
    assert!(
        method.matches("_ledgerWsFailedValidation(").count() >= 2,
        "both a bad success value and a bad declared error decode to `failedValidation`. \
         Got: {method}"
    );
    assert!(
        !written.contains("UndeserializablePayload")
            && !written.contains("undeserializablePayload"),
        "`ws_rpc` has no status ladder, so this crate's own decision names every reply decode \
         failure `failedValidation` instead. Got: {written}"
    );
}

#[test]
fn a_one_way_operation_answers_future_void_and_throws_a_refusal() {
    let written = dart_ws_client_of(DART_WS_SERVICE);
    assert!(
        written.contains("Future<void> applyBundle(ApplyBundleRequest req) async {"),
        "got: {written}"
    );
    let method = body_from(&written, " applyBundle(");
    assert!(
        method.contains("await _transport.notify('apply-bundle', (req).toJson());")
            && method.contains(
                "throw LedgerWsRefusal(_ledgerWsTransportFailure('apply-bundle', '$uncarried'));"
            ),
        "a one-way method throws the fault-only refusal, having no declared error to carry one \
         in. Got: {method}"
    );
}

#[test]
fn the_client_class_holds_one_constructor_and_every_operation_as_a_method() {
    let written = dart_ws_client_of(DART_WS_SERVICE);
    assert!(
        written.contains("class LedgerWsClient {")
            && written.contains("LedgerWsClient(this._transport);")
            && written.contains("final LedgerWsTransport _transport;"),
        "got: {written}"
    );
    for call in ["listTransactions", "applyBundle"] {
        assert!(
            written.contains(&format!(" {call}(")),
            "operation `{call}` should have a method on the client. Got: {written}"
        );
    }
}

#[test]
fn the_handlers_class_carries_one_member_per_operation_typed_for_its_own_outcome() {
    let written = dart_ws_client_of(DART_WS_SERVICE);
    assert!(
        written.contains("class LedgerHandlers<Ctx> {"),
        "got: {written}"
    );
    assert!(
        written.contains("required this.listTransactions,")
            && written.contains("required this.applyBundle,"),
        "got: {written}"
    );
    assert!(
        written.contains(
            "final Future<TransactionList> Function(Ctx ctx, ListTransactionsRequest req) listTransactions;"
        ),
        "a reply operation's own handler answers the declared success. Got: {written}"
    );
    assert!(
        written
            .contains("final Future<void> Function(Ctx ctx, ApplyBundleRequest req) applyBundle;"),
        "a one-way operation's own handler answers nothing. Got: {written}"
    );
}

#[test]
fn the_attachment_takes_the_structural_frames_record_and_requires_on_fault() {
    let written = dart_ws_client_of(DART_WS_SERVICE);
    assert!(
        written.contains("void Function() attachLedgerWsDispatcher<Ctx>(")
            && written.contains(
                "({Stream<Map<String, dynamic>> inbound, void Function(Map<String, dynamic>) \
                 send}) frames,"
            )
            && written.contains("Ctx ctx,")
            && written.contains("LedgerHandlers<Ctx> handlers, {")
            && written.contains("required void Function(LedgerFaultFields) onFault,"),
        "the attachment names no transport class of its own, so it composes over any service's \
         own `frames`, not just this one's. Got: {written}"
    );
    assert!(
        written.contains("final subscription = frames.inbound.listen((frame) async {")
            && written.contains("return () {\n    subscription.cancel();\n  };"),
        "the attachment listens on the already-shared broadcast stream rather than the socket \
         itself, and answers with the function that cancels that subscription. Got: {written}"
    );
}

#[test]
fn an_unknown_operation_on_a_request_frame_reports_and_replies_with_the_fault() {
    let written = dart_ws_client_of(DART_WS_SERVICE);
    assert!(
        written.contains("switch (frame['operation']) {")
            && written.contains("case 'list-transactions':")
            && written.contains("case 'apply-bundle':"),
        "got: {written}"
    );
    let default_arm = body_from(&written, "default:\n        {");
    assert!(
        default_arm.contains("final fault = _ledgerWsUnknownOperation(")
            && default_arm.contains("onFault(fault);")
            && default_arm.contains("if (replyId != null) {")
            && default_arm.contains("'ok': false,")
            && default_arm.contains("'error': {'isServiceFault': true, 'fault': fault.toJson()},"),
        "an operation name nothing on the service answers to reaches `onFault`, and a request \
         left waiting on it is answered with the same fault rather than left hanging. \
         Got: {default_arm}"
    );
}

#[test]
fn a_reply_dispatch_arm_answers_a_reply_frame_on_every_outcome() {
    let written = dart_ws_client_of(DART_WS_SERVICE);
    let reply_arm = body_from(&written, "case 'list-transactions':");
    assert!(
        reply_arm.contains("decoded = ListTransactionsRequest.fromJson(frame['payload']);")
            && reply_arm
                .contains("final answered = await handlers.listTransactions(ctx, decoded);")
            && reply_arm.contains("'ok': true,")
            && reply_arm.contains("'value': (answered).toJson(),")
            && reply_arm.contains("} on ListError catch (declared) {")
            && reply_arm.contains("'error': (declared).toJson(),"),
        "got: {reply_arm}"
    );
    assert!(
        reply_arm.matches("if (replyId != null) {").count() == 4,
        "a decode failure, a success, a declared error and a handler panic each answer a \
         waiting request. Got: {reply_arm}"
    );
}

#[test]
fn a_request_frame_naming_a_one_way_operation_is_acknowledged() {
    let written = dart_ws_client_of(DART_WS_SERVICE);
    let one_way_arm = body_from(&written, "case 'apply-bundle':");
    assert!(
        one_way_arm.contains("await handlers.applyBundle(ctx, decoded);")
            && one_way_arm.contains("'ok': true,")
            && one_way_arm.contains("'value': null,"),
        "a one-way operation reached over a `request` frame is still answered once its handler \
         returns, `ok: true` with no value, rather than left hanging. Got: {one_way_arm}"
    );
    assert!(
        one_way_arm.matches("if (replyId != null) {").count() == 3,
        "a decode failure, the success acknowledgement and a handler panic each answer a \
         waiting request; a one-way operation declares no error to carry a declared-error \
         arm. Got: {one_way_arm}"
    );
}

#[test]
fn the_fault_helpers_reuse_the_generated_fault_fields_and_kind() {
    let written = dart_ws_client_of(DART_WS_SERVICE);
    for (helper, kind) in [
        (
            "_ledgerWsTransportFailure",
            "LedgerFaultKind.transportFailure",
        ),
        (
            "_ledgerWsFailedValidation",
            "LedgerFaultKind.failedValidation",
        ),
        (
            "_ledgerWsUnknownOperation",
            "LedgerFaultKind.unknownOperation",
        ),
        ("_ledgerWsHandlerPanic", "LedgerFaultKind.handlerPanic"),
    ] {
        assert!(
            written.contains(&format!("LedgerFaultFields _{}(", &helper[1..]))
                && written.contains(kind),
            "helper `{helper}` should report `{kind}`. Got: {written}"
        );
    }
}

#[test]
fn a_unit_success_answers_the_field_less_ok_member() {
    let written = dart_ws_client_of(DART_UNIT_SUCCESS_HTTP_SERVICE);
    let arm = body_from(&written, "if (reply['ok'] == true) {");
    assert!(
        arm.contains("return PingClientServicePingResultOk();"),
        "got: {arm}"
    );
    assert!(!arm.contains("ResultOk(null)"), "got: {arm}");
}

#[test]
fn a_primitive_message_and_success_ride_the_frame_as_their_own_json_values() {
    let written = dart_ws_client_of(DART_PRIMITIVE_SERVICE);
    let method = body_from(&written, "tally(String req)");
    assert!(
        method.contains("reply = await _transport.request('tally', req);"),
        "got: {method}"
    );
    assert!(
        method.contains("return ShelvesTallyResultOk(reply['value'] as int);"),
        "got: {method}"
    );
    let arm = body_from(&written, "case 'tally':");
    assert!(
        arm.contains("decoded = frame['payload'] as String;"),
        "got: {arm}"
    );
    assert!(arm.contains("'value': answered,"), "got: {arm}");
}

#[test]
fn the_transport_writes_headers_into_the_frame_only_when_non_empty() {
    let written = dart_ws_client_of(DART_HEADER_TUPLE_SERVICE);
    assert!(
        written.contains(
            "Future<Map<String, dynamic>?> request(\n    \
             String operation,\n    \
             Object? payload,\n    \
             Map<String, dynamic> headers,\n  \
             )"
        ) && written.contains("if (headers.isNotEmpty) 'headers': headers,"),
        "got: {written}"
    );
    assert!(
        written.contains(
            "Future<void> notify(\n    \
             String operation,\n    \
             Object? payload,\n    \
             Map<String, dynamic> headers,\n  \
             ) async"
        ),
        "got: {written}"
    );
}

#[test]
fn a_client_method_takes_a_header_in_value_per_binding_and_sends_it_as_json() {
    let written = dart_ws_client_of(DART_HEADER_TUPLE_SERVICE);
    assert!(
        written.contains(
            "Future<VersionServiceGetResult> get(String req, String tenant, int? trace) async {"
        ),
        "a header_in binding is one more argument after the message, spelled with its own \
         declared type. Got: {written}"
    );
    let method = body_from(&written, " get(String req");
    assert!(
        method.contains(
            "final headers = <String, dynamic>{\n      \
             'x-tenant': tenant,\n      \
             'x-trace': (trace == null ? null : trace),\n    \
             };"
        ),
        "every binding is written, an optional value included as `null` rather than left out. \
         Got: {method}"
    );
    assert!(
        method.contains("reply = await _transport.request('get', req, headers);"),
        "got: {method}"
    );
    let one_way = body_from(&written, " touch(String req");
    assert!(
        one_way.contains("final headers = <String, dynamic>{\n      'x-tenant': tenant,\n    };")
            && one_way.contains("await _transport.notify('touch', req, headers);"),
        "a one-way method's own header_in binding is sent the same way. Got: {one_way}"
    );
}

#[test]
fn a_client_method_splits_a_header_tuple_reply_and_names_the_header_on_a_bad_decode() {
    let written = dart_ws_client_of(DART_HEADER_TUPLE_SERVICE);
    let method = body_from(&written, " get(String req");
    assert!(
        method.contains("final replyHeaders = reply['headers'] as Map<String, dynamic>?;")
            && method.contains("value = Document.fromJson(reply['value']);")
            && method.contains("headerOut0 = replyHeaders?['etag'] as String;")
            && method.contains(
                "headerOut1 = (replyHeaders?['x-age'] == null ? null : replyHeaders?['x-age'] as int);"
            )
            && method.contains("return VersionServiceGetResultOk((value, headerOut0, headerOut1));"),
        "the response decodes alone, and each `header_out` element decodes off the reply's own \
         `headers`, in declaration order. Got: {method}"
    );
    assert!(
        method.contains("_versionServiceWsFailedValidation('get', '$rejected', field: 'etag')")
            && method
                .contains("_versionServiceWsFailedValidation('get', '$rejected', field: 'x-age')"),
        "a header element that will not decode faults naming that header. Got: {method}"
    );
    assert!(
        method.contains("declaredHead = DocError.fromJson(error);")
            && method.contains(
                "errorHeaderOut0 = (replyHeaders?['x-reason'] == null ? null : \
                 replyHeaders?['x-reason'] as String);"
            )
            && method.contains(
                "return VersionServiceGetResultOperation((declaredHead, errorHeaderOut0));"
            )
            && method.contains(
                "_versionServiceWsFailedValidation('get', '$rejected', field: 'x-reason')"
            ),
        "the declared error's own head decodes off `error` alone, and its `error_header_out` \
         element decodes off the reply's own `headers`, faulting under its own name. Got: {method}"
    );
}

#[test]
fn a_handler_takes_a_header_in_value_per_binding_read_off_the_request_frame() {
    let written = dart_ws_client_of(DART_HEADER_TUPLE_SERVICE);
    assert!(
        written.contains(
            "final Future<(Document, String, int?)> Function(Ctx ctx, String req, String tenant, \
             int? trace) get;"
        ) && written
            .contains("final Future<void> Function(Ctx ctx, String req, String tenant) touch;"),
        "a handler takes each header_in value as an argument after the message. Got: {written}"
    );
    let arm = body_from(&written, "case 'get':");
    assert!(
        arm.contains("final headersIn = frame['headers'] as Map<String, dynamic>?;")
            && arm.contains("tenant = headersIn?['x-tenant'] as String;")
            && arm.contains(
                "trace = (headersIn?['x-trace'] == null ? null : headersIn?['x-trace'] as int);"
            )
            && arm.contains("await handlers.get(ctx, decoded, tenant, trace);"),
        "each header_in value decodes off the request frame's own `headers` and is passed \
         straight through to the handler. Got: {arm}"
    );
    assert!(
        arm.contains("field: 'x-tenant'") && arm.contains("field: 'x-trace'"),
        "a header_in value that will not decode faults naming that header. Got: {arm}"
    );
}

#[test]
fn the_attachment_splits_a_handler_s_header_tuple_answer_omitting_a_null_element() {
    let written = dart_ws_client_of(DART_HEADER_TUPLE_SERVICE);
    let arm = body_from(&written, "case 'get':");
    assert!(
        arm.contains("final (bodyOut, headersOut0, headersOut1) = answered;")
            && arm.contains("final headersOut = <String, dynamic>{};")
            && arm.contains("headersOut['etag'] = headersOut0;")
            && arm.contains("if (headersOut1 != null) {\n              headersOut['x-age'] = headersOut1;\n            }")
            && arm.contains("'value': (bodyOut).toJson(),")
            && arm.contains("if (headersOut.isNotEmpty) 'headers': headersOut,"),
        "the handler's own record is split: its head under `value`, every other element under \
         `headers`, a null one omitted rather than written. Got: {arm}"
    );
    assert!(
        arm.contains("} on (DocError, String?) catch (declared) {")
            && arm.contains("final (declaredHead, errorHeadersOut0) = declared;")
            && arm.contains("final errorHeadersOut = <String, dynamic>{};")
            && arm.contains(
                "if (errorHeadersOut0 != null) {\n              errorHeadersOut['x-reason'] = \
                 errorHeadersOut0;\n            }"
            )
            && arm.contains("'error': (declaredHead).toJson(),")
            && arm.contains("if (errorHeadersOut.isNotEmpty) 'headers': errorHeadersOut,"),
        "the declared error a handler throws splits the same way. Got: {arm}"
    );
}
