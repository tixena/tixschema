//! Headers both ways, run by a Kotlin toolchain against the Rust `ws_rpc` twins of
//! `StampClientService`: the emitted client writes the frames the Rust client writes, reads the
//! replies the Rust dispatcher writes, and the emitted mini server answers the Rust client's own
//! frames the way the Rust dispatcher does. One `kotlinc` invocation; `REPLIES`/`FRAMES` are
//! computed by the Rust side ahead of the run.

#![cfg(feature = "kotlin")]

use super::runtime::ran_kotlin;
use super::stamp_client_service_schema::{
    stamp_client_service_fault_fields_kotlin, stamp_client_service_fault_kind_kotlin,
};
use super::stamp_ws_rpc_client::FrameSession;
use super::stamp_ws_rpc_transport;
use super::tests::{
    StampBackEnd, StampClientServiceSchema, stamp_error_kotlin, stamp_receipt_kotlin,
};
use alloc::sync::Arc;
use core::fmt::Write as _;
use core::future::{Future, Ready, ready};
use core::pin::{Pin, pin};
use core::task::{Context as PollContext, Poll, Waker};
use serde_json::Value;
use std::sync::Mutex;

const KOTLIN_IMPORTS: &str = "import kotlinx.coroutines.*\n\
     import kotlinx.coroutines.flow.*\n\
     import kotlinx.serialization.*\n\
     import kotlinx.serialization.json.*\n\
     import kotlinx.serialization.descriptors.*\n\
     import kotlinx.serialization.encoding.*\n\
     import kotlinx.serialization.builtins.*";

/// `onSend`, when given, delivers a reply synchronously off the frame just sent — the transport
/// has already registered its pending deferred by the time `send` runs.
const MANUAL_SOCKET: &str = r#"
class ManualSocket(private val onSend: ((String) -> Unit)? = null) : StampClientServiceWsSocket {
    // Dispatched frames run on separate Dispatchers.Default threads; synchronized avoids lost writes.
    val sent = java.util.Collections.synchronizedList(mutableListOf<String>())
    override var onMessage: ((String) -> Unit)? = null
    override var onClose: (() -> Unit)? = null
    override fun send(text: String) {
        sent.add(text)
        onSend?.invoke(text)
    }
    override fun close() {
        onClose?.invoke()
    }
    fun deliver(text: String) {
        onMessage?.invoke(text)
    }
}

fun requestId(sent: String?): String? {
    if (sent == null) return null
    val frame = Json.parseToJsonElement(sent).jsonObject
    return (frame["id"] as? JsonPrimitive)?.contentOrNull
}

fun described(result: StampClientServiceStampResult): JsonObject = when (result) {
    is StampClientServiceStampResult.Ok -> buildJsonObject {
        put("ok", true)
        put("etag", result.value.headerOut0)
        put("ageIsNull", result.value.headerOut1 == null)
    }
    is StampClientServiceStampResult.Declared -> buildJsonObject {
        put("ok", false)
        put("declared", true)
        put("reasonIsNull", result.error.errorHeaderOut0 == null)
    }
    is StampClientServiceStampResult.Fault -> buildJsonObject {
        put("ok", false)
        put("faultKind", result.fault.kind.name)
        put("faultField", result.fault.field)
    }
}

object Handlers : StampClientServiceHandlers {
    val marked = java.util.Collections.synchronizedList(mutableListOf<String>())
    override suspend fun mark(req: String, tenant: String) {
        marked.add("$req $tenant")
    }
    override suspend fun stamp(req: String, tenant: String, trace: UInt?): StampClientServiceStampResult {
        if (req == "refuse") {
            return StampClientServiceStampResult.Declared(
                StampClientServiceStampResultDeclared(StampErrorRefused, trace?.let { "refused at $it" }),
            )
        }
        return StampClientServiceStampResult.Ok(
            StampClientServiceStampResultValue(StampReceipt(req, tenant), "etag-$tenant", trace?.let { it + 1u }),
        )
    }
}

