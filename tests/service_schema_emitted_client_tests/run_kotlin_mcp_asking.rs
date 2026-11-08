//! The emitted Kotlin `mcp` server and client run under a Kotlin toolchain on `AskingService`: the
//! server's answer to each request of the form, link and content flows is the one the Rust
//! dispatcher gives, and the client reads the answers back into the pair, the list and the
//! declared arm. One `kotlinc` invocation computes both reports.

#![cfg(feature = "kotlin")]

use super::mcp_asking_rows::{
    asking_confirmation_kotlin, asking_error_kotlin, asking_link_error_kotlin,
    asking_linked_kotlin, asking_miss_kotlin, asking_priority_kotlin, asking_report_request_kotlin,
    asking_shipping_kotlin, assert_client, assert_matches_except, client_input,
    delete_document_request_kotlin, input, ship_document_request_kotlin,
};
use super::mcp_mobile_rows::kotlin_text;
use super::runtime::ran_kotlin;
use super::{AskingServiceSchema, PreviewServiceSchema};
use super::{asking_service_schema, preview_service_schema};
use serde_json::{Value, json};
use std::sync::OnceLock;

const KOTLIN_IMPORTS: &str = "import kotlinx.coroutines.*\n\
     import kotlinx.coroutines.flow.*\n\
     import kotlinx.serialization.*\n\
     import kotlinx.serialization.json.*\n\
     import kotlinx.serialization.descriptors.*\n\
     import kotlinx.serialization.encoding.*\n\
     import kotlinx.serialization.builtins.*";

/// The handlers the server serves: they write down what reaches them.
const HANDLERS: &str = r#"
val reached = mutableListOf<JsonElement>()

fun note(vararg parts: JsonElement) {
    reached.add(JsonArray(parts.toList()))
}

object AskingHandlers : AskingServiceHandlers {
    override suspend fun buildReport(req: AskingReportRequest): AskingServiceBuildReportResult {
        note(JsonPrimitive("build-report"), JsonPrimitive(req.title))
        return AskingServiceBuildReportResult.Ok(
            listOf(
                McpContent.Text(req.title),
                McpContent.Image(listOf<UByte>(0x89u, 0x50u, 0x4eu, 0x47u), "image/png"),
                McpContent.Audio(listOf<UByte>(1u, 2u, 3u, 4u, 5u), "audio/mpeg"),
                McpContent.ResourceLink("workspace://documents/doc-1", "id-card.pdf", "Id card", null, "application/pdf"),
                McpContent.Resource("workspace://documents/doc-2", "text/plain", McpResourceContents.Text("passport")),
                McpContent.Resource("workspace://documents/doc-3", null, McpResourceContents.Blob(listOf<UByte>(9u, 8u, 7u))),
            ),
        )
    }

    override suspend fun deleteDocument(req: DeleteDocumentRequest): AskingServiceDeleteDocumentResult {
        val confirmation = req.confirmation
        note(
            JsonPrimitive("delete-document"),
            JsonPrimitive(req.documentId),
            if (confirmation == null) JsonNull else Json.encodeToJsonElement(serializer<AskingConfirmation>(), confirmation),
        )
        if (req.documentId == "missing") return AskingServiceDeleteDocumentResult.Declared(AskingErrorNotFound)
        if (confirmation == null) {
            return AskingServiceDeleteDocumentResult.Declared(AskingErrorNeedsConfirmation("Delete this document for good?"))
        }
        return if (confirmation.confirm) AskingServiceDeleteDocumentResult.Ok
        else AskingServiceDeleteDocumentResult.Declared(AskingErrorRefused)
    }

    override suspend fun importFromDrive(req: String): AskingServiceImportFromDriveResult {
        note(JsonPrimitive("import-from-drive"), JsonPrimitive(req))
        if (req == "connected") return AskingServiceImportFromDriveResult.Ok(AskingLinked("doc-" + req))
        return AskingServiceImportFromDriveResult.Declared(
            AskingLinkErrorNeedsConnection(
                "Connect your Google Drive so this server can import the file.",
                "https://mcp.example.com/connect/drive?request=7f3a",
            ),
        )
    }

