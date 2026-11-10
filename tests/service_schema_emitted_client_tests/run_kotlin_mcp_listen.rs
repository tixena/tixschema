//! The emitted Kotlin `mcp` server run under a Kotlin toolchain over `subscriptions/listen`: it
//! writes and answers the bytes the Rust dispatcher writes and answers to the same requests,
//! handed the same changes, on stdio, on an endpoint of two services and over plain HTTP terms,
//! and every message is held to the official 2026-07-28 schema. Each stream ends on the events it
//! observes. One `kotlinc` invocation.

#![cfg(feature = "kotlin")]

use super::LibraryServiceSchema;
use super::mcp_listen_rows::{assert_matches, input};
use super::run_kotlin_mcp_resources::{
    HANDLERS, KOTLIN_IMPORTS, definitions, kotlin_text, wrapper_of_bytes,
};
use super::runtime::ran_kotlin;
use crate::mcp_streams::kotlin_respond;
use serde_json::Value;
use std::sync::OnceLock;

const DRIVER: &str = r#"
class RecordingStream(private val goneAfter: Int? = null) : LibraryServiceMcpHttpStream {
    val seen = mutableListOf<JsonObject>()
    private val changed = kotlinx.coroutines.channels.Channel<Unit>(kotlinx.coroutines.channels.Channel.UNLIMITED)

    suspend fun until(done: () -> Boolean) {
        while (!done()) changed.receive()
    }

    private fun take() {
        if (goneAfter != null && seen.size >= goneAfter) throw IllegalStateException("the client has gone")
    }

    override suspend fun begin(status: Int, headers: List<Pair<String, String>>) {
        take()
        seen.add(
            buildJsonObject {
                put("began", JsonArray(listOf(JsonPrimitive(status), JsonArray(headers.map { JsonArray(listOf(JsonPrimitive(it.first), JsonPrimitive(it.second))) }))))
            },
        )
        changed.trySend(Unit)
    }

    override suspend fun write(event: String) {
        take()
        seen.add(buildJsonObject { put("wrote", event) })
        changed.trySend(Unit)
    }
}

class Feed {
    private val channel = kotlinx.coroutines.channels.Channel<LibraryServiceMcpChange>(kotlinx.coroutines.channels.Channel.UNLIMITED)

    suspend fun next(): LibraryServiceMcpChange? = channel.receiveCatching().getOrNull()

    fun push(change: LibraryServiceMcpChange) {
        channel.trySend(change)
    }

    fun close() {
        channel.close()
    }
}

fun changeOf(spec: JsonObject): LibraryServiceMcpChange = when (spec.getValue("kind").jsonPrimitive.content) {
    "resourcesListChanged" -> LibraryServiceMcpChange.ResourcesListChanged
    "resourceUpdated" -> LibraryServiceMcpChange.ResourceUpdated(spec.getValue("uri").jsonPrimitive.content)
    "task" -> LibraryServiceMcpChange.Task(spec.getValue("taskId").jsonPrimitive.content, spec.getValue("state").jsonObject)
    else -> LibraryServiceMcpChange.KeepAlive
}

fun changesOf(script: JsonArray): LibraryServiceMcpChanges {
    val queue = ArrayDeque(script.map { changeOf(it.jsonObject) })
    return { queue.removeFirstOrNull() }
}

class Lines {
    val written = mutableListOf<String>()
    val write: LibraryServiceMcpWriter = { message -> written.add(message) }
    fun json(): JsonElement = JsonArray(written.map { JsonPrimitive(it) })
}

fun httpReport(answered: LibraryServiceMcpHttpResponse): JsonObject = buildJsonObject {
    put("status", answered.status)
    put("headers", JsonArray(answered.headers.map { JsonArray(listOf(JsonPrimitive(it.first), JsonPrimitive(it.second))) }))
    put("body", answered.body.decodeToString())
}

fun httpRequest(row: JsonObject): LibraryServiceMcpHttpRequest =
    LibraryServiceMcpHttpRequest(
        row.getValue("method").jsonPrimitive.content,
        row.getValue("path").jsonPrimitive.content,
        row.getValue("headers").jsonArray.map { it.jsonArray[0].jsonPrimitive.content to it.jsonArray[1].jsonPrimitive.content },
        row.getValue("body").jsonPrimitive.content.encodeToByteArray(),
    )