suspend fun until(timeoutMs: Long = 2000, predicate: () -> Boolean): Boolean {
    val deadline = System.currentTimeMillis() + timeoutMs
    while (!predicate() && System.currentTimeMillis() < deadline) delay(5)
    return predicate()
}
"#;

/// `REPLIES` (id -> reply text, one per `stamp` call) and `FRAMES` (every frame the Rust client
/// wrote) are declared above this by the Rust side, both computed off the Rust twins alone.
const DRIVER: &str = r#"
fun main() = runBlocking {
    val socketA = ManualSocket()
    val transportA = StampClientServiceWsTransport(socketA, StampClientServiceWsOptions(heartbeat = null))
    val clientA = StampClientServiceWsClient(transportA)
    val pendingA = listOf(
        async { clientA.stamp("a", "acme", 7u) },
        async { clientA.stamp("b", "acme", null) },
        async { clientA.stamp("refuse", "acme", 3u) },
        async { clientA.stamp("refuse", "acme", null) },
    )
    delay(50)
    clientA.mark("m", "acme")
    val sentByKotlin = socketA.sent.toList()
    pendingA.forEach { it.cancel() }

    lateinit var socketB: ManualSocket
    socketB = ManualSocket(onSend = { text ->
        val id = requestId(text)
        val reply = id?.let { REPLIES[it] }
        if (reply != null) socketB.deliver(reply)
    })
    val transportB = StampClientServiceWsTransport(socketB, StampClientServiceWsOptions(heartbeat = null))
    val clientB = StampClientServiceWsClient(transportB)
    val readResults = listOf(
        described(clientB.stamp("a", "acme", 7u)),
        described(clientB.stamp("b", "acme", null)),
        described(clientB.stamp("refuse", "acme", 3u)),
        described(clientB.stamp("refuse", "acme", null)),
    )

    val socketD = ManualSocket()
    val transportD = StampClientServiceWsTransport(socketD, StampClientServiceWsOptions(heartbeat = null))
    val clientD = StampClientServiceWsClient(transportD)
    val missingEtag = async { clientD.stamp("x", "acme", null) }
    delay(50)
    val missingId = requestId(socketD.sent.firstOrNull())
    socketD.deliver(
        """{"kind":"reply","id":"$missingId","service":"StampClientService","ok":true,"value":{"label":"x","tenant":"acme"}}""",
    )
    val missingEtagResult = described(missingEtag.await())

    val socketC = ManualSocket()
    val transportC = StampClientServiceWsTransport(socketC, StampClientServiceWsOptions(heartbeat = null))
    val faults = java.util.Collections.synchronizedList(mutableListOf<StampClientServiceFaultFields>())
    attachStampClientServiceWsDispatcher(transportC.frames, Handlers, onFault = { faults.add(it) })
    delay(50)
    val requestFrameCount = FRAMES.count { Json.parseToJsonElement(it).jsonObject["id"] != null }
    for (frame in FRAMES) socketC.deliver(frame)
    until { socketC.sent.size >= requestFrameCount && Handlers.marked.isNotEmpty() }
    val written = socketC.sent.toList()

    val report = buildJsonObject {
        put("sentByKotlin", JsonArray(sentByKotlin.map { JsonPrimitive(it) }))
        put("readResults", JsonArray(readResults))
        put("missingEtagResult", missingEtagResult)
        put("written", JsonArray(written.map { JsonPrimitive(it) }))
        put("marked", JsonArray(Handlers.marked.map { JsonPrimitive(it) }))
        put("faultsCount", faults.size)
    }
    println(report.toString())
}
"#;

fn poll_by_hand<Answered>(pinned: Pin<&mut Answered>) -> Poll<Answered::Output>
where
    Answered: Future,
{
    pinned.poll(&mut PollContext::from_waker(Waker::noop()))
}

/// A function that puts a text frame on the wire, recording it in `sent` instead.
fn recording(
    sent: &Arc<Mutex<Vec<String>>>,
) -> impl Fn(String) -> Ready<Result<(), String>> + Send + Sync + 'static {
    let mailbox = Arc::clone(sent);
    move |text| {
        mailbox.lock().unwrap().push(text);
        ready(Ok(()))
    }
}

