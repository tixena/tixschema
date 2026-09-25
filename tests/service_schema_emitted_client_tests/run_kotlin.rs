//! The emitted Kotlin run under a Kotlin toolchain: the codec rows the Kotlin spike proved,
//! decoded from the JSON Rust wrote and re-encoded; the `http_rest` client's own URLs (the same
//! three `run_dart.rs` asserts); the `ws_rpc` client's own scenarios against a fake socket; and
//! the mini server, sharing its socket with a second service, against frames sent by hand.
//!
//! Every group compiles one `main.kt` with `kotlinc` and runs the result with `java` — no package
//! manifest, no Gradle — standing down exactly as [`super::run_dart`] does where no Kotlin
//! toolchain is reachable. A single fake socket class carries every `ws_rpc` scenario: unpaired
//! and fed by hand for the client group, paired for the mini-server group's own live connection.

#![cfg(feature = "kotlin")]

use super::runtime::ran_kotlin;
use super::tests::conversation_client_service_schema::{
    conversation_client_service_fault_fields_kotlin, conversation_client_service_fault_kind_kotlin,
};
use super::tests::echo_client_service_schema::{
    echo_client_service_fault_fields_kotlin, echo_client_service_fault_kind_kotlin,
};
use super::tests::pulse_client_service_schema::{
    pulse_client_service_fault_fields_kotlin, pulse_client_service_fault_kind_kotlin,
};
use super::tests::swift_codec_fixture::{
    CodecAdjacentTagged, CodecEnvelope, CodecExternalTagged, CodecInternalTagged, CodecMapKeys,
    CodecOptionsRow, CodecPrimary, CodecTuplePoint, CodecUnitField, CodecUnitPayload,
    CodecUntagged, codec_adjacent_tagged_kotlin, codec_envelope_kotlin,
    codec_external_tagged_kotlin, codec_internal_tagged_kotlin, codec_map_keys_kotlin,
    codec_options_row_kotlin, codec_primary_kotlin, codec_tuple_point_kotlin,
    codec_unit_field_kotlin, codec_unit_payload_kotlin, codec_untagged_kotlin,
};
use super::tests::thumbnail_client_service_schema::{
    thumbnail_client_service_fault_fields_kotlin, thumbnail_client_service_fault_kind_kotlin,
};
use super::tests::{
    ConversationClientServiceSchema, EchoClientServiceSchema, PulseClientServiceSchema,
    ThumbnailClientServiceSchema, conversation_id_kotlin, echo_range_error_kotlin,
    echo_range_response_kotlin, pulse_error_kotlin, pulse_request_kotlin, pulse_response_kotlin,
    thumbnail_error_kotlin, window_error_kotlin, window_page_kotlin, window_request_kotlin,
};
use std::collections::HashMap;

/// Every import a driver in this file reaches for, across all four groups — `kotlinx.serialization`
/// needs the `descriptors`/`encoding` packages by name; a wildcard on the top-level package alone
/// leaves `SerialDescriptor`, `Encoder` and `Decoder` unresolved.
const KOTLIN_IMPORTS: &str = "import kotlinx.coroutines.*\n\
     import kotlinx.coroutines.flow.*\n\
     import kotlinx.serialization.*\n\
     import kotlinx.serialization.json.*\n\
     import kotlinx.serialization.descriptors.*\n\
     import kotlinx.serialization.encoding.*\n\
     import kotlinx.serialization.builtins.*";

/// Group 2's own driver: a recording `ConversationClientServiceHttpTransport` answering 200 for
/// every call but `DELETE`, which it answers 204 — then the three calls `run_dart.rs` makes of
/// the same three URLs, reported as one JSON array of `{method, path, query}`.
const REST_DRIVER: &str = r#"
class RecordingTransport : ConversationClientServiceHttpTransport {
    val sent = mutableListOf<ConversationClientServiceHttpRequest>()
    override suspend fun send(request: ConversationClientServiceHttpRequest): ConversationClientServiceHttpResponse {
        sent.add(request)
        if (request.method == "DELETE") return ConversationClientServiceHttpResponse(204, emptyList(), ByteArray(0))
        return ConversationClientServiceHttpResponse(200, emptyList(), """{"items":[]}""".encodeToByteArray())
    }
}

