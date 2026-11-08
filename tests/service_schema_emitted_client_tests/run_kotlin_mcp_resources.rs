//! The emitted Kotlin `mcp` server run under a Kotlin toolchain over resources, templates, a
//! listing and a completion: it answers the bytes the Rust dispatcher answers to the same
//! requests, for services that declare them and for one that declares none, alone and beside a
//! second service on one endpoint and over plain HTTP terms, and every answer is held to the
//! official 2026-07-28 schema. One `kotlinc` invocation; the requests are computed by the Rust
//! side ahead of the run.

#![cfg(feature = "kotlin")]

use super::mcp_library_rows::{
    HttpAnswer, HttpRow, ORIGIN, Row, assert_answers, assert_http, bare_answer, depot_answer,
    http_rows, library_answer, rows, workspace_answer,
};
use super::mcp_library_service::{
    book_error_kotlin, book_info_kotlin, book_request_kotlin, get_bin_request_kotlin,
    library_guide_request_kotlin, suggest_books_request_kotlin,
};
use super::runtime::ran_kotlin;
use super::{BareServiceSchema, DepotServiceSchema, LibraryServiceSchema};
use super::{bare_service_schema, depot_service_schema, library_service_schema};
use crate::mcp_streams::kotlin_respond;
use serde_json::{Value, json};
use std::sync::OnceLock;

pub const KOTLIN_IMPORTS: &str = "import kotlinx.coroutines.*\n\
     import kotlinx.coroutines.flow.*\n\
     import kotlinx.serialization.*\n\
     import kotlinx.serialization.json.*\n\
     import kotlinx.serialization.descriptors.*\n\
     import kotlinx.serialization.encoding.*\n\
     import kotlinx.serialization.builtins.*";

pub const HANDLERS: &str = r##"
object LibraryHandlers : LibraryServiceHandlers {
    override suspend fun getBook(req: BookRequest): LibraryServiceGetBookResult {
        if (req.bookId == "panic") throw IllegalStateException("the handler came apart")
        return if (req.bookId == "book-1") LibraryServiceGetBookResult.Ok(BookInfo(7u))
        else LibraryServiceGetBookResult.Declared(BookErrorNotFound)
    }

    override suspend fun libraryGuide(req: LibraryGuideRequest): LibraryServiceLibraryGuideResult =
        LibraryServiceLibraryGuideResult.Ok("# The library")

    override suspend fun bookCover(req: String): LibraryServiceBookCoverResult = when (req) {
        "book-1" -> LibraryServiceBookCoverResult.Ok(%TUPLE%(listOf<UByte>(37u, 80u, 68u, 70u, 45u), "application/pdf"))
        "book-2" -> LibraryServiceBookCoverResult.Ok(%TUPLE%(listOf<UByte>(0u, 255u, 128u, 1u, 2u, 3u, 4u), "image/png"))
        "panic" -> throw IllegalStateException("the handler came apart")
        else -> LibraryServiceBookCoverResult.Declared(BookErrorNotFound)
    }

    override suspend fun listBooks(req: String?): LibraryServiceListBooksResult = when (req) {
        null -> LibraryServiceListBooksResult.Ok(
            McpResourcePage(
                listOf(McpResource("library://covers/book-1", "cover-1.pdf", mimeType = "application/pdf")),
                "page-2",
            ),
        )
        "page-2" -> LibraryServiceListBooksResult.Ok(
            McpResourcePage(
                listOf(
                    McpResource(
                        "library://covers/book-2",
                        "cover-2.pdf",
                        title = "Second cover",
                        mimeType = "application/pdf",
                        size = 48213L,
                    ),
                ),
            ),
        )
        "panic" -> throw IllegalStateException("the handler came apart")
        else -> LibraryServiceListBooksResult.Declared(BookErrorNotFound)
    }

    override suspend fun suggestBooks(req: SuggestBooksRequest): LibraryServiceSuggestBooksResult = when (req.typed) {
        "many" -> LibraryServiceSuggestBooksResult.Ok((0 until 150).map { "many-$it" })
        "panic" -> throw IllegalStateException("the handler came apart")
        "none" -> LibraryServiceSuggestBooksResult.Declared(BookErrorNotFound)
        "kind" -> LibraryServiceSuggestBooksResult.Ok(listOf(req.chosen["kind"] ?: ""))
        else -> LibraryServiceSuggestBooksResult.Ok(listOf("book-1", "book-2").filter { it.startsWith(req.typed) })
    }
}

object DepotHandlers : DepotServiceHandlers {
    override suspend fun getCrate(req: String): DepotServiceGetCrateResult =
        if (req == "crate-1") DepotServiceGetCrateResult.Ok("{\"total\":120}")
        else DepotServiceGetCrateResult.Declared(BookErrorNotFound)

