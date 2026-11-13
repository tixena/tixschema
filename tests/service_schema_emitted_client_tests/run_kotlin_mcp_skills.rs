//! The emitted Kotlin `mcp` server run under a Kotlin toolchain over skills: a folder named on the
//! service and two skills an operation writes at run time. It answers the bytes the Rust
//! dispatcher answers to the same requests, alone and on an endpoint and over plain HTTP terms,
//! and every answer is held to the official 2026-07-28 schema. One `kotlinc` invocation; the
//! requests are computed by the Rust side ahead of the run.

#![cfg(feature = "kotlin")]

use super::LessonServiceSchema;
use super::lesson_service_schema;
use super::mcp_lesson_service::{assert_http, http_rows, lesson_answer, rows};
use super::mcp_library_rows::{HttpAnswer, ORIGIN, assert_answers};
use super::mcp_library_service::{book_error_kotlin, book_info_kotlin, book_request_kotlin};
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

const HANDLERS: &str = r##"
object LessonHandlers : LessonServiceHandlers {
    override suspend fun releaseNotesSkill(req: String): LessonServiceReleaseNotesSkillResult = when (req) {
        "SKILL.md" -> LessonServiceReleaseNotesSkillResult.Ok("# Release notes\n\nRead the changes since 2026-11-01, then write one line per change.\n")
        "references/changes.md" -> LessonServiceReleaseNotesSkillResult.Ok("- renamed a document\n")
        "panic" -> throw IllegalStateException("the handler came apart")
        else -> LessonServiceReleaseNotesSkillResult.Declared(BookErrorNotFound.validate())
    }

    override suspend fun figuresSkill(req: String): LessonServiceFiguresSkillResult = when (req) {
        "SKILL.md" -> LessonServiceFiguresSkillResult.Ok(%TUPLE%("# Figures\n".encodeToByteArray().map { it.toUByte() }, "text/markdown"))
        "figures.bin" -> LessonServiceFiguresSkillResult.Ok(%TUPLE%(listOf<UByte>(0u, 255u, 128u, 1u), "application/octet-stream"))
        else -> LessonServiceFiguresSkillResult.Declared(BookErrorNotFound.validate())
    }

    override suspend fun getLesson(req: BookRequest): LessonServiceGetLessonResult =
        if (req.bookId == "lesson-1") LessonServiceGetLessonResult.Ok(BookInfoDraft(3u).validate())
        else LessonServiceGetLessonResult.Declared(BookErrorNotFound.validate())
}

fun nullable(text: String?): JsonElement = if (text == null) JsonNull else JsonPrimitive(text)
"##;

const DRIVER: &str = r#"
suspend fun report(input: JsonObject): JsonObject {
    val endpoint = LessonServiceMcpEndpoint(lessonServiceMcpCatalog)
    val route = createLessonServiceMcpRoute(LessonHandlers)
    val alone = createLessonServiceMcpDispatcher { LessonHandlers }
    val nowhere: LessonServiceMcpWriter = { }
    val host = LessonServiceMcpHost(nowhere, LessonServiceMcpInFlight())
    val http = createLessonServiceMcpHttpDispatcher(
        LessonServiceMcpHttpSettings(listOf(input.getValue("origin").jsonPrimitive.content)),
    ) { _, _ -> LessonServiceMcpAdmitted.Caller(LessonHandlers) }
    val requests = input.getValue("requests").jsonArray.map { it.jsonPrimitive.content }
    val endpointAnswers = requests.map { text ->
        nullable(lessonServiceMcpServe(text, lessonServiceMcpServerName, lessonServiceMcpInstructions, endpoint, host) { _, _, method, params ->
            route(method, params)
        })
    }
    val httpAnswers = input.getValue("http").jsonArray.map { row ->
        val given = row.jsonObject
        val headers = given.getValue("headers").jsonArray.map { it.jsonArray[0].jsonPrimitive.content to it.jsonArray[1].jsonPrimitive.content }
        val answered = respondLessonService(
            http,
            LessonServiceMcpHttpRequest(
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
    return buildJsonObject {
        put("names", JsonArray(lessonServiceMcpCatalog.getValue("names").map { JsonPrimitive(it) }))
        put("endpoint", JsonArray(endpointAnswers))
        put("alone", JsonArray(requests.map { nullable(alone(it, nowhere, LessonServiceMcpInFlight())) }))
        put("http", JsonArray(httpAnswers))
    }
}

fun main() = runBlocking {
    println(report(Json.parseToJsonElement(INPUT).jsonObject).toString())
}
"#;

/// The name of the class `text` declares for a byte array and its content type.
fn wrapper_of_bytes(text: &str) -> String {
    let shape = "(val slot0: List<UByte>, val slot1: String)";
    let before = &text[..text.find(shape).unwrap()];
    let start = before.rfind("data class ").unwrap() + "data class ".len();
    before[start..].to_owned()
}

fn definitions() -> Vec<String> {
    vec![
        book_info_kotlin::kotlin_definition(),
        book_error_kotlin::kotlin_definition(),
        book_request_kotlin::kotlin_definition(),
        lesson_service_schema::lesson_service_fault_fields_kotlin::kotlin_definition(),
        lesson_service_schema::lesson_service_fault_kind_kotlin::kotlin_definition(),
        LessonServiceSchema::kotlin_http_client(),
        LessonServiceSchema::kotlin_ws_client(),
        LessonServiceSchema::kotlin_mcp_service(),
    ]
}

/// `text` as it is written between the quotes of a Kotlin string.
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
    let requests: Vec<String> = rows().into_iter().map(|row| row.text).collect();
    let http: Vec<Value> = http_rows()
        .into_iter()
        .map(|row| {
            json!({
                "headers": row.headers.iter().map(|(name, value)| json!([name, value])).collect::<Vec<_>>(),
                "body": row.body,
            })
        })
        .collect();
    let input = json!({ "origin": ORIGIN, "requests": requests, "http": http });
    let literal = kotlin_text(&input.to_string());
    let mut parts = vec![KOTLIN_IMPORTS.to_owned()];
    parts.extend(definitions());
    parts.push(HANDLERS.replace(
        "%TUPLE%",
        &wrapper_of_bytes(&LessonServiceSchema::kotlin_http_client()),
    ));
    parts.push(kotlin_respond("LessonService"));
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
    assert_answers("alone", lesson_answer, &texts(&report["alone"]), &rows);
    assert_answers(
        "endpoint",
        lesson_answer,
        &texts(&report["endpoint"]),
        &rows,
    );
}

#[test]
fn an_endpoint_names_every_skill_as_the_rust_registry_does() {
    let Some(report) = driven() else {
        return;
    };
    assert_eq!(
        report["names"],
        json!(
            LessonServiceSchema::MCP_NAMES
                .iter()
                .map(|(kind, name)| format!("{kind}:{name}"))
                .collect::<Vec<_>>()
        )
    );
}

#[test]
fn the_kotlin_server_answers_the_skills_over_plain_http_terms_as_rust_does() {
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
    assert_http("kotlin", &answered);
}