fun main() = runBlocking {
    val transport = RecordingTransport()
    val client = ConversationClientServiceHttpClient(transport)
    client.window(WindowRequest(conversationId = "652f1a3b4c5d6e7f8a9b0c1d", limit = 10u))
    client.window(WindowRequest(conversationId = "652f1a3b4c5d6e7f8a9b0c1d", limit = null))
    client.purgeConversation(ConversationId("652f1a3b4c5d6e7f8a9b0c1d"))
    val report = buildJsonArray {
        for (request in transport.sent) {
            add(buildJsonObject {
                put("method", request.method)
                put("path", request.path)
                put("query", request.query)
            })
        }
    }
    println(report.toString())
}
"#;

/// The one fake socket every `ws_rpc` group drives: `deliver` stands in for an inbound frame
/// arriving (group 3, unpaired), `peer` stands in for a live connection (group 4, paired). Each
/// group compiles its own separate `main.kt` — a fresh `java` process per group, never shared.
const FAKE_SOCKET: &str = r#"
class FakeSocket(private val scope: CoroutineScope) : ConversationClientServiceWsSocket {
    var peer: FakeSocket? = null
    val sent = mutableListOf<String>()
    var isClosed = false
        private set
    override var onMessage: ((String) -> Unit)? = null
    override var onClose: (() -> Unit)? = null

    override fun send(text: String) {
        if (isClosed) return
        sent.add(text)
        val target = peer ?: return
        scope.launch { target.deliver(text) }
    }

    fun deliver(text: String) {
        if (isClosed) return
        onMessage?.invoke(text)
    }

    override fun close() {
        if (isClosed) return
        isClosed = true
        val callback = onClose
        if (callback != null) scope.launch { callback() }
    }
}

fun requestId(sent: String?): String? {
    if (sent == null) return null
    val frame = Json.parseToJsonElement(sent).jsonObject
    return (frame["id"] as? JsonPrimitive)?.contentOrNull
}
"#;

/// Group 3's own driver: five scenarios against a fake socket fed by hand — a ping answered with
/// one pong, a missed pong closing the socket, a reply that fails validation, a request settled
/// when the transport closes, and a frame for another service dropped while the genuine reply resolves.
const WS_CLIENT_DRIVER: &str = r#"
fun main() = runBlocking {
    val pingSocket = FakeSocket(this)
    val pingTransport = ConversationClientServiceWsTransport(pingSocket, ConversationClientServiceWsOptions(heartbeat = null))
    pingSocket.deliver("""{"kind":"ping"}""")
    delay(50)
    val pong = pingSocket.sent.contains("""{"kind":"pong"}""")
    pingTransport.close()

    val heartbeatSocket = FakeSocket(this)
    val heartbeatStart = System.nanoTime()
    val heartbeatTransport = ConversationClientServiceWsTransport(
        heartbeatSocket,
        ConversationClientServiceWsOptions(heartbeat = ConversationClientServiceWsOptions.Heartbeat(150, 100)),
    )
    while (!heartbeatSocket.isClosed) delay(5)
    val closeElapsedMs = (System.nanoTime() - heartbeatStart) / 1_000_000
    heartbeatTransport.close()

    val invalidSocket = FakeSocket(this)
    val invalidTransport = ConversationClientServiceWsTransport(invalidSocket, ConversationClientServiceWsOptions(heartbeat = null))
    val invalidClient = ConversationClientServiceWsClient(invalidTransport)
    val invalidOutcome = async { invalidClient.window(WindowRequest("abc", null)) }
    delay(50)
    val invalidId = requestId(invalidSocket.sent.firstOrNull())!!
    invalidSocket.deliver("""{"kind":"reply","id":"$invalidId","service":"ConversationClientService","ok":true,"value":7}""")
    val failedValidationKind = (invalidOutcome.await() as ConversationClientServiceWindowResult.Fault).fault.kind
    invalidTransport.close()

    val closingSocket = FakeSocket(this)
    val closingTransport = ConversationClientServiceWsTransport(closingSocket, ConversationClientServiceWsOptions(heartbeat = null))
    val closingClient = ConversationClientServiceWsClient(closingTransport)
    val closingOutcome = async { closingClient.window(WindowRequest("abc", null)) }
    delay(50)
    closingTransport.close()
    val transportFailureKind = (closingOutcome.await() as ConversationClientServiceWindowResult.Fault).fault.kind

    val foreignSocket = FakeSocket(this)
    val foreignTransport = ConversationClientServiceWsTransport(foreignSocket, ConversationClientServiceWsOptions(heartbeat = null))
    val foreignClient = ConversationClientServiceWsClient(foreignTransport)
    val foreignOutcome = async { foreignClient.window(WindowRequest("abc", null)) }
    delay(50)
    val foreignId = requestId(foreignSocket.sent.firstOrNull())!!
    foreignSocket.deliver("""{"kind":"reply","id":"$foreignId","service":"OtherService","ok":true,"value":{"items":[]}}""")
    foreignSocket.deliver("""{"kind":"reply","id":"$foreignId","service":"ConversationClientService","ok":true,"value":{"items":[]}}""")
    val foreignResolvedOk = foreignOutcome.await() is ConversationClientServiceWindowResult.Ok
    foreignTransport.close()

    fun wire(kind: ConversationClientServiceFaultKind) =
        Json.encodeToJsonElement(serializer<ConversationClientServiceFaultKind>(), kind).jsonPrimitive.content

    val report = buildJsonObject {
        put("pong", pong)
        put("closeElapsedMs", closeElapsedMs)
        put("failedValidationKind", wire(failedValidationKind))
        put("transportFailureKind", wire(transportFailureKind))
        put("foreignResolvedOk", foreignResolvedOk)
    }
    println(report.toString())
}
"#;

