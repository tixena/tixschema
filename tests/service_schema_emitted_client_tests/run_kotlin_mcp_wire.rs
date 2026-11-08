//! The emitted Kotlin `http_rest` and `ws_rpc` clients, and the `ws_rpc` attachment, of
//! `ShelfWireService`, run under a Kotlin toolchain: an operation answering content blocks or
//! prompt messages is read and written in the block form the Rust dispatchers write.

#![cfg(feature = "kotlin")]

use super::ShelfWireServiceSchema;
use super::mcp_wire_rust::{BLOCKS, MESSAGES, PAGE, over_http, over_ws};
use super::mcp_wire_service::{shelf_wire_error_kotlin, shelf_wire_request_kotlin};
use super::runtime::ran_kotlin;
use super::shelf_wire_service_schema;
use serde_json::{Value, json};

const KOTLIN_IMPORTS: &str = "import kotlinx.coroutines.*\n\
     import kotlinx.coroutines.flow.*\n\
     import kotlinx.serialization.*\n\
     import kotlinx.serialization.json.*\n\
     import kotlinx.serialization.descriptors.*\n\
     import kotlinx.serialization.encoding.*\n\
     import kotlinx.serialization.builtins.*";

const DRIVER: &str = r##"
object CannedHttp : ShelfWireServiceHttpTransport {
    override suspend fun send(request: ShelfWireServiceHttpRequest): ShelfWireServiceHttpResponse =
        ShelfWireServiceHttpResponse(200, emptyList(), RUST_HTTP.getValue(request.path).encodeToByteArray())
}

class ManualSocket(private val onSend: (String) -> Unit) : ShelfWireServiceWsSocket {
    override var onMessage: ((String) -> Unit)? = null
    override var onClose: (() -> Unit)? = null
    override fun send(text: String) = onSend(text)
    override fun close() { onClose?.invoke() }
    fun deliver(text: String) { onMessage?.invoke(text) }
}

fun blocksOf(result: ShelfWireServiceDescribeShelfResult): JsonElement = when (result) {
    is ShelfWireServiceDescribeShelfResult.Ok -> Json.encodeToJsonElement(serializer<List<McpContent>>(), result.value)
    else -> JsonPrimitive("other")
}

fun pageOf(result: ShelfWireServiceListShelvesResult): JsonElement = when (result) {
    is ShelfWireServiceListShelvesResult.Ok -> Json.encodeToJsonElement(serializer<ShelfWireServiceMcpResourcePage>(), result.value)
    else -> JsonPrimitive("other")
}

fun messagesOf(result: ShelfWireServiceReviewShelfResult): JsonElement = when (result) {
    is ShelfWireServiceReviewShelfResult.Ok -> Json.encodeToJsonElement(serializer<List<McpPromptMessage>>(), result.value)
    else -> JsonPrimitive("other")
}

object Handlers : ShelfWireServiceHandlers {
    override suspend fun describeShelf(req: ShelfWireRequest) = ShelfWireServiceDescribeShelfResult.Ok(listOf(
        McpContent.Text("Eleven books."),
        McpContent.Image(listOf<UByte>(0x89u, 0x50u, 0x4eu, 0x47u), "image/png"),
        McpContent.Audio(listOf<UByte>(0xffu, 0xfbu, 0x90u), "audio/mpeg"),
        McpContent.ResourceLink("shelf://history", "history", "History", null, "text/markdown"),
        McpContent.Resource("shelf://history/index", "text/markdown", McpResourceContents.Text("# History")),
        McpContent.Resource("shelf://history/cover", null, McpResourceContents.Blob(listOf<UByte>(9u, 8u, 7u))),
    ))

    override suspend fun listShelves(req: String?) = ShelfWireServiceListShelvesResult.Ok(
        ShelfWireServiceMcpResourcePage(
            listOf(ShelfWireServiceMcpResource("shelf://history", "history", null, null, "text/markdown", 11L)),
            "page-2",
        ),
    )

    override suspend fun reviewShelf(req: String) = ShelfWireServiceReviewShelfResult.Ok(listOf(
        McpPromptMessage(McpRole.User, McpContent.Text("Review the history shelf.")),
        McpPromptMessage(McpRole.Assistant, McpContent.Text("Which books?")),
    ))
}

