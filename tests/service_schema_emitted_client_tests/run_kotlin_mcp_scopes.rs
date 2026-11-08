//! The emitted Kotlin `mcp` server run under a Kotlin toolchain over a service whose resources,
//! template, listing and completion declare scopes: its scopes function and its HTTP server under
//! OAuth answer what the Rust dispatcher answers.

#![cfg(feature = "kotlin")]

use super::mcp_library_service::{book_error_kotlin, book_info_kotlin, book_request_kotlin};
use super::mcp_stacks_service::{
    assert_matches_rust, input, open_notice_request_kotlin, pinned_item_request_kotlin,
    stacks_index_request_kotlin, suggest_items_request_kotlin,
};
use super::runtime::ran_kotlin;
use super::{StacksServiceSchema, stacks_service_schema};
use crate::mcp_streams::kotlin_respond;
use serde_json::Value;

const KOTLIN_IMPORTS: &str = "import kotlinx.coroutines.*\n\
     import kotlinx.coroutines.flow.*\n\
     import kotlinx.serialization.*\n\
     import kotlinx.serialization.json.*\n\
     import kotlinx.serialization.descriptors.*\n\
     import kotlinx.serialization.encoding.*\n\
     import kotlinx.serialization.builtins.*";

const DRIVER: &str = r##"
var reached = 0

object Handlers : StacksServiceHandlers {
    override suspend fun stacksIndex(req: StacksIndexRequest): StacksServiceStacksIndexResult {
        reached += 1
        return StacksServiceStacksIndexResult.Ok("# The stacks")
    }

    override suspend fun pinnedItem(req: PinnedItemRequest): StacksServicePinnedItemResult {
        reached += 1
        return StacksServicePinnedItemResult.Ok("pinned")
    }

    override suspend fun getItem(req: String): StacksServiceGetItemResult {
        reached += 1
        return StacksServiceGetItemResult.Ok(req)
    }

    override suspend fun openNotice(req: OpenNoticeRequest): StacksServiceOpenNoticeResult {
        reached += 1
        return StacksServiceOpenNoticeResult.Ok("open")
    }

    override suspend fun listItems(req: String?): StacksServiceListItemsResult {
        reached += 1
        return StacksServiceListItemsResult.Ok(StacksServiceMcpResourcePage(listOf(StacksServiceMcpResource("stacks://items/item-1", "item-1"))))
    }

    override suspend fun suggestItems(req: SuggestItemsRequest): StacksServiceSuggestItemsResult {
        reached += 1
        return StacksServiceSuggestItemsResult.Ok(listOf("item-1", "item-2").filter { it.startsWith(req.typed) })
    }

    override suspend fun countItems(req: BookRequest): StacksServiceCountItemsResult {
        reached += 1
        return StacksServiceCountItemsResult.Ok(BookInfo(3u))
    }
}