/// Group 4's own driver: the mini server against a connected fake-socket pair — every frame that
/// carries an id answered, the one-way included; a declared error; a bad-payload notify reaching
/// `onFault` only; sharing the socket with `PulseClientService`; and `share()` throwing once detached.
const MINI_SERVER_DRIVER: &str = r#"
fun connectedPair(scope: CoroutineScope): Pair<FakeSocket, FakeSocket> {
    val a = FakeSocket(scope)
    val b = FakeSocket(scope)
    a.peer = b
    b.peer = a
    return a to b
}

object Handlers : ConversationClientServiceHandlers {
    override suspend fun window(req: WindowRequest): ConversationClientServiceWindowResult =
        if (req.conversationId == "archived") {
            ConversationClientServiceWindowResult.Declared(WindowErrorNotFound)
        } else {
            ConversationClientServiceWindowResult.Ok(WindowPage(items = listOf("a", "b")))
        }
    override suspend fun purgeConversation(req: ConversationId) {}
}

object PulseHandlers : PulseClientServiceHandlers {
    override suspend fun pulse(req: PulseRequest): PulseClientServicePulseResult =
        PulseClientServicePulseResult.Ok(PulseResponse(alive = true))
}

fun main() = runBlocking {
    val (clientSocket, serverSocket) = connectedPair(this)
    val clientTransport = ConversationClientServiceWsTransport(clientSocket, ConversationClientServiceWsOptions(heartbeat = null))
    val serverTransport = ConversationClientServiceWsTransport(serverSocket, ConversationClientServiceWsOptions(heartbeat = null))
    val client = ConversationClientServiceWsClient(clientTransport)

    val faults = mutableListOf<ConversationClientServiceFaultFields>()
    val attachment = attachConversationClientServiceWsDispatcher(serverTransport.frames, Handlers, onFault = { faults.add(it) })

    val ok = client.window(WindowRequest("abc")) is ConversationClientServiceWindowResult.Ok

    val oneWayReply = clientTransport.request(
        "purge-conversation",
        Json.encodeToJsonElement(serializer<ConversationId>(), ConversationId("xyz")),
    )
    val oneWayOk = oneWayReply != null &&
        (oneWayReply["ok"] as? JsonPrimitive)?.booleanOrNull == true &&
        oneWayReply["value"] is JsonNull

    val declared = client.window(WindowRequest("archived"))
    val declaredOk = declared is ConversationClientServiceWindowResult.Declared &&
        declared.error == WindowErrorNotFound

    val faultsBeforeNotify = faults.size
    clientTransport.notify("window", JsonPrimitive(7))
    delay(50)
    val notifyFaultReached = faults.size > faultsBeforeNotify

    var pulseDetachRan = false
    attachment.share { send, scope ->
        val pulseFrames = PulseClientServiceWsFrames(serverTransport.frames.inbound, send, scope)
        attachPulseClientServiceWsDispatcher(pulseFrames, PulseHandlers, onFault = {})
        ({ pulseDetachRan = true })
    }

    val observed = mutableListOf<String>()
    val collectJob = launch { clientTransport.frames.inbound.collect { observed.add(it) } }
    clientSocket.send("""{"kind":"request","id":"cA","service":"ConversationClientService","operation":"window","payload":{"conversationId":"abc"}}""")
    clientSocket.send("""{"kind":"request","id":"pA","service":"PulseClientService","operation":"pulse","payload":{}}""")
    delay(50)
    fun serviceOf(id: String) = observed
        .map { Json.parseToJsonElement(it).jsonObject }
        .firstOrNull { (it["id"] as? JsonPrimitive)?.contentOrNull == id }
        ?.get("service")?.jsonPrimitive?.contentOrNull
    val conversationAnsweredOwn = serviceOf("cA") == "ConversationClientService"
    val pulseAnsweredOwn = serviceOf("pA") == "PulseClientService"
    collectJob.cancel()

    val pongsBefore = serverSocket.sent.count { it == """{"kind":"pong"}""" }
    clientSocket.send("""{"kind":"ping"}""")
    delay(50)
    val onePongPerPing = serverSocket.sent.count { it == """{"kind":"pong"}""" } == pongsBefore + 1

    attachment.detach()
    val sharedDetached = pulseDetachRan
    val shareThrew = try {
        attachment.share { _, _ -> {} }
        false
    } catch (_: IllegalStateException) {
        true
    }

    val report = buildJsonObject {
        put("ok", ok)
        put("oneWayOk", oneWayOk)
        put("declaredOk", declaredOk)
        put("notifyFaultReached", notifyFaultReached)
        put("conversationAnsweredOwn", conversationAnsweredOwn)
        put("pulseAnsweredOwn", pulseAnsweredOwn)
        put("onePongPerPing", onePongPerPing)
        put("sharedDetached", sharedDetached)
        put("shareThrew", shareThrew)
    }
    println(report.toString())
}
"#;