fun main() = runBlocking {
    val overHttp = ShelfWireServiceHttpClient(CannedHttp)
    val httpDescribed = blocksOf(overHttp.describeShelf(ShelfWireRequest("history")))
    val httpReviewed = messagesOf(overHttp.reviewShelf("history"))
    val httpListed = pageOf(overHttp.listShelves(null))

    lateinit var socket: ManualSocket
    socket = ManualSocket { text ->
        val sent = Json.parseToJsonElement(text).jsonObject
        val operation = sent.getValue("operation").jsonPrimitive.content
        val reply = Json.parseToJsonElement(RUST_REPLIES.getValue(operation)).jsonObject
        socket.deliver(JsonObject(reply + ("id" to sent.getValue("id"))).toString())
    }
    val transport = ShelfWireServiceWsTransport(socket, ShelfWireServiceWsOptions(heartbeat = null))
    val overWs = ShelfWireServiceWsClient(transport)
    val wsDescribed = blocksOf(overWs.describeShelf(ShelfWireRequest("history")))
    val wsReviewed = messagesOf(overWs.reviewShelf("history"))
    val wsListed = pageOf(overWs.listShelves(null))

    val written = java.util.Collections.synchronizedList(mutableListOf<String>())
    val everyReplyArrived = CompletableDeferred<Unit>()
    val served = ManualSocket { text ->
        written.add(text)
        if (written.size == FRAMES.size) everyReplyArrived.complete(Unit)
    }
    val serving = ShelfWireServiceWsTransport(served, ShelfWireServiceWsOptions(heartbeat = null))
    attachShelfWireServiceWsDispatcher(serving.frames, Handlers, onFault = {})
    for (frame in FRAMES) served.deliver(frame)
    everyReplyArrived.await()

    println(buildJsonObject {
        put("http", JsonArray(listOf(httpDescribed, httpReviewed, httpListed)))
        put("ws", JsonArray(listOf(wsDescribed, wsReviewed, wsListed)))
        put("answers", JsonArray(written.map { Json.parseToJsonElement(it) }))
    }.toString())
}
"##;

fn module(declared: &str) -> String {
    [
        KOTLIN_IMPORTS.to_owned(),
        shelf_wire_error_kotlin::kotlin_definition(),
        shelf_wire_request_kotlin::kotlin_definition(),
        shelf_wire_service_schema::shelf_wire_service_fault_fields_kotlin::kotlin_definition(),
        shelf_wire_service_schema::shelf_wire_service_fault_kind_kotlin::kotlin_definition(),
        ShelfWireServiceSchema::kotlin_http_client(),
        ShelfWireServiceSchema::kotlin_ws_client(),
        ShelfWireServiceSchema::kotlin_mcp_content(),
        declared.to_owned(),
        DRIVER.to_owned(),
    ]
    .join("\n\n")
}

fn declared() -> (String, Vec<Value>) {
    let history = json!({ "shelf": "history" });
    let (describe_frame, describe_reply) = over_ws("describe-shelf", &history);
    let (review_frame, review_reply) = over_ws("review-shelf", &json!("history"));
    let (list_frame, list_reply) = over_ws("list-shelves", &Value::Null);
    let (_, list_body) = over_http("/list-shelves", &Value::Null);
    let (_, describe_body) = over_http("/describe-shelf", &history);
    let (_, review_body) = over_http("/review-shelf", &json!("history"));
    let text = |held: &str| format!("\"\"\"{held}\"\"\"");
    (
        format!(
            "val RUST_HTTP = mapOf(\"/describe-shelf\" to {}, \"/review-shelf\" to {}, \"/list-shelves\" to {})\n\
             val RUST_REPLIES = mapOf(\"describe-shelf\" to {}, \"review-shelf\" to {}, \"list-shelves\" to {})\n\
             val FRAMES = listOf({}, {}, {})",
            text(&describe_body),
            text(&review_body),
            text(&list_body),
            text(&describe_reply),
            text(&review_reply),
            text(&list_reply),
            text(&describe_frame),
            text(&review_frame),
            text(&list_frame),
        ),
        vec![
            serde_json::from_str(&describe_reply).unwrap(),
            serde_json::from_str(&review_reply).unwrap(),
            serde_json::from_str(&list_reply).unwrap(),
        ],
    )
}

#[test]
fn the_kotlin_clients_read_and_the_attachment_writes_the_block_form_the_rust_dispatchers_write() {
    let (text, replies) = declared();
    let Some(wrote) = ran_kotlin(&module(&text)) else {
        return;
    };
    let report: Value = serde_json::from_str(wrote.trim()).unwrap();
    let blocks: Value = serde_json::from_str(BLOCKS).unwrap();
    let messages: Value = serde_json::from_str(MESSAGES).unwrap();
    let page: Value = serde_json::from_str(PAGE).unwrap();
    for transport in ["http", "ws"] {
        assert_eq!(report[transport][0], blocks, "{transport}");
        assert_eq!(report[transport][1], messages, "{transport}");
        assert_eq!(report[transport][2], page, "{transport}");
    }
    let answers = report["answers"].as_array().unwrap();
    assert_eq!(answers.len(), replies.len());
    for wanted in &replies {
        assert!(
            answers.contains(wanted),
            "{wanted} is not among {answers:#?}"
        );
    }
}
