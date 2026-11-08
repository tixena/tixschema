//! The emitted Kotlin `mcp` server and client run under a Kotlin toolchain on `JobService`: the
//! server's answer to each task request, HTTP request, scope query and `subscriptions/listen` is
//! the one the Rust dispatcher gives, and the client reads the handle and the state of a task back
//! as ordinary successes. One `kotlinc` invocation computes both reports.

#![cfg(feature = "kotlin")]

use super::JobServiceSchema;
use super::job_service_schema;
use super::mcp_mobile_rows::kotlin_text;
use super::mcp_task_rows::{
    answer_export_request_kotlin, assert_client, assert_matches, client_input,
    export_report_kotlin, export_request_kotlin, input, job_error_kotlin,
};
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
const HANDLERS: &str = r#"
val reached = mutableListOf<JsonElement>()

fun note(vararg parts: JsonElement) {
    reached.add(JsonArray(parts.toList()))
}

fun task(id: String, updated: String = "2026-11-01T20:00:00Z") =
    McpTask(id, "2026-11-01T20:00:00Z", updated, 3600000L, 2000L)

object JobHandlers : JobServiceHandlers {
    override suspend fun startExport(req: ExportRequest): JobServiceStartExportResult {
        note(JsonPrimitive("start-export"), JsonPrimitive(req.folder))
        if (req.folder == "missing") return JobServiceStartExportResult.Declared(JobErrorNotFound)
        return JobServiceStartExportResult.Ok(task("7f3a"))
    }

    override suspend fun exportStatus(req: String): JobServiceExportStatusResult {
        note(JsonPrimitive("export-status"), JsonPrimitive(req))
        if (req == "boom") throw IllegalStateException("the handler came apart")
        return when (req) {
            "7f3a" -> JobServiceExportStatusResult.Ok(McpTaskState.Working(task(req)))
            "done" -> JobServiceExportStatusResult.Ok(McpTaskState.Completed(task(req, "2026-11-01T20:04:10Z"), ExportReport(3u)))
            "needs" -> JobServiceExportStatusResult.Ok(
                McpTaskState.InputRequired(
                    task(req),
                    mapOf(
                        "overwrite" to buildJsonObject {
                            put("method", "elicitation/create")
                            put("params", buildJsonObject {
                                put("message", "Overwrite the folder?")
                                put("mode", "form")
                                put("requestedSchema", buildJsonObject { put("properties", buildJsonObject { }); put("type", "object") })
                            })
                        },
                    ),
                ),
            )
            "broken" -> JobServiceExportStatusResult.Ok(McpTaskState.Failed(task(req), McpTaskError(-32603, "the folder vanished")))
            "stopped" -> JobServiceExportStatusResult.Ok(McpTaskState.Cancelled(task(req)))
            else -> JobServiceExportStatusResult.Declared(JobErrorNotFound)
        }
    }

    override suspend fun cancelExport(req: String): JobServiceCancelExportResult {
        note(JsonPrimitive("cancel-export"), JsonPrimitive(req))
        if (req == "gone") return JobServiceCancelExportResult.Declared(JobErrorNotFound)
        return JobServiceCancelExportResult.Ok
    }

    override suspend fun answerExport(req: AnswerExportRequest): JobServiceAnswerExportResult {
        note(JsonPrimitive("answer-export"), JsonPrimitive(req.taskId), JsonPrimitive(req.key), JsonPrimitive(req.action), req.content?.let { JsonPrimitive(it) } ?: JsonNull)
        if (req.taskId == "gone") return JobServiceAnswerExportResult.Declared(JobErrorNotFound)
        return JobServiceAnswerExportResult.Ok
    }
}

fun changesOf(script: JsonArray): JobServiceMcpChanges {
    val queue = ArrayDeque<JobServiceMcpChange>(script.map { spec ->
        val given = spec.jsonObject
        if (given.getValue("kind").jsonPrimitive.content == "task") {
            JobServiceMcpChange.Task(given.getValue("taskId").jsonPrimitive.content, given.getValue("state").jsonObject)
        } else {
            JobServiceMcpChange.KeepAlive
        }
    })
    return { queue.removeFirstOrNull() }
}

suspend fun serverReport(input: JsonObject): JsonObject {
    val dispatch = createJobServiceMcpDispatcher { _ -> JobHandlers }
    val inFlight = JobServiceMcpInFlight()
    val nowhere: JobServiceMcpWriter = { }
    val http = createJobServiceMcpHttpDispatcher(
        JobServiceMcpHttpSettings(emptyList()),
    ) { _, _ -> JobServiceMcpAdmitted.Caller(JobHandlers) }
    val answers = mutableMapOf<String, JsonElement>()
    val calls = mutableMapOf<String, JsonElement>()
    for ((name, text) in input.getValue("requests").jsonObject) {
        reached.clear()
        val reply = dispatch(text.jsonPrimitive.content, nowhere, inFlight)
        answers[name] = if (reply == null) JsonNull else JsonPrimitive(reply)
        calls[name] = JsonArray(reached.toList())
    }
    reached.clear()
    val answered = mutableMapOf<String, JsonElement>()
    for (row in input.getValue("http").jsonArray) {
        val given = row.jsonObject
        val headers = given.getValue("headers").jsonArray.map { it.jsonArray[0].jsonPrimitive.content to it.jsonArray[1].jsonPrimitive.content }
        val response = respondJobService(
            http,
            JobServiceMcpHttpRequest("POST", "/mcp", headers, given.getValue("body").jsonPrimitive.content.encodeToByteArray()),
        )
        answered[given.getValue("name").jsonPrimitive.content] = buildJsonObject {
            put("status", response.status)
            put("body", response.body.decodeToString())
        }
    }
    val scopes = input.getValue("queries").jsonArray.map { query ->
        val asked = query.jsonArray
        val name = (asked[1] as? JsonPrimitive)?.takeIf { it.isString }?.content
        JsonArray(jobServiceMcpScopes(asked[0].jsonPrimitive.content, name).map { JsonPrimitive(it) })
    }
    val listens = input.getValue("listens").jsonArray.map { row ->
        val given = row.jsonObject
        val written = mutableListOf<JsonElement>()
        val listening = createJobServiceMcpDispatcher(changesOf(given.getValue("script").jsonArray)) { _ -> JobHandlers }
        val reply = listening(given.getValue("request").jsonPrimitive.content, { written.add(JsonPrimitive(it)) }, JobServiceMcpInFlight())
        buildJsonObject {
            put("reply", if (reply == null) JsonNull else JsonPrimitive(reply))
            put("written", JsonArray(written))
        }
    }
    return buildJsonObject {
        put("answers", JsonObject(answers))
        put("reached", JsonObject(calls))
        put("http", JsonObject(answered))
        put("scopes", JsonArray(scopes))
        put("listens", JsonArray(listens))
    }
}
"#;