suspend fun report(input: JsonObject): JsonObject = coroutineScope {
    val listens = input.getValue("listens").jsonArray
    val libraryRoute = createLibraryServiceMcpRoute(LibraryHandlers)
    val depotRoute = createDepotServiceMcpRoute(DepotHandlers)
    val workspace = LibraryServiceMcpEndpoint(libraryServiceMcpCatalog, depotServiceMcpCatalog)
    val discover = input.getValue("discover").jsonPrimitive.content
    val denied: suspend (LibraryServiceMcpHttpRequest, LibraryServiceMcpCall) -> LibraryServiceMcpAdmitted =
        { _, _ -> LibraryServiceMcpAdmitted.Caller(LibraryHandlers) }
    val settings = LibraryServiceMcpHttpSettings(emptyList())

    val stdio = listens.map { held ->
        val row = held.jsonObject
        val out = Lines()
        val dispatch = createLibraryServiceMcpDispatcher(changesOf(row.getValue("script").jsonArray)) { LibraryHandlers }
        val reply = dispatch(row.getValue("request").jsonPrimitive.content, out.write, LibraryServiceMcpInFlight())
        buildJsonObject { put("reply", nullable(reply)); put("written", out.json()) }
    }

    val first = listens[0].jsonObject
    val out = Lines()
    val served = libraryServiceMcpServe(
        first.getValue("request").jsonPrimitive.content, "Workspace", null, workspace,
        LibraryServiceMcpHost(out.write, LibraryServiceMcpInFlight(), changesOf(first.getValue("script").jsonArray)),
    ) { _, part, method, params ->
        when (part) {
            0 -> libraryRoute(method, params)
            1 -> depotRoute(method, params)
            else -> null
        }
    }
    val workspaceReport = buildJsonObject { put("reply", nullable(served)); put("written", out.json()) }

    val http = input.getValue("http").jsonArray.map { held ->
        val row = held.jsonObject
        val over = createLibraryServiceMcpHttpDispatcher(settings, changesOf(row.getValue("script").jsonArray), denied)
        httpReport(respondLibraryService(over, httpRequest(row)))
    }

    val firstHttp = input.getValue("http").jsonArray[0].jsonObject
    val plainOver = createLibraryServiceMcpHttpDispatcher(settings, null, denied)
    val plainResponse = respondLibraryService(plainOver, httpRequest(firstHttp))
    val plainDispatch = createLibraryServiceMcpDispatcher { LibraryHandlers }
    val plainReply = plainDispatch(first.getValue("request").jsonPrimitive.content, Lines().write, LibraryServiceMcpInFlight())

    val listeningDispatch = createLibraryServiceMcpDispatcher(changesOf(JsonArray(emptyList()))) { LibraryHandlers }
    val listening = listeningDispatch(discover, Lines().write, LibraryServiceMcpInFlight())
    val plainDiscover = plainDispatch(discover, Lines().write, LibraryServiceMcpInFlight())

    val cancelLines = input.getValue("cancel").jsonArray
    val cancelOut = Lines()
    val firstWritten = CompletableDeferred<Unit>()
    val inFlight = LibraryServiceMcpInFlight()
    val pendingDispatch = createLibraryServiceMcpDispatcher({ awaitCancellation() }) { LibraryHandlers }
    val pending = async {
        pendingDispatch(cancelLines[0].jsonPrimitive.content, { message -> cancelOut.written.add(message); firstWritten.complete(Unit) }, inFlight)
    }
    firstWritten.await()
    val start = cancelOut.written.size
    val stopped = plainDispatch(cancelLines[1].jsonPrimitive.content, cancelOut.write, inFlight)
    check(stopped == null) { "a notification was answered" }
    val cancelledReply = pending.await()

    val gone = input.getValue("gone").jsonObject
    val goneDispatch = createLibraryServiceMcpDispatcher(changesOf(gone.getValue("script").jsonArray)) { LibraryHandlers }
    val goneReply = goneDispatch(
        gone.getValue("request").jsonPrimitive.content,
        { throw IllegalStateException("the client has gone") },
        LibraryServiceMcpInFlight(),
    )

    val streamedRow = input.getValue("streamed").jsonObject
    val orderedStream = RecordingStream()
    val feed = Feed()
    val ordered = createLibraryServiceMcpHttpDispatcher(settings, { feed.next() }, denied)
    val orderedRunning = async { ordered(httpRequest(streamedRow), orderedStream) }
    val snapshots = mutableListOf<JsonElement>()
    orderedStream.until { orderedStream.seen.size >= 2 }
    snapshots.add(JsonArray(orderedStream.seen.toList()))
    var expected = 2
    for (change in listOf<LibraryServiceMcpChange>(
        LibraryServiceMcpChange.ResourcesListChanged,
        LibraryServiceMcpChange.ResourceUpdated("library://covers/book-1"),
        LibraryServiceMcpChange.KeepAlive,
    )) {
        feed.push(change)
        expected += 1
        orderedStream.until { orderedStream.seen.size >= expected }
        snapshots.add(JsonArray(orderedStream.seen.toList()))
    }
    val endedWhileOpen = orderedRunning.isCompleted
    feed.close()
    val orderedAnswered = orderedRunning.await()

    val goneStream = RecordingStream(1)
    var goneTaken: LibraryServiceMcpCall? = null
    val goneOver = createLibraryServiceMcpHttpDispatcher(settings, { awaitCancellation() }) { _, call ->
        goneTaken = call
        LibraryServiceMcpAdmitted.Caller(LibraryHandlers)
    }
    val goneAnswered = goneOver(httpRequest(streamedRow), goneStream)
    val streams = buildJsonObject {
        put("ordered", buildJsonObject {
            put("endedWhileOpen", endedWhileOpen)
            put("full", JsonArray(orderedStream.seen.toList()))
            put("returned", orderedAnswered != null)
            put("snapshots", JsonArray(snapshots))
        })
        put("gone", buildJsonObject {
            put("cancelled", goneTaken!!.cancelled)
            put("returned", goneAnswered != null)
            put("seen", JsonArray(goneStream.seen.toList()))
        })
    }

    buildJsonObject {
        put("streams", streams)
        put("stdio", JsonArray(stdio))
        put("workspace", workspaceReport)
        put("http", JsonArray(http))
        put("plain", buildJsonObject { put("http", httpReport(plainResponse)); put("stdio", nullable(plainReply)) })
        put("discover", buildJsonObject { put("listening", nullable(listening)); put("plain", nullable(plainDiscover)) })
        put("cancelled", buildJsonObject {
            put("reply", nullable(cancelledReply))
            put("running", 0)
            put("start", start)
            put("written", cancelOut.json())
        })
        put("gone", buildJsonObject { put("reply", nullable(goneReply)); put("written", JsonArray(emptyList())) })
    }
}

fun main() = runBlocking {
    println(report(Json.parseToJsonElement(INPUT).jsonObject).toString())
}
"#;

fn module() -> String {
    let literal = kotlin_text(&input().to_string());
    let mut parts = vec![KOTLIN_IMPORTS.to_owned()];
    parts.extend(definitions());
    parts.push(HANDLERS.replace(
        "%TUPLE%",
        &wrapper_of_bytes(&LibraryServiceSchema::kotlin_http_client()),
    ));
    parts.push(kotlin_respond("LibraryService"));
    parts.push(format!("const val INPUT = \"{literal}\""));
    parts.push(DRIVER.to_owned());
    parts.join("\n\n")
}

/// The report of the one Kotlin run; `None` where no Kotlin toolchain was reachable.
fn driven() -> Option<&'static Value> {
    static REPORT: OnceLock<Option<Value>> = OnceLock::new();
    REPORT
        .get_or_init(|| {
            let written = ran_kotlin(&module())?;
            Some(serde_json::from_str(written.trim()).unwrap())
        })
        .as_ref()
}

#[test]
fn the_kotlin_server_writes_and_answers_what_the_rust_dispatcher_does_on_a_listen() {
    let Some(report) = driven() else {
        return;
    };
    assert_matches(report);
}