/// A stub `ThumbnailClientServiceHttpTransport` answering by path alone, driving the emitted
/// client's own declared-error and `header_out` decode - the Kotlin twin of the Node, Dart and
/// Swift client tests on the same fixture.
const THUMBNAIL_DRIVER: &str = r#"
class ThumbnailRecorder : ThumbnailClientServiceHttpTransport {
    override suspend fun send(request: ThumbnailClientServiceHttpRequest): ThumbnailClientServiceHttpResponse {
        if (request.path == "/thumbnails/missing") {
            return ThumbnailClientServiceHttpResponse(404, listOf("x-thumbnail-reason" to "archived"), """{"errorCode":"not-found"}""".encodeToByteArray())
        }
        if (request.path == "/thumbnails/gone") {
            return ThumbnailClientServiceHttpResponse(404, emptyList(), """{"errorCode":"not-found"}""".encodeToByteArray())
        }
        return ThumbnailClientServiceHttpResponse(200, listOf("content-type" to "image/png"), "PNGDATA".encodeToByteArray())
    }
}

fun describeThumbnail(result: ThumbnailClientServiceGetThumbnailResult): JsonObject = buildJsonObject {
    when (result) {
        is ThumbnailClientServiceGetThumbnailResult.Ok -> {
            put("kind", "ok")
            put("headerOut", result.value.headerOut0)
        }
        is ThumbnailClientServiceGetThumbnailResult.Declared -> {
            put("kind", "operation")
            put("errorHeaderOut", result.error.errorHeaderOut0)
        }
        is ThumbnailClientServiceGetThumbnailResult.Fault -> {
            put("kind", "fault")
        }
    }
}