suspend fun report(input: JsonObject): JsonObject {
    val tokens = mapOf("read-token" to "documents:read", "write-token" to "documents:read documents:write")
    val oauth = input.getValue("oauth").jsonObject
    val strings = { name: String -> oauth.getValue(name).jsonArray.map { it.jsonPrimitive.content } }
    val settings = StacksServiceMcpHttpSettings(
        listOf(input.getValue("origin").jsonPrimitive.content),
        StacksServiceMcpOAuth(
            strings("authorizationServers"),
            oauth.getValue("resource").jsonPrimitive.content,
            oauth.getValue("resourceMetadataUrl").jsonPrimitive.content,
            strings("scopesSupported"),
        ),
    )
    val serve = createStacksServiceMcpHttpDispatcher(settings) { request, _ ->
        val needed = stacksServiceMcpScopes(request.header("mcp-method") ?: "", request.mcpName())
        val token = request.bearerToken()
        val held = tokens[token]
        val scope = needed.joinToString(" ")
        when {
            token == null -> StacksServiceMcpAdmitted.Denied(
                StacksServiceMcpDenied.NoValidCredentials(if (needed.isEmpty()) null else scope),
            )
            held == null -> StacksServiceMcpAdmitted.Denied(StacksServiceMcpDenied.NoValidCredentials())
            needed.any { it !in held.split(" ") } -> StacksServiceMcpAdmitted.Denied(
                StacksServiceMcpDenied.InsufficientScope(scope),
            )
            else -> StacksServiceMcpAdmitted.Caller(Handlers)
        }
    }
    val answers = input.getValue("rows").jsonArray.map { row ->
        val given = row.jsonObject
        val before = reached
        val response = respondStacksService(
            serve,
            StacksServiceMcpHttpRequest(
                given.getValue("method").jsonPrimitive.content,
                given.getValue("path").jsonPrimitive.content,
                given.getValue("headers").jsonArray.map { it.jsonArray[0].jsonPrimitive.content to it.jsonArray[1].jsonPrimitive.content },
                given.getValue("body").jsonPrimitive.content.encodeToByteArray(),
            ),
        )
        buildJsonObject {
            put("status", response.status)
            put("headers", JsonArray(response.headers.map { JsonArray(listOf(JsonPrimitive(it.first), JsonPrimitive(it.second))) }))
            put("body", response.body.decodeToString())
            put("reached", reached - before)
        }
    }
    val scopes = input.getValue("queries").jsonArray.map { query ->
        val asked = query.jsonArray
        val name = asked[1].let { if (it is JsonNull) null else it.jsonPrimitive.content }
        JsonArray(stacksServiceMcpScopes(asked[0].jsonPrimitive.content, name).map { JsonPrimitive(it) })
    }
    return buildJsonObject {
        put("answers", JsonArray(answers))
        put("scopes", JsonArray(scopes))
    }
}

fun main() = runBlocking {
    println(report(Json.parseToJsonElement(INPUT).jsonObject).toString())
}
"##;

fn definitions() -> Vec<String> {
    vec![
        book_info_kotlin::kotlin_definition(),
        book_error_kotlin::kotlin_definition(),
        book_request_kotlin::kotlin_definition(),
        stacks_index_request_kotlin::kotlin_definition(),
        pinned_item_request_kotlin::kotlin_definition(),
        open_notice_request_kotlin::kotlin_definition(),
        suggest_items_request_kotlin::kotlin_definition(),
        stacks_service_schema::stacks_service_fault_fields_kotlin::kotlin_definition(),
        stacks_service_schema::stacks_service_fault_kind_kotlin::kotlin_definition(),
        StacksServiceSchema::kotlin_http_client(),
        StacksServiceSchema::kotlin_ws_client(),
        StacksServiceSchema::kotlin_mcp_content(),
        StacksServiceSchema::kotlin_mcp_service(),
    ]
}

fn kotlin_text(text: &str) -> String {
    let mut written = String::with_capacity(text.len());
    for letter in text.chars() {
        match letter {
            '\\' => written.push_str("\\\\"),
            '"' => written.push_str("\\\""),
            '$' => written.push_str("\\$"),
            '\n' => written.push_str("\\n"),
            '\r' => written.push_str("\\r"),
            '\t' => written.push_str("\\t"),
            other => written.push(other),
        }
    }
    written
}

fn module() -> String {
    let literal = kotlin_text(&input().to_string());
    let mut parts = vec![KOTLIN_IMPORTS.to_owned()];
    parts.extend(definitions());
    parts.push(kotlin_respond("StacksService"));
    parts.push(format!("const val INPUT = \"{literal}\""));
    parts.push(DRIVER.to_owned());
    parts.join("\n\n")
}

#[test]
fn the_kotlin_server_answers_the_scopes_of_a_resource_as_the_rust_dispatcher_does() {
    let Some(written) = ran_kotlin(&module()) else {
        return;
    };
    let report: Value = serde_json::from_str(written.trim()).unwrap();
    assert_matches_rust("kotlin", &report);
}
