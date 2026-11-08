//! The emitted Kotlin `mcp` server and client run under a Kotlin toolchain on `GuideService`: the
//! server's answer to each prompt request, completion, HTTP request and scope query is the one the
//! Rust dispatcher gives, and the client reads the answers back into the text, the messages and
//! the declared error. One `kotlinc` invocation computes both reports.

#![cfg(feature = "kotlin")]

use super::GuideServiceSchema;
use super::guide_service_schema;
use super::mcp_guide_rows::{
    assert_client, assert_matches, client_input, configure_organization_request_kotlin,
    get_guide_version_request_kotlin, guide_error_kotlin, guide_version_kotlin, input,
    review_document_type_request_kotlin, suggest_document_types_request_kotlin,
};
use super::mcp_mobile_rows::kotlin_text;
use super::runtime::ran_kotlin;
use crate::mcp_streams::kotlin_respond;
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
const HANDLERS: &str = r##"
val reached = mutableListOf<JsonElement>()

fun note(vararg parts: JsonElement) {
    reached.add(JsonArray(parts.toList()))
}

object GuideHandlers : GuideServiceHandlers {
    override suspend fun configureOrganization(req: ConfigureOrganizationRequest): GuideServiceConfigureOrganizationResult {
        note(JsonPrimitive("configure-organization"))
        return GuideServiceConfigureOrganizationResult.Ok("Apply the entity types in this order.")
    }

    override suspend fun getGuideVersion(req: GetGuideVersionRequest): GuideServiceGetGuideVersionResult {
        note(JsonPrimitive("get-guide-version"))
        return GuideServiceGetGuideVersionResult.Ok(GuideVersion(3u))
    }

    override suspend fun greet(req: String): GuideServiceGreetResult {
        note(JsonPrimitive("greet"), JsonPrimitive(req))
        if (req == "boom") throw IllegalStateException("the handler came apart")
        if (req == "missing") return GuideServiceGreetResult.Declared(GuideErrorNotFound)
        return GuideServiceGreetResult.Ok("Say hello to " + req + ".")
    }

    override suspend fun reviewDocumentType(req: ReviewDocumentTypeRequest): GuideServiceReviewDocumentTypeResult {
        note(JsonPrimitive("review-document-type"), JsonPrimitive(req.documentType), req.focus?.let { JsonPrimitive(it) } ?: JsonNull)
        return GuideServiceReviewDocumentTypeResult.Ok(
            listOf(
                McpPromptMessage(McpRole.User, McpContent.Text("Review the document type " + req.documentType + " against this guide.")),
                McpPromptMessage(McpRole.User, McpContent.Resource("guide://naming", "text/markdown", McpResourceContents.Text("# Naming"))),
                McpPromptMessage(McpRole.Assistant, McpContent.Image(listOf<UByte>(0x89u, 0x50u, 0x4eu, 0x47u), "image/png")),
                McpPromptMessage(McpRole.Assistant, McpContent.ResourceLink("guide://naming", "naming", "Naming", null, "text/markdown")),
            ),
        )
    }

    override suspend fun suggestDocumentTypes(req: SuggestDocumentTypesRequest): GuideServiceSuggestDocumentTypesResult {
        val known = req.chosen.entries.sortedBy { it.key }.map { JsonArray(listOf(JsonPrimitive(it.key), JsonPrimitive(it.value))) }
        note(JsonPrimitive("suggest-document-types"), JsonPrimitive(req.typed), JsonArray(known))
        return GuideServiceSuggestDocumentTypesResult.Ok(
            listOf("ID card", "Invoice", "Passport").filter { it.lowercase().startsWith(req.typed.lowercase()) },
        )
    }
}

suspend fun serverReport(input: JsonObject): JsonObject {
    val dispatch = createGuideServiceMcpDispatcher { _ -> GuideHandlers }
    val inFlight = GuideServiceMcpInFlight()
    val nowhere: GuideServiceMcpWriter = { }
    val http = createGuideServiceMcpHttpDispatcher(
        GuideServiceMcpHttpSettings(emptyList()),
    ) { _, _ -> GuideServiceMcpAdmitted.Caller(GuideHandlers) }
    val answers = mutableMapOf<String, JsonElement>()
    val calls = mutableMapOf<String, JsonElement>()
    for ((name, text) in input.getValue("requests").jsonObject) {
        reached.clear()
        val reply = dispatch(text.jsonPrimitive.content, nowhere, inFlight)
        answers[name] = if (reply == null) JsonNull else JsonPrimitive(reply)
        calls[name] = JsonArray(reached.toList())
    }
    val answered = mutableMapOf<String, JsonElement>()
    for (row in input.getValue("http").jsonArray) {
        val given = row.jsonObject
        val headers = given.getValue("headers").jsonArray.map { it.jsonArray[0].jsonPrimitive.content to it.jsonArray[1].jsonPrimitive.content }
        val response = respondGuideService(
            http,
            GuideServiceMcpHttpRequest("POST", "/mcp", headers, given.getValue("body").jsonPrimitive.content.encodeToByteArray()),
        )
        answered[given.getValue("name").jsonPrimitive.content] = buildJsonObject {
            put("status", response.status)
            put("body", response.body.decodeToString())
        }
    }
    val scopes = input.getValue("queries").jsonArray.map { query ->
        val asked = query.jsonArray
        val name = (asked[1] as? JsonPrimitive)?.takeIf { it.isString }?.content
        JsonArray(guideServiceMcpScopes(asked[0].jsonPrimitive.content, name).map { JsonPrimitive(it) })
    }
    return buildJsonObject {
        put("answers", JsonObject(answers))
        put("reached", JsonObject(calls))
        put("http", JsonObject(answered))
        put("scopes", JsonArray(scopes))
    }
}
"##;