    override suspend fun shipDocument(req: ShipDocumentRequest): AskingServiceShipDocumentResult {
        val shipping = req.shipping
        note(
            JsonPrimitive("ship-document"),
            JsonPrimitive(req.documentId),
            if (shipping == null) JsonNull else Json.encodeToJsonElement(serializer<AskingShipping>(), shipping),
        )
        if (shipping == null) {
            return AskingServiceShipDocumentResult.Declared(AskingErrorNeedsConfirmation("How should it ship?"))
        }
        return AskingServiceShipDocumentResult.Ok
    }
}

suspend fun serverReport(input: JsonObject): JsonObject {
    val dispatch = createAskingServiceMcpDispatcher { _ -> AskingHandlers }
    val inFlight = AskingServiceMcpInFlight()
    val nowhere: AskingServiceMcpWriter = { }
    val answers = mutableMapOf<String, JsonElement>()
    val calls = mutableMapOf<String, JsonElement>()
    val skipped = input.getValue("previews").jsonArray.map { it.jsonPrimitive.content }
    for ((name, text) in input.getValue("requests").jsonObject) {
        if (name in skipped) continue
        reached.clear()
        val reply = dispatch(text.jsonPrimitive.content, nowhere, inFlight)
        answers[name] = if (reply == null) JsonNull else JsonPrimitive(reply)
        calls[name] = JsonArray(reached.toList())
    }
    return buildJsonObject {
        put("answers", JsonObject(answers))
        put("reached", JsonObject(calls))
    }
}
"#;

const CLIENT_DRIVER: &str = r#"
class Canned : AskingServiceMcpTransport, PreviewServiceMcpTransport {
    val sent = mutableListOf<JsonElement>()
    val queue = ArrayDeque<String>()

    override suspend fun send(message: AskingServiceMcpMessage): String {
        sent.add(Json.parseToJsonElement(message.text))
        return queue.removeFirst()
    }

    override suspend fun send(message: PreviewServiceMcpMessage): String {
        sent.add(Json.parseToJsonElement(message.text))
        return queue.removeFirst()
    }
}

fun bytes(held: List<UByte>): JsonElement = JsonArray(held.map { JsonPrimitive(it.toInt()) })

fun deleted(result: AskingServiceDeleteDocumentResult): JsonElement = when (result) {
    is AskingServiceDeleteDocumentResult.Ok -> buildJsonObject { put("ok", JsonNull) }
    is AskingServiceDeleteDocumentResult.Declared ->
        buildJsonObject { put("declared", Json.encodeToJsonElement(serializer<AskingError>(), result.error)) }
    is AskingServiceDeleteDocumentResult.Fault -> buildJsonObject { put("fault", result.fault.kind.toString()) }
}

fun previewed(result: PreviewServiceDocumentPreviewResult): JsonElement = when (result) {
    is PreviewServiceDocumentPreviewResult.Ok -> buildJsonObject {
        put("ok", buildJsonObject { put("bytes", bytes(result.value.slot0)); put("contentType", result.value.slot1) })
    }
    is PreviewServiceDocumentPreviewResult.Declared ->
        buildJsonObject { put("declared", Json.encodeToJsonElement(serializer<AskingMiss>(), result.error)) }
    is PreviewServiceDocumentPreviewResult.Fault -> buildJsonObject { put("fault", result.fault.kind.toString()) }
}

fun reported(result: AskingServiceBuildReportResult): JsonElement = when (result) {
    is AskingServiceBuildReportResult.Ok -> buildJsonObject {
        put("ok", JsonArray(result.value.map { Json.encodeToJsonElement(McpContentSerializer, it) }))
    }
    is AskingServiceBuildReportResult.Declared ->
        buildJsonObject { put("declared", Json.encodeToJsonElement(serializer<AskingMiss>(), result.error)) }
    is AskingServiceBuildReportResult.Fault -> buildJsonObject { put("fault", result.fault.kind.toString()) }
}

fun imported(result: AskingServiceImportFromDriveResult): JsonElement = when (result) {
    is AskingServiceImportFromDriveResult.Ok ->
        buildJsonObject { put("ok", Json.encodeToJsonElement(serializer<AskingLinked>(), result.value)) }
    is AskingServiceImportFromDriveResult.Declared ->
        buildJsonObject { put("declared", Json.encodeToJsonElement(serializer<AskingLinkError>(), result.error)) }
    is AskingServiceImportFromDriveResult.Fault -> buildJsonObject { put("fault", result.fault.kind.toString()) }
}