/// The Rust dispatcher's answer to one frame. `StampBackEnd` never suspends, so one poll answers.
fn rust_answer(backend: &StampBackEnd, frame: &str) -> Option<String> {
    match poll_by_hand(pin!(stamp_ws_rpc_transport::answer(frame, backend, &())).as_mut()) {
        Poll::Ready(answered) => answered,
        Poll::Pending => None,
    }
}

fn parsed(text: &str) -> Value {
    serde_json::from_str(text).unwrap()
}

fn stamp_definitions() -> Vec<String> {
    vec![
        stamp_receipt_kotlin::kotlin_definition(),
        stamp_error_kotlin::kotlin_definition(),
        stamp_client_service_fault_fields_kotlin::kotlin_definition(),
        stamp_client_service_fault_kind_kotlin::kotlin_definition(),
        StampClientServiceSchema::kotlin_http_client(),
        StampClientServiceSchema::kotlin_ws_client(),
    ]
}

fn module(preamble: &str) -> String {
    let mut parts = vec![KOTLIN_IMPORTS.to_owned()];
    parts.extend(stamp_definitions());
    parts.push(MANUAL_SOCKET.to_owned());
    parts.push(preamble.to_owned());
    parts.push(DRIVER.to_owned());
    parts.join("\n\n")
}

/// The four `stamp` calls and `mark`, sent over a recording Rust client and answered through the
/// Rust dispatcher — the reference every assertion below is measured against.
fn rust_reference() -> (Vec<String>, serde_json::Map<String, Value>) {
    let sent = Arc::new(Mutex::new(Vec::new()));
    let session = FrameSession::new(recording(&sent));
    let client = super::stamp_ws_rpc_client::StampClientServiceClient::new(session);
    let mut calls = [
        Box::pin(client.stamp("a".to_owned(), "acme".to_owned(), Some(7))),
        Box::pin(client.stamp("b".to_owned(), "acme".to_owned(), None)),
        Box::pin(client.stamp("refuse".to_owned(), "acme".to_owned(), Some(3))),
        Box::pin(client.stamp("refuse".to_owned(), "acme".to_owned(), None)),
    ];
    for call in &mut calls {
        assert!(poll_by_hand(call.as_mut()).is_pending());
    }
    assert!(poll_by_hand(pin!(client.mark("m".to_owned(), "acme".to_owned())).as_mut()).is_ready());
    let frames = sent.lock().unwrap().clone();
    let backend = StampBackEnd::default();
    let mut replies = serde_json::Map::new();
    for frame in &frames {
        if let Some(reply) = rust_answer(&backend, frame) {
            let id = parsed(frame)["id"].as_str().unwrap().to_owned();
            replies.insert(id, Value::String(reply));
        }
    }
    (frames, replies)
}

/// The `REPLIES`/`FRAMES` preamble the driver reads by name.
fn kotlin_preamble(rust_frames: &[String], replies: &serde_json::Map<String, Value>) -> String {
    let mut replies_literal = String::new();
    for (id, reply) in replies {
        let text = reply.as_str().unwrap_or_default();
        let _ = writeln!(replies_literal, "  \"{id}\" to \"\"\"{text}\"\"\",");
    }
    let mut frames_literal = String::new();
    for frame in rust_frames {
        let _ = writeln!(frames_literal, "  \"\"\"{frame}\"\"\",");
    }
    format!("val REPLIES = mapOf(\n{replies_literal})\n\nval FRAMES = listOf(\n{frames_literal})")
}

/// Runs the combined Kotlin program once; `None` where no Kotlin toolchain was reachable.
fn driven() -> Option<(Vec<String>, Value)> {
    let (rust_frames, replies) = rust_reference();
    let preamble = kotlin_preamble(&rust_frames, &replies);
    let written = ran_kotlin(&module(&preamble))?;
    let report: Value = serde_json::from_str(written.trim()).unwrap_or_default();
    Some((rust_frames, report))
}