fun main() = runBlocking {
    val client = ThumbnailClientServiceHttpClient(ThumbnailRecorder())
    val missing = client.getThumbnail("missing")
    val gone = client.getThumbnail("gone")
    val anon = client.getThumbnail("anon")
    val report = buildJsonObject {
        put("missing", describeThumbnail(missing))
        put("gone", describeThumbnail(gone))
        put("anon", describeThumbnail(anon))
    }
    println(report.toString())
}
"#;

/// A stub `EchoClientServiceHttpTransport` recording whether `send` was ever called, driving the
/// emitted client's own `header_in` legality check on a value carrying a line feed.
const ECHO_DRIVER: &str = r#"
class EchoRecorder : EchoClientServiceHttpTransport {
    var sendCalled = false
    override suspend fun send(request: EchoClientServiceHttpRequest): EchoClientServiceHttpResponse {
        sendCalled = true
        return EchoClientServiceHttpResponse(200, emptyList(), """{"received":"unreachable"}""".encodeToByteArray())
    }
}

fun main() = runBlocking {
    val recorder = EchoRecorder()
    val client = EchoClientServiceHttpClient(recorder)
    val refused = client.echoRange("doc-1", "bytes=0-10\nX-Injected: yes")
    val faultKind = if (refused is EchoClientServiceEchoRangeResult.Fault) refused.fault.kind.name else null
    val report = buildJsonObject {
        put("sendCalled", recorder.sendCalled)
        put("faultKind", faultKind)
    }
    println(report.toString())
}
"#;

// -------------------------------------------------------------------------------------------
// The generated text every group but the codec one drives: `ConversationClientService`'s own
// declared types, its fault pair, and both of its clients.
// -------------------------------------------------------------------------------------------

fn client_definitions() -> Vec<String> {
    vec![
        conversation_id_kotlin::kotlin_definition(),
        window_request_kotlin::kotlin_definition(),
        window_page_kotlin::kotlin_definition(),
        window_error_kotlin::kotlin_definition(),
        conversation_client_service_fault_fields_kotlin::kotlin_definition(),
        conversation_client_service_fault_kind_kotlin::kotlin_definition(),
        ConversationClientServiceSchema::kotlin_http_client(),
        ConversationClientServiceSchema::kotlin_ws_client(),
    ]
}

/// `PulseClientService`'s own declared types, its fault pair, and both of its clients — the
/// mini-server group's second, sharing service.
fn pulse_definitions() -> Vec<String> {
    vec![
        pulse_request_kotlin::kotlin_definition(),
        pulse_response_kotlin::kotlin_definition(),
        pulse_error_kotlin::kotlin_definition(),
        pulse_client_service_fault_fields_kotlin::kotlin_definition(),
        pulse_client_service_fault_kind_kotlin::kotlin_definition(),
        PulseClientServiceSchema::kotlin_http_client(),
        PulseClientServiceSchema::kotlin_ws_client(),
    ]
}

/// `ThumbnailClientService`'s own declared error type, its fault pair, and its client - the
/// `error_header_out`/`header_out` `Option<T>`-omission group's own fixture.
fn thumbnail_definitions() -> Vec<String> {
    vec![
        thumbnail_error_kotlin::kotlin_definition(),
        thumbnail_client_service_fault_fields_kotlin::kotlin_definition(),
        thumbnail_client_service_fault_kind_kotlin::kotlin_definition(),
        ThumbnailClientServiceSchema::kotlin_http_client(),
    ]
}

fn client_module(driver: &str) -> String {
    let mut parts = vec![KOTLIN_IMPORTS.to_owned()];
    parts.extend(client_definitions());
    parts.push(driver.to_owned());
    parts.join("\n\n")
}

fn thumbnail_module(driver: &str) -> String {
    let mut parts = vec![KOTLIN_IMPORTS.to_owned()];
    parts.extend(thumbnail_definitions());
    parts.push(driver.to_owned());
    parts.join("\n\n")
}