    override suspend fun getBin(req: GetBinRequest): DepotServiceGetBinResult =
        DepotServiceGetBinResult.Ok(req.aisle + "/" + req.binId)
}

object BareHandlers : BareServiceHandlers {
    override suspend fun getPlain(req: BookRequest): BareServiceGetPlainResult {
        if (req.bookId == "panic") throw IllegalStateException("the handler came apart")
        return if (req.bookId == "book-1") BareServiceGetPlainResult.Ok(BookInfo(7u))
        else BareServiceGetPlainResult.Declared(BookErrorNotFound)
    }
}

fun nullable(text: String?): JsonElement = if (text == null) JsonNull else JsonPrimitive(text)
"##;

const DRIVER: &str = r#"
suspend fun report(input: JsonObject): JsonObject {
    val workspace = LibraryServiceMcpEndpoint(libraryServiceMcpCatalog, depotServiceMcpCatalog)
    val libraryRoute = createLibraryServiceMcpRoute(LibraryHandlers)
    val depotRoute = createDepotServiceMcpRoute(DepotHandlers)
    val aloneLibrary = createLibraryServiceMcpDispatcher { LibraryHandlers }
    val aloneDepot = createDepotServiceMcpDispatcher { DepotHandlers }
    val aloneBare = createBareServiceMcpDispatcher { BareHandlers }
    val nowhere: LibraryServiceMcpWriter = { }
    val host = LibraryServiceMcpHost(nowhere, LibraryServiceMcpInFlight())
    val http = createLibraryServiceMcpHttpDispatcher(
        LibraryServiceMcpHttpSettings(listOf(input.getValue("origin").jsonPrimitive.content)),
    ) { _, _ -> LibraryServiceMcpAdmitted.Caller(LibraryHandlers) }
    val requests = input.getValue("requests").jsonArray.map { it.jsonPrimitive.content }
    val workspaceAnswers = requests.map { text ->
        nullable(libraryServiceMcpServe(text, "Workspace", null, workspace, host) { _, part, method, params ->
            when (part) {
                0 -> libraryRoute(method, params)
                1 -> depotRoute(method, params)?.let { answered ->
                    when (answered) {
                        is DepotServiceMcpAnswer.Answered -> LibraryServiceMcpAnswer.Answered(answered.result)
                        is DepotServiceMcpAnswer.Refused -> LibraryServiceMcpAnswer.Refused(answered.error)
                    }
                }
                else -> null
            }
        })
    }
    val httpAnswers = input.getValue("http").jsonArray.map { row ->
        val given = row.jsonObject
        val headers = given.getValue("headers").jsonArray.map { it.jsonArray[0].jsonPrimitive.content to it.jsonArray[1].jsonPrimitive.content }
        val answered = respondLibraryService(
            http,
            LibraryServiceMcpHttpRequest(
                "POST",
                "/mcp",
                headers,
                given.getValue("body").jsonPrimitive.content.encodeToByteArray(),
            ),
        )
        buildJsonObject {
            put("status", answered.status)
            put("headers", JsonArray(answered.headers.map { JsonArray(listOf(JsonPrimitive(it.first), JsonPrimitive(it.second))) }))
            put("body", answered.body.decodeToString())
        }
    }
    val shared = try {
        LibraryServiceMcpEndpoint(
            libraryServiceMcpCatalog,
            mapOf("names" to listOf("resource:guide://library")),
        )
        "none"
    } catch (refused: IllegalStateException) {
        refused.message ?: ""
    }
    val kept = try {
        LibraryServiceMcpEndpoint(
            libraryServiceMcpCatalog,
            mapOf("names" to listOf("tool:guide://library", "resource template:guide://library")),
        )
        "built"
    } catch (refused: IllegalStateException) {
        refused.message ?: ""
    }
    return buildJsonObject {
        put("names", JsonArray(listOf(libraryServiceMcpCatalog, depotServiceMcpCatalog, bareServiceMcpCatalog).map { catalog ->
            JsonArray(catalog.getValue("names").map { JsonPrimitive(it) })
        }))
        put("shared", shared)
        put("kept", kept)
        put("workspace", JsonArray(workspaceAnswers))
        put("library", JsonArray(requests.map { nullable(aloneLibrary(it, nowhere, LibraryServiceMcpInFlight())) }))
        put("depot", JsonArray(requests.map { nullable(aloneDepot(it, nowhere, DepotServiceMcpInFlight())) }))
        put("bare", JsonArray(requests.map { nullable(aloneBare(it, nowhere, BareServiceMcpInFlight())) }))
        put("http", JsonArray(httpAnswers))
    }
}

fun main() = runBlocking {
    println(report(Json.parseToJsonElement(INPUT).jsonObject).toString())
}
"#;