const CLIENT_DRIVER: &str = r#"
class Canned : GuideServiceMcpTransport {
    val sent = mutableListOf<JsonElement>()
    val queue = ArrayDeque<String>()

    override suspend fun send(message: GuideServiceMcpMessage): String {
        sent.add(Json.parseToJsonElement(message.text))
        return queue.removeFirst()
    }
}

fun kebab(name: String): String = name.replace(Regex("([a-z])([A-Z])"), "\$1-\$2").lowercase()

fun greeted(result: GuideServiceGreetResult): JsonElement = when (result) {
    is GuideServiceGreetResult.Ok -> buildJsonObject { put("ok", result.value) }
    is GuideServiceGreetResult.Declared ->
        buildJsonObject { put("declared", Json.encodeToJsonElement(serializer<GuideError>(), result.error)) }
    is GuideServiceGreetResult.Fault -> buildJsonObject { put("fault", kebab(result.fault.kind.toString())) }
}

fun configured(result: GuideServiceConfigureOrganizationResult): JsonElement = when (result) {
    is GuideServiceConfigureOrganizationResult.Ok -> buildJsonObject { put("ok", result.value) }
    is GuideServiceConfigureOrganizationResult.Declared ->
        buildJsonObject { put("declared", Json.encodeToJsonElement(serializer<GuideError>(), result.error)) }
    is GuideServiceConfigureOrganizationResult.Fault -> buildJsonObject { put("fault", kebab(result.fault.kind.toString())) }
}

fun reviewed(result: GuideServiceReviewDocumentTypeResult): JsonElement = when (result) {
    is GuideServiceReviewDocumentTypeResult.Ok -> buildJsonObject {
        put("ok", JsonArray(result.value.map { Json.encodeToJsonElement(serializer<McpPromptMessage>(), it) }))
    }
    is GuideServiceReviewDocumentTypeResult.Declared ->
        buildJsonObject { put("declared", Json.encodeToJsonElement(serializer<GuideError>(), result.error)) }
    is GuideServiceReviewDocumentTypeResult.Fault -> buildJsonObject { put("fault", kebab(result.fault.kind.toString())) }
}

suspend fun clientReport(input: JsonObject): JsonObject {
    val canned = Canned()
    val client = GuideServiceMcpClient(canned)
    val calls: Map<String, suspend () -> JsonElement> = mapOf(
        "string" to { configured(client.configureOrganization(ConfigureOrganizationRequest())) },
        "named" to { greeted(client.greet("Ana")) },
        "messages" to { reviewed(client.reviewDocumentType(ReviewDocumentTypeRequest("ID card", null))) },
        "declared" to { greeted(client.greet("missing")) },
        "panicked" to { greeted(client.greet("boom")) },
        "unknown" to { greeted(client.greet("nobody")) },
        "unreadable" to { reviewed(client.reviewDocumentType(ReviewDocumentTypeRequest("ID card", "names"))) },
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
        guide_error_kotlin::kotlin_definition(),
        guide_version_kotlin::kotlin_definition(),
        configure_organization_request_kotlin::kotlin_definition(),
        get_guide_version_request_kotlin::kotlin_definition(),
        review_document_type_request_kotlin::kotlin_definition(),
        suggest_document_types_request_kotlin::kotlin_definition(),
        guide_service_schema::guide_service_fault_fields_kotlin::kotlin_definition(),
        guide_service_schema::guide_service_fault_kind_kotlin::kotlin_definition(),
        GuideServiceSchema::kotlin_http_client(),
        GuideServiceSchema::kotlin_ws_client(),
        GuideServiceSchema::kotlin_mcp_content(),
        GuideServiceSchema::kotlin_mcp_service(),
        GuideServiceSchema::kotlin_mcp_client(),
    ]
}

fn module() -> String {
    let literal = kotlin_text(&json!({ "server": input(), "client": client_input() }).to_string());
    let mut parts = vec![KOTLIN_IMPORTS.to_owned()];
    parts.extend(definitions());
    parts.push(kotlin_respond("GuideService"));
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
    assert_matches(&report["server"]);
}

#[test]
fn the_kotlin_client_reads_the_text_the_messages_and_the_error_the_rust_dispatcher_answers() {
    let Some(report) = driven() else {
        return;
    };
    assert_client(&report["client"]);
}