/// Like [`client_module`], with `PulseClientService`'s own text alongside — the mini-server
/// group's only user.
fn mini_server_module(driver: &str) -> String {
    let mut parts = vec![KOTLIN_IMPORTS.to_owned()];
    parts.extend(client_definitions());
    parts.extend(pulse_definitions());
    parts.push(driver.to_owned());
    parts.join("\n\n")
}

// -------------------------------------------------------------------------------------------
// Group 1: the codec rows the Kotlin spike proved, decoded from the JSON Rust wrote and
// re-encoded.
// -------------------------------------------------------------------------------------------

/// One row: the name it prints under, the Kotlin type it decodes into, and the JSON Rust wrote
/// for it — the same value the printed, re-encoded JSON must equal.
fn codec_rows() -> Vec<(&'static str, &'static str, serde_json::Value)> {
    vec![
        (
            "options",
            "CodecOptionsRow",
            serde_json::to_value(CodecOptionsRow {
                nullable_field: None,
                omitted_optional: None,
                plain_field: "hello".to_owned(),
                present_optional: Some(9),
            })
            .unwrap(),
        ),
        (
            "external_tagged",
            "CodecExternalTagged",
            serde_json::to_value(CodecExternalTagged::Foo {
                a: "ex-a".to_owned(),
            })
            .unwrap(),
        ),
        (
            "internal_tagged",
            "CodecInternalTagged",
            serde_json::to_value(CodecInternalTagged::Foo {
                a: "in-a".to_owned(),
            })
            .unwrap(),
        ),
        (
            "adjacent_tagged",
            "CodecAdjacentTagged",
            serde_json::to_value(CodecAdjacentTagged::Flag(true)).unwrap(),
        ),
        (
            "untagged",
            "CodecUntagged",
            serde_json::to_value(CodecUntagged::Text {
                text: "hi".to_owned(),
            })
            .unwrap(),
        ),
        (
            "tuple_point",
            "CodecTuplePoint",
            serde_json::to_value(CodecTuplePoint {
                label: "pt".to_owned(),
                pair: ("a".to_owned(), 7),
            })
            .unwrap(),
        ),
        (
            "envelope",
            "CodecEnvelope<String>",
            serde_json::to_value(CodecEnvelope {
                note: "n1".to_owned(),
                value: "payload".to_owned(),
            })
            .unwrap(),
        ),
        (
            "map_keys",
            "CodecMapKeys",
            serde_json::to_value(CodecMapKeys {
                counters: HashMap::from([(7, "seven".to_owned())]),
                tiers: HashMap::from([(CodecPrimary::Primary, "p".to_owned())]),
            })
            .unwrap(),
        ),
        (
            "unit_field",
            "CodecUnitField",
            serde_json::to_value(CodecUnitField {
                label: "marker".to_owned(),
                payload: CodecUnitPayload,
            })
            .unwrap(),
        ),
    ]
}

/// Rust's `"` and `\` are the only characters any row above can write, so escaping just those two
/// is enough to carry `json` inside a Kotlin triple-quote-free string literal.
fn kotlin_string_literal(json: &str) -> String {
    format!("\"{}\"", json.replace('\\', "\\\\").replace('"', "\\\""))
}

/// One `decode/re-encode/print` block per row, each guarded so a decode failure prints a message
/// rather than aborting the rows after it.
fn codec_driver(rows: &[(&str, &str, serde_json::Value)]) -> String {
    let blocks = rows
        .iter()
        .map(|(name, kotlin_type, value)| {
            let literal = kotlin_string_literal(&value.to_string());
            format!(
                "  try {{\n    \
                 val decoded = Json.decodeFromString(serializer<{kotlin_type}>(), {literal})\n    \
                 println(\"{name}\\t\" + Json.encodeToString(serializer<{kotlin_type}>(), decoded))\n  \
                 }} catch (rejected: Throwable) {{\n    \
                 println(\"{name}\\tERROR $rejected\")\n  \
                 }}"
            )
        })
        .collect::<Vec<_>>()
        .join("\n\n");
    format!("fun main() {{\n{blocks}\n}}")
}