/// The name of the class `text` declares for a byte array and its content type.
pub fn wrapper_of_bytes(text: &str) -> String {
    let shape = "(val slot0: List<UByte>, val slot1: String)";
    let before = &text[..text.find(shape).unwrap()];
    let start = before.rfind("data class ").unwrap() + "data class ".len();
    before[start..].to_owned()
}

pub fn definitions() -> Vec<String> {
    vec![
        book_info_kotlin::kotlin_definition(),
        book_error_kotlin::kotlin_definition(),
        book_request_kotlin::kotlin_definition(),
        library_guide_request_kotlin::kotlin_definition(),
        suggest_books_request_kotlin::kotlin_definition(),
        get_bin_request_kotlin::kotlin_definition(),
        library_service_schema::library_service_fault_fields_kotlin::kotlin_definition(),
        library_service_schema::library_service_fault_kind_kotlin::kotlin_definition(),
        depot_service_schema::depot_service_fault_fields_kotlin::kotlin_definition(),
        depot_service_schema::depot_service_fault_kind_kotlin::kotlin_definition(),
        bare_service_schema::bare_service_fault_fields_kotlin::kotlin_definition(),
        bare_service_schema::bare_service_fault_kind_kotlin::kotlin_definition(),
        LibraryServiceSchema::kotlin_http_client(),
        LibraryServiceSchema::kotlin_ws_client(),
        LibraryServiceSchema::kotlin_mcp_service(),
        DepotServiceSchema::kotlin_http_client(),
        DepotServiceSchema::kotlin_ws_client(),
        DepotServiceSchema::kotlin_mcp_service(),
        BareServiceSchema::kotlin_http_client(),
        BareServiceSchema::kotlin_ws_client(),
        BareServiceSchema::kotlin_mcp_service(),
    ]
}

/// `text` as it is written between the quotes of a Kotlin string.
pub fn kotlin_text(text: &str) -> String {
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

fn input(requests: &[Row], http: &[HttpRow]) -> Value {
    json!({
        "origin": ORIGIN,
        "requests": requests.iter().map(|row| row.text.clone()).collect::<Vec<_>>(),
        "http": http.iter().map(|row| json!({
            "headers": row.headers.iter().map(|(name, value)| json!([name, value])).collect::<Vec<_>>(),
            "body": row.body,
        })).collect::<Vec<_>>(),
    })
}

fn module() -> String {
    let literal = kotlin_text(&input(&rows(), &http_rows()).to_string());
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

fn texts(report: &Value) -> Vec<Option<String>> {
    report
        .as_array()
        .unwrap()
        .iter()
        .map(|answered| answered.as_str().map(str::to_owned))
        .collect()
}

#[test]
fn the_kotlin_server_answers_the_bytes_the_rust_dispatcher_answers() {
    let Some(report) = driven() else {
        return;
    };
    let rows = rows();
    assert_answers(
        "endpoint",
        workspace_answer,
        &texts(&report["workspace"]),
        &rows,
    );
    assert_answers("library", library_answer, &texts(&report["library"]), &rows);
    assert_answers("depot", depot_answer, &texts(&report["depot"]), &rows);
    assert_answers("bare", bare_answer, &texts(&report["bare"]), &rows);
}

#[test]
fn the_kotlin_server_answers_resource_requests_over_plain_http_terms_as_rust_does() {
    let Some(report) = driven() else {
        return;
    };
    let answered: Vec<HttpAnswer> = report["http"]
        .as_array()
        .unwrap()
        .iter()
        .map(|held| {
            (
                u16::try_from(held["status"].as_u64().unwrap()).unwrap(),
                held["headers"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|pair| {
                        (
                            pair[0].as_str().unwrap().to_owned(),
                            pair[1].as_str().unwrap().to_owned(),
                        )
                    })
                    .collect(),
                held["body"].as_str().unwrap().to_owned(),
            )
        })
        .collect();
    assert_http("library", &http_rows(), &answered);
}

#[test]
fn an_endpoint_names_a_resource_and_a_template_by_their_kind_and_refuses_one_two_services_share() {
    let Some(report) = driven() else {
        return;
    };
    assert_eq!(
        report["names"],
        json!([
            [
                "resource template:library://covers/{book_id}",
                "tool:get-book",
                "resource:guide://library"
            ],
            [
                "resource template:depot://aisles/{aisle}/bins/{bin_id}",
                "resource template:depot://crates/{crate_id}"
            ],
            ["tool:get-plain"],
        ])
    );
    assert_eq!(
        report["shared"],
        "two services share the MCP name resource:guide://library"
    );
    assert_eq!(
        report["kept"], "built",
        "a tool, a resource and a template of one text do not collide"
    );
}