/// The Kotlin client writes exactly the frames the Rust client writes: each header value its own
/// JSON value, an absent optional one crossing as `null`.
fn assert_writes(rust_frames: &[String], report: &Value) {
    let kotlin_frames: Vec<Value> = report["sentByKotlin"]
        .as_array()
        .unwrap()
        .iter()
        .map(|frame| parsed(frame.as_str().unwrap()))
        .collect();
    let rust_frames_parsed: Vec<Value> = rust_frames.iter().map(|frame| parsed(frame)).collect();
    assert_eq!(
        kotlin_frames, rust_frames_parsed,
        "the Kotlin client writes exactly the frames the Rust client writes"
    );
    assert_eq!(
        kotlin_frames[0]["headers"],
        serde_json::json!({ "x-tenant": "acme", "x-trace": 7_u32 }),
        "each header value crosses as the JSON value it is"
    );
    assert_eq!(
        kotlin_frames[1]["headers"],
        serde_json::json!({ "x-tenant": "acme", "x-trace": null }),
        "an optional header holding nothing crosses as `null`, as the Rust client writes a `None`"
    );
    assert_eq!(
        kotlin_frames[4]["headers"],
        serde_json::json!({ "x-tenant": "acme" }),
        "the one-way call's own header crosses too"
    );
}

/// The Kotlin client reads the replies the Rust dispatcher writes, including a reply missing a
/// required header.
fn assert_reads(report: &Value) {
    let read = &report["readResults"];
    assert_eq!(
        read[0],
        serde_json::json!({ "ok": true, "etag": "etag-acme", "ageIsNull": false }),
        "got: {read}"
    );
    assert_eq!(
        read[1],
        serde_json::json!({ "ok": true, "etag": "etag-acme", "ageIsNull": true }),
        "an absent optional `header_out` element decodes as null. got: {read}"
    );
    assert_eq!(
        read[2],
        serde_json::json!({ "ok": false, "declared": true, "reasonIsNull": false }),
        "got: {read}"
    );
    assert_eq!(
        read[3],
        serde_json::json!({ "ok": false, "declared": true, "reasonIsNull": true }),
        "an absent optional `error_header_out` element decodes as null. got: {read}"
    );
    assert_eq!(
        report["missingEtagResult"]["faultKind"], "FailedValidation",
        "got: {report}"
    );
    assert_eq!(
        report["missingEtagResult"]["faultField"], "etag",
        "a missing required `header_out` element answers a failed-validation fault naming it. \
         got: {report}"
    );
}

/// The mini server answers every request frame the Rust client wrote the way the Rust dispatcher
/// answers the same frame, and reads the one-way call's own header off it too.
fn assert_mini_server(rust_frames: &[String], report: &Value) {
    let backend = StampBackEnd::default();
    let written_replies: Vec<Value> = report["written"]
        .as_array()
        .unwrap()
        .iter()
        .map(|reply| parsed(reply.as_str().unwrap()))
        .collect();
    for frame in rust_frames {
        let Some(id) = parsed(frame)["id"].as_str().map(str::to_owned) else {
            continue;
        };
        let found = written_replies
            .iter()
            .find(|reply| reply["id"] == Value::String(id.clone()));
        assert!(
            found.is_some(),
            "no reply for id {id}. got: {written_replies:#?}"
        );
        let kotlin_reply = found.unwrap();
        let rust_reply = parsed(&rust_answer(&backend, frame).unwrap());
        assert_eq!(
            kotlin_reply, &rust_reply,
            "the Kotlin mini server answers frame {frame} the way the Rust dispatcher does"
        );
    }
    assert_eq!(
        report["marked"],
        serde_json::json!(["m acme"]),
        "the mini server read the one-way call's own header off the Rust client's frame. \
         got: {report}"
    );
}

#[test]
fn the_kotlin_client_and_mini_server_agree_with_the_rust_ws_rpc_twins() {
    let Some((rust_frames, report)) = driven() else {
        return;
    };
    assert_writes(&rust_frames, &report);
    assert_reads(&report);
    assert_mini_server(&rust_frames, &report);
}