fn codec_module(driver: &str) -> String {
    let mut parts = vec![KOTLIN_IMPORTS.to_owned()];
    parts.extend([
        codec_options_row_kotlin::kotlin_definition(),
        codec_external_tagged_kotlin::kotlin_definition(),
        codec_internal_tagged_kotlin::kotlin_definition(),
        codec_adjacent_tagged_kotlin::kotlin_definition(),
        codec_untagged_kotlin::kotlin_definition(),
        codec_tuple_point_kotlin::kotlin_definition(),
        codec_envelope_kotlin::kotlin_definition(),
        codec_primary_kotlin::kotlin_definition(),
        codec_map_keys_kotlin::kotlin_definition(),
        codec_unit_payload_kotlin::kotlin_definition(),
        codec_unit_field_kotlin::kotlin_definition(),
    ]);
    parts.push(driver.to_owned());
    parts.join("\n\n")
}

#[test]
fn every_awkward_shape_round_trips_through_the_serializer() {
    let rows = codec_rows();
    let driver = codec_driver(&rows);
    let Some(written) = ran_kotlin(&codec_module(&driver)) else {
        return;
    };
    for (name, _, expected) in &rows {
        let prefix = format!("{name}\t");
        let found = written.lines().find(|line| line.starts_with(&prefix));
        assert!(
            found.is_some(),
            "no printed line for row `{name}`. Got:\n{written}"
        );
        let line = found.unwrap();
        let payload = &line[prefix.len()..];
        assert!(
            !payload.starts_with("ERROR"),
            "row `{name}` did not decode and re-encode: {payload}"
        );
        let parsed = serde_json::from_str::<serde_json::Value>(payload);
        assert!(
            parsed.is_ok(),
            "row `{name}` printed invalid JSON `{payload}`: {parsed:?}"
        );
        let actual = parsed.unwrap();
        assert_eq!(
            &actual, expected,
            "row `{name}` round-tripped to a different value. Got: {actual:#?}"
        );
    }
}

// -------------------------------------------------------------------------------------------
// Group 2: the REST client's own URLs — the same three `run_dart.rs` asserts.
// -------------------------------------------------------------------------------------------

/// What the recorder captured, or `None` where no toolchain was reachable.
fn driven_rest() -> Option<Vec<serde_json::Value>> {
    let written = ran_kotlin(&client_module(REST_DRIVER))?;
    let parsed: serde_json::Value = serde_json::from_str(written.trim()).unwrap();
    assert!(parsed.is_array(), "expected a JSON array, got: {parsed:#?}");
    parsed.as_array().cloned()
}

#[test]
fn a_lone_placeholder_sends_the_field_it_names_and_the_rest_as_a_query() {
    let Some(sent) = driven_rest() else {
        return;
    };
    assert_eq!(
        sent[0]["path"], "/v1/conversations/652f1a3b4c5d6e7f8a9b0c1d/window",
        "the placeholder is filled by the field it names. Got: {sent:#?}"
    );
    assert!(
        !sent[0]["path"].as_str().unwrap().contains("%7B"),
        "rendering the whole message puts its own map spelling in the segment. Got: {sent:#?}"
    );
    assert_eq!(
        sent[0]["query"], "limit=10",
        "`limit` is bound to no placeholder, so the query string is the only place left for it. \
         Got: {sent:#?}"
    );
    assert_eq!(
        sent[1]["query"], "",
        "the same operation with `limit` absent sends no key for it rather than `limit=null`. \
         Got: {sent:#?}"
    );
    assert_eq!(
        sent[2]["path"], "/v1/conversations/652f1a3b4c5d6e7f8a9b0c1d",
        "a message that already is a wire scalar has no field to read: it is the segment. \
         Got: {sent:#?}"
    );
    assert_eq!(
        sent[2]["query"], "",
        "and no key is left over for a query. Got: {sent:#?}"
    );
}

// -------------------------------------------------------------------------------------------
// Group 3: the `ws_rpc` client against a fake socket fed by hand.
// -------------------------------------------------------------------------------------------

/// What the driver's own report printed, or `None` where no toolchain was reachable.
fn driven_ws_client() -> Option<serde_json::Value> {
    let written = ran_kotlin(&client_module(&format!(
        "{FAKE_SOCKET}\n\n{WS_CLIENT_DRIVER}"
    )))?;
    Some(serde_json::from_str(written.trim()).unwrap())
}