suspend fun clientReport(input: JsonObject): JsonObject {
    val canned = Canned()
    val client = AskingServiceMcpClient(canned)
    val previewer = PreviewServiceMcpClient(canned)
    val calls: Map<String, suspend () -> JsonElement> = mapOf(
        "declared" to { deleted(client.deleteDocument(DeleteDocumentRequest("doc-1", null))) },
        "confirmed" to { deleted(client.deleteDocument(DeleteDocumentRequest("doc-1", AskingConfirmation(true)))) },
        "image" to { previewed(previewer.documentPreview("doc-1")) },
        "audio" to { previewed(previewer.documentPreview("voice")) },
        "plain" to { previewed(previewer.documentPreview("plain")) },
        "missing" to { previewed(previewer.documentPreview("nothing")) },
        "report" to { reported(client.buildReport(AskingReportRequest("Q3"))) },
        "link" to { imported(client.importFromDrive("1A2b3C")) },
        "linked" to { imported(client.importFromDrive("connected")) },
    )
    val outcomes = mutableMapOf<String, JsonElement>()
    for (call in input.getValue("calls").jsonArray) {
        val label = call.jsonObject.getValue("label").jsonPrimitive.content
        canned.queue.addLast(call.jsonObject.getValue("answer").jsonPrimitive.content)
        outcomes[label] = calls.getValue(label)()
    }
    return buildJsonObject {
        put("outcomes", JsonObject(outcomes))
        put("seen", JsonArray(canned.sent))
    }
}

fun main() = runBlocking {
    val input = Json.parseToJsonElement(INPUT).jsonObject
    println(buildJsonObject {
        put("server", serverReport(input.getValue("server").jsonObject))
        put("client", clientReport(input.getValue("client").jsonObject))
    }.toString())
}
"#;

fn definitions() -> Vec<String> {
    vec![
        asking_priority_kotlin::kotlin_definition(),
        asking_shipping_kotlin::kotlin_definition(),
        asking_confirmation_kotlin::kotlin_definition(),
        asking_error_kotlin::kotlin_definition(),
        asking_link_error_kotlin::kotlin_definition(),
        asking_miss_kotlin::kotlin_definition(),
        asking_report_request_kotlin::kotlin_definition(),
        asking_linked_kotlin::kotlin_definition(),
        delete_document_request_kotlin::kotlin_definition(),
        ship_document_request_kotlin::kotlin_definition(),
        asking_service_schema::asking_service_fault_fields_kotlin::kotlin_definition(),
        asking_service_schema::asking_service_fault_kind_kotlin::kotlin_definition(),
        AskingServiceSchema::kotlin_http_client(),
        AskingServiceSchema::kotlin_ws_client(),
        AskingServiceSchema::kotlin_mcp_content(),
        AskingServiceSchema::kotlin_mcp_service(),
        AskingServiceSchema::kotlin_mcp_client(),
        preview_service_schema::preview_service_fault_fields_kotlin::kotlin_definition(),
        preview_service_schema::preview_service_fault_kind_kotlin::kotlin_definition(),
        PreviewServiceSchema::kotlin_http_client(),
        PreviewServiceSchema::kotlin_mcp_client(),
    ]
}

fn module() -> String {
    let literal = kotlin_text(&json!({ "server": input(), "client": client_input() }).to_string());
    let mut parts = vec![KOTLIN_IMPORTS.to_owned()];
    parts.extend(definitions());
    parts.push(HANDLERS.to_owned());
    parts.push(format!("const val INPUT = \"{literal}\""));
    parts.push(CLIENT_DRIVER.to_owned());
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
fn the_kotlin_server_answers_the_bytes_the_rust_dispatcher_answers() {
    let Some(report) = driven() else {
        return;
    };
    let previews = input()["previews"].clone();
    let skipped: Vec<&str> = previews
        .as_array()
        .unwrap()
        .iter()
        .map(|name| name.as_str().unwrap())
        .collect();
    assert_matches_except(&report["server"], &skipped);
}

#[test]
fn the_kotlin_client_reads_the_blocks_and_the_arm_the_rust_dispatcher_answers() {
    let Some(report) = driven() else {
        return;
    };
    assert_client(&report["client"]);
}