const CLIENT_DRIVER: &str = r#"
class Canned : JobServiceMcpTransport {
    val sent = mutableListOf<JsonElement>()
    val queue = ArrayDeque<String>()

    override suspend fun send(message: JobServiceMcpMessage): String {
        sent.add(Json.parseToJsonElement(message.text))
        return queue.removeFirst()
    }
}

fun kebab(name: String): String = name.replace(Regex("([a-z])([A-Z])"), "\$1-\$2").lowercase()

fun declared(error: JobError): JsonElement = buildJsonObject { put("declared", Json.encodeToJsonElement(serializer<JobError>(), error)) }

fun fault(kind: Any): JsonElement = buildJsonObject { put("fault", kebab(kind.toString())) }

fun started(result: JobServiceStartExportResult): JsonElement = when (result) {
    is JobServiceStartExportResult.Ok -> buildJsonObject {
        put("ok", JsonObject((Json.encodeToJsonElement(McpTask.serializer(), result.value) as JsonObject) - "status"))
    }
    is JobServiceStartExportResult.Declared -> declared(result.error)
    is JobServiceStartExportResult.Fault -> fault(result.fault.kind)
}

fun status(result: JobServiceExportStatusResult): JsonElement = when (result) {
    is JobServiceExportStatusResult.Ok -> buildJsonObject {
        put("ok", Json.encodeToJsonElement(McpTaskState.serializer(serializer<ExportReport>()), result.value))
    }
    is JobServiceExportStatusResult.Declared -> declared(result.error)
    is JobServiceExportStatusResult.Fault -> fault(result.fault.kind)
}

fun cancelled(result: JobServiceCancelExportResult): JsonElement = when (result) {
    is JobServiceCancelExportResult.Ok -> buildJsonObject { put("ok", JsonNull) }
    is JobServiceCancelExportResult.Declared -> declared(result.error)
    is JobServiceCancelExportResult.Fault -> fault(result.fault.kind)
}

fun answered(result: JobServiceAnswerExportResult): JsonElement = when (result) {
    is JobServiceAnswerExportResult.Ok -> buildJsonObject { put("ok", JsonNull) }
    is JobServiceAnswerExportResult.Declared -> declared(result.error)
    is JobServiceAnswerExportResult.Fault -> fault(result.fault.kind)
}

suspend fun clientReport(input: JsonObject): JsonObject {
    val canned = Canned()
    val client = JobServiceMcpClient(canned)
    val calls: Map<String, suspend () -> JsonElement> = mapOf(
        "started" to { started(client.startExport(ExportRequest("reports"))) },
        "working" to { status(client.exportStatus("7f3a")) },
        "completed" to { status(client.exportStatus("done")) },
        "waiting" to { status(client.exportStatus("needs")) },
        "failed" to { status(client.exportStatus("broken")) },
        "cancelled" to { status(client.exportStatus("stopped")) },
        "cancel" to { cancelled(client.cancelExport("7f3a")) },
        "answer" to { answered(client.answerExport(AnswerExportRequest("needs", "overwrite", "accept", "{}"))) },
        "declared" to { status(client.exportStatus("gone")) },
        "panicked" to { status(client.exportStatus("boom")) },
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
        job_error_kotlin::kotlin_definition(),
        export_request_kotlin::kotlin_definition(),
        export_report_kotlin::kotlin_definition(),
        answer_export_request_kotlin::kotlin_definition(),
        job_service_schema::job_service_fault_fields_kotlin::kotlin_definition(),
        job_service_schema::job_service_fault_kind_kotlin::kotlin_definition(),
        JobServiceSchema::kotlin_http_client(),
        JobServiceSchema::kotlin_ws_client(),
        JobServiceSchema::kotlin_mcp_task(),
        JobServiceSchema::kotlin_mcp_service(),
        JobServiceSchema::kotlin_mcp_client(),
    ]
}

fn module() -> String {
    let literal = kotlin_text(&json!({ "server": input(), "client": client_input() }).to_string());
    let mut parts = vec![KOTLIN_IMPORTS.to_owned()];
    parts.extend(definitions());
    parts.push(kotlin_respond("JobService"));
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
fn the_kotlin_client_reads_the_handle_and_the_state_the_rust_dispatcher_answers() {
    let Some(report) = driven() else {
        return;
    };
    assert_client(&report["client"]);
}