#[test]
fn the_ws_client_probes_answers_checks_and_settles() {
    let Some(written) = driven_ws_client() else {
        return;
    };
    assert_eq!(written["pong"], true, "got: {written:#?}");
    let elapsed = written["closeElapsedMs"].as_i64().unwrap();
    assert!(
        (250..=400).contains(&elapsed),
        "a 150 ms interval plus a 100 ms timeout should close the socket around 250 ms in, got \
         {elapsed} ms. Full output: {written:#?}"
    );
    assert_eq!(
        written["failedValidationKind"], "failed-validation",
        "got: {written:#?}"
    );
    assert_eq!(
        written["transportFailureKind"], "transport-failure",
        "got: {written:#?}"
    );
    assert_eq!(written["foreignResolvedOk"], true, "got: {written:#?}");
}

// -------------------------------------------------------------------------------------------
// Group 4: the mini server, sharing its socket with `PulseClientService`.
// -------------------------------------------------------------------------------------------

#[test]
fn the_mini_server_answers_every_frame_with_an_id_and_shares_the_socket() {
    let Some(written) = ran_kotlin(&mini_server_module(&format!(
        "{FAKE_SOCKET}\n\n{MINI_SERVER_DRIVER}"
    ))) else {
        return;
    };
    let report: serde_json::Value = serde_json::from_str(written.trim()).unwrap();
    for key in [
        "ok",
        "oneWayOk",
        "declaredOk",
        "notifyFaultReached",
        "conversationAnsweredOwn",
        "pulseAnsweredOwn",
        "onePongPerPing",
        "sharedDetached",
        "shareThrew",
    ] {
        assert_eq!(report[key], true, "`{key}` was false. Got: {report:#?}");
    }
}

#[test]
fn the_client_decodes_the_declared_errors_own_header_and_omits_a_none_header_out_element() {
    let Some(written) = ran_kotlin(&thumbnail_module(THUMBNAIL_DRIVER)) else {
        return;
    };
    let results: serde_json::Value = serde_json::from_str(written.trim()).unwrap();
    assert_eq!(results["missing"]["kind"], "operation", "got: {results:#?}");
    assert_eq!(
        results["missing"]["errorHeaderOut"], "archived",
        "the declared error's own `error_header_out` element must decode. got: {results:#?}"
    );
    assert_eq!(results["gone"]["kind"], "operation", "got: {results:#?}");
    assert!(
        results["gone"]["errorHeaderOut"].is_null(),
        "an absent `error_header_out` header must decode as `null`. got: {results:#?}"
    );
    assert_eq!(results["anon"]["kind"], "ok", "got: {results:#?}");
    assert!(
        results["anon"]["headerOut"].is_null(),
        "an absent `header_out` header must decode as `null`. got: {results:#?}"
    );
}

/// `EchoClientService`'s own declared types, its fault pair, and its client - the `header_in`
/// line-feed-refusal group's own fixture.
fn echo_definitions() -> Vec<String> {
    vec![
        echo_range_response_kotlin::kotlin_definition(),
        echo_range_error_kotlin::kotlin_definition(),
        echo_client_service_fault_fields_kotlin::kotlin_definition(),
        echo_client_service_fault_kind_kotlin::kotlin_definition(),
        EchoClientServiceSchema::kotlin_http_client(),
    ]
}

fn echo_module(driver: &str) -> String {
    let mut parts = vec![KOTLIN_IMPORTS.to_owned()];
    parts.extend(echo_definitions());
    parts.push(driver.to_owned());
    parts.join("\n\n")
}

#[test]
fn a_header_in_value_with_a_line_feed_is_refused_before_the_transport_is_ever_reached() {
    let Some(written) = ran_kotlin(&echo_module(ECHO_DRIVER)) else {
        return;
    };
    let results: serde_json::Value = serde_json::from_str(written.trim()).unwrap();
    assert_eq!(
        results["sendCalled"], false,
        "an illegal `header_in` value must refuse before the transport is ever reached. \
         got: {results:#?}"
    );
    assert_eq!(
        results["faultKind"], "FailedValidation",
        "got: {results:#?}"
    );
}
