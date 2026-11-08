//! The emitted Kotlin `mcp` server and client run under a Kotlin toolchain: the server answers the
//! bytes the Rust dispatcher answers to the same requests, alone and beside a second service on
//! one endpoint and over plain HTTP terms, and the client reads the outcomes the design lists from
//! the answers the Rust dispatcher gives it. One `kotlinc` invocation; the requests and the
//! canned answers are computed by the Rust side ahead of the run.

#![cfg(feature = "kotlin")]

use super::mcp_auth_rows::{
    AUTHORIZATION_SERVER, METADATA_URL, RESOURCE, SCOPE_QUERIES, SCOPES_SUPPORTED, auth_rows,
};
use super::mcp_mobile_document;
use super::mcp_mobile_rows::block_on;
use super::mcp_mobile_rows::{
    CLIENT_LINES, alone_answer, assert_answer_valid, assert_client_request, client_calls,
    client_replies, kotlin_text,
};
use super::mcp_mobile_server_rows::{
    HttpRow, ORIGIN, ServerRow, billing_answer, expected, http_rows, observed, rust_http,
    server_rows, workspace_answer,
};
use super::mcp_mobile_service::Archive;
use super::mcp_mobile_service::{
    balance_error_kotlin, balance_response_kotlin, document_id_kotlin, get_version_request_kotlin,
    version_error_kotlin, version_response_kotlin,
};
use super::runtime::ran_kotlin;
use super::{MobileBillingServiceSchema, MobileDocumentServiceSchema};
use super::{mobile_billing_service_schema, mobile_document_service_schema};
use serde_json::{Value, json};
use std::sync::OnceLock;

const KOTLIN_IMPORTS: &str = "import kotlinx.coroutines.*\n\
     import kotlinx.coroutines.flow.*\n\
     import kotlinx.serialization.*\n\
     import kotlinx.serialization.json.*\n\
     import kotlinx.serialization.descriptors.*\n\
     import kotlinx.serialization.encoding.*\n\
     import kotlinx.serialization.builtins.*";

/// The handlers of both services: the logic `Archive` answers with.
const HANDLERS: &str = r#"
object DocumentHandlers : MobileDocumentServiceHandlers {
    private fun versionOf(id: String): VersionResponse? = when (id) {
        "doc-1" -> VersionResponse(7u)
        "panic" -> throw IllegalStateException("the handler came apart")
        else -> null
    }

    override suspend fun getTagged(req: String, knownTag: String?): MobileDocumentServiceGetTaggedResult {
        val found = versionOf(req)
        return if (found != null) {
            MobileDocumentServiceGetTaggedResult.Ok(MobileDocumentServiceGetTaggedResultValue(found, "\"v7\""))
        } else {
            MobileDocumentServiceGetTaggedResult.Declared(
                MobileDocumentServiceGetTaggedResultDeclared(VersionErrorNotFound, "no such document"),
            )
        }
    }

    override suspend fun getVersion(req: GetVersionRequest): MobileDocumentServiceGetVersionResult {
        val found = versionOf(req.documentId)
        return if (found != null) MobileDocumentServiceGetVersionResult.Ok(found)
        else MobileDocumentServiceGetVersionResult.Declared(VersionErrorNotFound)
    }

    override suspend fun lookup(req: DocumentId): MobileDocumentServiceLookupResult {
        val found = versionOf(req.value)
        return if (found != null) MobileDocumentServiceLookupResult.Ok(found)
        else MobileDocumentServiceLookupResult.Declared(VersionErrorNotFound)
    }

    override suspend fun renameDocument(req: RenameDocumentRequest): MobileDocumentServiceRenameDocumentResult =
        if (versionOf(req.documentId) != null) MobileDocumentServiceRenameDocumentResult.Ok
        else MobileDocumentServiceRenameDocumentResult.Declared(VersionErrorNotFound)

    override suspend fun archiveDocument(req: ArchiveDocumentRequest, exchange: String): MobileDocumentServiceArchiveDocumentResult =
        if (versionOf(req.documentId) != null) MobileDocumentServiceArchiveDocumentResult.Ok
        else MobileDocumentServiceArchiveDocumentResult.Declared(VersionErrorNotFound)

    override suspend fun tagDocument(
        req: String,
        arguments: String,
        message: String,
        outcome: String,
        members: String,
    ): MobileDocumentServiceTagDocumentResult = MobileDocumentServiceTagDocumentResult.Ok(VersionResponse(9u))

    override suspend fun labelDocument(
        req: String,
        sent: String,
        nextId: String,
        mcpSource: String,
        carried: String,
    ): MobileDocumentServiceLabelDocumentResult = MobileDocumentServiceLabelDocumentResult.Ok(VersionResponse(10u))

    override suspend fun touchDocument(req: TouchDocumentRequest) {
        if (req.reader == "panic") throw IllegalStateException("the handler came apart")
    }
}

object BillingHandlers : MobileBillingServiceHandlers {
    override suspend fun getBalance(req: String, ledger: String): MobileBillingServiceGetBalanceResult =
        if (req == "acct-1") MobileBillingServiceGetBalanceResult.Ok(BalanceResponse(1250L))
        else MobileBillingServiceGetBalanceResult.Declared(BalanceErrorUnknownAccount)
}

fun nullable(text: String?): JsonElement = if (text == null) JsonNull else JsonPrimitive(text)

fun kind(fault: MobileDocumentServiceFaultFields): String =
    fault.kind.name.replace(Regex("([a-z])([A-Z])"), "$1-$2").lowercase()
"#;

const SERVER_DRIVER: &str = r#"
suspend fun serverReport(input: JsonObject): JsonObject {
    val workspace = MobileDocumentServiceMcpEndpoint(mobileDocumentServiceMcpCatalog, mobileBillingServiceMcpCatalog)
    val documentRoute = createMobileDocumentServiceMcpRoute(DocumentHandlers)
    val billingRoute = createMobileBillingServiceMcpRoute(BillingHandlers)
    val alone = createMobileDocumentServiceMcpDispatcher(DocumentHandlers)
    val billingAlone = createMobileBillingServiceMcpDispatcher(BillingHandlers)
    val http = createMobileDocumentServiceMcpHttpDispatcher(
        MobileDocumentServiceMcpHttpSettings(listOf(input.getValue("origin").jsonPrimitive.content)),
    ) { _, _ -> MobileDocumentServiceMcpAdmitted.Caller(DocumentHandlers) }
    val requests = input.getValue("requests").jsonArray.map { it.jsonPrimitive.content }
    val workspaceAnswers = requests.map { text ->
        nullable(mobileDocumentServiceMcpServe(text, "Workspace", null, workspace) { part, method, params ->
            when (part) {
                0 -> documentRoute(method, params)
                1 -> billingRoute(method, params)?.let { answered ->
                    when (answered) {
                        is MobileBillingServiceMcpAnswer.Answered -> MobileDocumentServiceMcpAnswer.Answered(answered.result)
                        is MobileBillingServiceMcpAnswer.Refused -> MobileDocumentServiceMcpAnswer.Refused(answered.error)
                    }
                }
                else -> null
            }
        })
    }
    val aloneAnswers = requests.map { nullable(alone(it)) }
    val billingAnswers = requests.map { nullable(billingAlone(it)) }
    val httpAnswers = input.getValue("http").jsonArray.map { row ->
        val given = row.jsonObject
        val headers = given.getValue("headers").jsonArray.map { it.jsonArray[0].jsonPrimitive.content to it.jsonArray[1].jsonPrimitive.content }
        val answered = http(
            MobileDocumentServiceMcpHttpRequest(
                given.getValue("method").jsonPrimitive.content,
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
        MobileDocumentServiceMcpEndpoint(
            mapOf("names" to listOf("tool:get-version")),
            mapOf("names" to listOf("prompt:get-version", "tool:get-version")),
        )
        "none"
    } catch (refused: IllegalStateException) {
        refused.message ?: ""
    }
    val kept = try {
        MobileDocumentServiceMcpEndpoint(
            mapOf("names" to listOf("tool:get-version")),
            mapOf("names" to listOf("prompt:get-version")),
        )
        "built"
    } catch (refused: IllegalStateException) {
        refused.message ?: ""
    }
    return buildJsonObject {
        put("workspace", JsonArray(workspaceAnswers))
        put("alone", JsonArray(aloneAnswers))
        put("billing", JsonArray(billingAnswers))
        put("http", JsonArray(httpAnswers))
        put("shared", shared)
        put("kept", kept)
    }
}
"#;

/// Authorization: every request of `auth_rows()` through the server with the `admit` of its scheme,
/// and every scope query through `mobileDocumentServiceMcpScopes`.
const AUTH_DRIVER: &str = r#"
suspend fun authReport(input: JsonObject): JsonObject {
    var reached = 0
    val counted = object : MobileDocumentServiceHandlers by DocumentHandlers {
        override suspend fun getVersion(req: GetVersionRequest): MobileDocumentServiceGetVersionResult {
            reached += 1
            return DocumentHandlers.getVersion(req)
        }

        override suspend fun renameDocument(req: RenameDocumentRequest): MobileDocumentServiceRenameDocumentResult {
            reached += 1
            return DocumentHandlers.renameDocument(req)
        }

        override suspend fun touchDocument(req: TouchDocumentRequest) {
            reached += 1
            DocumentHandlers.touchDocument(req)
        }
    }
    val tokens = mapOf("read-token" to "documents:read", "write-token" to "documents:read documents:write")
    val admits = mapOf<String, suspend (MobileDocumentServiceMcpHttpRequest, MobileDocumentServiceMcpCall) -> MobileDocumentServiceMcpAdmitted>(
        "oauth" to { request, _ ->
            val needed = mobileDocumentServiceMcpScopes(request.header("mcp-method") ?: "", request.mcpName())
            val token = request.bearerToken()
            val held = tokens[token]
            val scope = needed.joinToString(" ")
            when {
                token == null -> MobileDocumentServiceMcpAdmitted.Denied(
                    MobileDocumentServiceMcpDenied.NoValidCredentials(if (needed.isEmpty()) null else scope),
                )
                token == "forbidden-token" -> MobileDocumentServiceMcpAdmitted.Denied(MobileDocumentServiceMcpDenied.Forbidden)
                held == null -> MobileDocumentServiceMcpAdmitted.Denied(MobileDocumentServiceMcpDenied.NoValidCredentials())
                needed.any { it !in held.split(" ") } -> MobileDocumentServiceMcpAdmitted.Denied(
                    MobileDocumentServiceMcpDenied.InsufficientScope(scope),
                )
                else -> MobileDocumentServiceMcpAdmitted.Caller(counted)
            }
        },
        "keys" to { request, _ ->
            val key = request.header("x-api-key")
            val secret = request.header("x-api-secret")
            when {
                key == null || secret == null -> MobileDocumentServiceMcpAdmitted.Denied(
                    MobileDocumentServiceMcpDenied.NoValidCredentials(),
                )
                key == "admin-key" && secret == "admin-secret" -> MobileDocumentServiceMcpAdmitted.Caller(counted)
                else -> MobileDocumentServiceMcpAdmitted.Denied(MobileDocumentServiceMcpDenied.Forbidden)
            }
        },
    )
    val oauth = input.getValue("oauth").jsonObject
    val strings = { name: String -> oauth.getValue(name).jsonArray.map { it.jsonPrimitive.content } }
    val origin = input.getValue("origin").jsonPrimitive.content
    val settings = mapOf(
        "oauth" to MobileDocumentServiceMcpHttpSettings(
            listOf(origin),
            MobileDocumentServiceMcpOAuth(
                strings("authorizationServers"),
                oauth.getValue("resource").jsonPrimitive.content,
                oauth.getValue("resourceMetadataUrl").jsonPrimitive.content,
                strings("scopesSupported"),
            ),
        ),
        "keys" to MobileDocumentServiceMcpHttpSettings(listOf(origin)),
    )
    val answers = input.getValue("rows").jsonArray.map { row ->
        val given = row.jsonObject
        val scheme = given.getValue("scheme").jsonPrimitive.content
        val serve = createMobileDocumentServiceMcpHttpDispatcher(settings.getValue(scheme), admits.getValue(scheme))
        val before = reached
        val response = serve(
            MobileDocumentServiceMcpHttpRequest(
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
        JsonArray(mobileDocumentServiceMcpScopes(asked[0].jsonPrimitive.content, name).map { JsonPrimitive(it) })
    }
    return buildJsonObject {
        put("answers", JsonArray(answers))
        put("scopes", JsonArray(scopes))
    }
}
"#;

const CLIENT_DRIVER: &str = r#"
class Canned(private val replies: Map<Long, String?>) : MobileDocumentServiceMcpTransport {
    val sent = mutableListOf<MobileDocumentServiceMcpMessage>()

    override suspend fun send(message: MobileDocumentServiceMcpMessage): String {
        sent.add(message)
        val id = Json.parseToJsonElement(message.text).jsonObject.getValue("id").jsonPrimitive.long
        return replies[id] ?: throw IllegalStateException("the connection is down")
    }
}

fun describedVersion(result: MobileDocumentServiceGetVersionResult): String = when (result) {
    is MobileDocumentServiceGetVersionResult.Ok -> "ok version=${result.value.version}"
    is MobileDocumentServiceGetVersionResult.Declared -> "declared not-found"
    is MobileDocumentServiceGetVersionResult.Fault -> "fault ${kind(result.fault)}"
}

suspend fun clientReport(input: JsonObject): JsonObject {
    val replies = input.getValue("replies").jsonObject.entries.associate {
        it.key.toLong() to (it.value as? JsonPrimitive)?.takeIf { text -> text.isString }?.content
    }
    val canned = Canned(replies)
    val client = MobileDocumentServiceMcpClient(canned)
    val lines = mutableListOf<String>()
    lines.add(describedVersion(client.getVersion(GetVersionRequest("doc-1"))))
    lines.add(describedVersion(client.getVersion(GetVersionRequest("missing"))))
    lines.add(describedVersion(client.getVersion(GetVersionRequest(""))))
    lines.add(describedVersion(client.getVersion(GetVersionRequest("panic"))))
    lines.add(describedVersion(client.getVersion(GetVersionRequest("doc-1"))))
    lines.add(describedVersion(client.getVersion(GetVersionRequest("doc-1"))))
    lines.add(describedVersion(client.getVersion(GetVersionRequest("doc-1"))))
    lines.add(
        when (val looked = client.lookup(DocumentId("doc-1"))) {
            is MobileDocumentServiceLookupResult.Ok -> "ok version=${looked.value.version}"
            is MobileDocumentServiceLookupResult.Declared -> "declared not-found"
            is MobileDocumentServiceLookupResult.Fault -> "fault ${kind(looked.fault)}"
        },
    )
    lines.add(
        when (val tagged = client.getTagged("doc-1", "\"v6\"")) {
            is MobileDocumentServiceGetTaggedResult.Ok -> "ok version=${tagged.value.value.version} etag=${tagged.value.headerOut0}"
            is MobileDocumentServiceGetTaggedResult.Declared -> "declared not-found"
            is MobileDocumentServiceGetTaggedResult.Fault -> "fault ${kind(tagged.fault)}"
        },
    )
    lines.add(
        when (val tagged = client.getTagged("missing", null)) {
            is MobileDocumentServiceGetTaggedResult.Ok -> "ok"
            is MobileDocumentServiceGetTaggedResult.Declared -> "declared not-found reason=${tagged.error.errorHeaderOut0}"
            is MobileDocumentServiceGetTaggedResult.Fault -> "fault ${kind(tagged.fault)}"
        },
    )
    lines.add(
        when (val renamed = client.renameDocument(RenameDocumentRequest("doc-1", "New"))) {
            is MobileDocumentServiceRenameDocumentResult.Ok -> "ok unit"
            is MobileDocumentServiceRenameDocumentResult.Declared -> "declared"
            is MobileDocumentServiceRenameDocumentResult.Fault -> "fault ${kind(renamed.fault)}"
        },
    )
    lines.add(
        when (val archived = client.archiveDocument(ArchiveDocumentRequest("doc-1", "stale"), "reviewer-7")) {
            is MobileDocumentServiceArchiveDocumentResult.Ok -> "ok unit"
            is MobileDocumentServiceArchiveDocumentResult.Declared -> "declared"
            is MobileDocumentServiceArchiveDocumentResult.Fault -> "fault ${kind(archived.fault)}"
        },
    )
    client.touchDocument(TouchDocumentRequest("doc-1", "reviewer-7"))
    lines.add("ok sent")
    lines.add(
        when (val tagged = client.tagDocument("t", "a", "m", "o", "s")) {
            is MobileDocumentServiceTagDocumentResult.Ok -> "ok version=${tagged.value.version}"
            is MobileDocumentServiceTagDocumentResult.Declared -> "declared"
            is MobileDocumentServiceTagDocumentResult.Fault -> "fault ${kind(tagged.fault)}"
        },
    )
    lines.add(
        when (val labelled = client.labelDocument("l", "s", "n", "m", "c")) {
            is MobileDocumentServiceLabelDocumentResult.Ok -> "ok version=${labelled.value.version}"
            is MobileDocumentServiceLabelDocumentResult.Declared -> "declared"
            is MobileDocumentServiceLabelDocumentResult.Fault -> "fault ${kind(labelled.fault)}"
        },
    )
    for (reason in 0..1) {
        lines.add(
            try {
                client.touchDocument(TouchDocumentRequest("doc-1", "reviewer-7"))
                "ok sent"
            } catch (refused: MobileDocumentServiceMcpRefusal) {
                "refused ${kind(refused.fault)}"
            },
        )
    }
    return buildJsonObject {
        put("lines", JsonArray(lines.map { JsonPrimitive(it) }))
        put("sent", JsonArray(canned.sent.map { JsonPrimitive(it.text) }))
        put("names", JsonArray(canned.sent.map { JsonPrimitive(it.name) }))
        put("methods", JsonArray(canned.sent.map { JsonPrimitive(it.method) }))
        put("versions", JsonArray(canned.sent.map { JsonPrimitive(it.protocolVersion) }))
    }
}

fun main() = runBlocking {
    val input = Json.parseToJsonElement(INPUT).jsonObject
    val report = buildJsonObject {
        put("server", serverReport(input))
        put("client", clientReport(input))
        put("auth", authReport(input))
    }
    println(report.toString())
}
"#;

fn definitions() -> Vec<String> {
    vec![
        version_response_kotlin::kotlin_definition(),
        version_error_kotlin::kotlin_definition(),
        get_version_request_kotlin::kotlin_definition(),
        document_id_kotlin::kotlin_definition(),
        balance_response_kotlin::kotlin_definition(),
        balance_error_kotlin::kotlin_definition(),
        super::mcp_mobile_service::rename_document_request_kotlin::kotlin_definition(),
        super::mcp_mobile_service::touch_document_request_kotlin::kotlin_definition(),
        super::mcp_mobile_service::archive_document_request_kotlin::kotlin_definition(),
        mobile_document_service_schema::mobile_document_service_fault_fields_kotlin::kotlin_definition(),
        mobile_document_service_schema::mobile_document_service_fault_kind_kotlin::kotlin_definition(),
        mobile_billing_service_schema::mobile_billing_service_fault_fields_kotlin::kotlin_definition(),
        mobile_billing_service_schema::mobile_billing_service_fault_kind_kotlin::kotlin_definition(),
        MobileDocumentServiceSchema::kotlin_http_client(),
        MobileDocumentServiceSchema::kotlin_ws_client(),
        MobileDocumentServiceSchema::kotlin_mcp_service(),
        MobileDocumentServiceSchema::kotlin_mcp_client(),
        MobileBillingServiceSchema::kotlin_http_client(),
        MobileBillingServiceSchema::kotlin_ws_client(),
        MobileBillingServiceSchema::kotlin_mcp_service(),
    ]
}

fn http_input(row: &HttpRow) -> Value {
    json!({
        "method": row.method,
        "headers": row.headers.iter().map(|(name, value)| json!([name, value])).collect::<Vec<_>>(),
        "body": row.body,
    })
}

/// What the Rust side hands the Kotlin program: every request, every plain HTTP request and every
/// answer the client's transport gives.
fn input() -> Value {
    let replies: serde_json::Map<String, Value> = client_replies()
        .into_iter()
        .map(|(number, answered)| {
            (
                number.to_string(),
                answered.map_or(Value::Null, Value::String),
            )
        })
        .collect();
    json!({
        "origin": ORIGIN,
        "requests": server_rows().iter().map(|row| row.text.clone()).collect::<Vec<_>>(),
        "http": http_rows().iter().map(http_input).collect::<Vec<_>>(),
        "replies": replies,
        "oauth": {
            "authorizationServers": [AUTHORIZATION_SERVER],
            "resource": RESOURCE,
            "resourceMetadataUrl": METADATA_URL,
            "scopesSupported": [SCOPES_SUPPORTED],
        },
        "rows": auth_rows().iter().map(|row| json!({
            "body": row.body,
            "headers": row.headers.iter().map(|(name, value)| json!([name, value])).collect::<Vec<_>>(),
            "method": row.method,
            "path": row.path,
            "scheme": row.scheme.word(),
        })).collect::<Vec<_>>(),
        "queries": SCOPE_QUERIES.iter().map(|(method, name)| json!([method, name])).collect::<Vec<_>>(),
    })
}

fn module() -> String {
    let literal = kotlin_text(&input().to_string());
    let mut parts = vec![KOTLIN_IMPORTS.to_owned()];
    parts.extend(definitions());
    parts.push(HANDLERS.to_owned());
    parts.push(format!("const val INPUT = \"{literal}\""));
    parts.push(SERVER_DRIVER.to_owned());
    parts.push(AUTH_DRIVER.to_owned());
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

fn texts(report: &Value) -> Vec<Option<String>> {
    report
        .as_array()
        .unwrap()
        .iter()
        .map(|answered| answered.as_str().map(str::to_owned))
        .collect()
}

/// Every answer, held to the official schema and to the Rust dispatcher's.
fn assert_answers(
    which: &str,
    rust: fn(&str) -> Option<String>,
    answers: &[Option<String>],
    rows: &[ServerRow],
) {
    assert_eq!(answers.len(), rows.len());
    for (row, answered) in rows.iter().zip(answers) {
        let rust_said = rust(&row.text);
        assert_eq!(
            answered.is_some(),
            rust_said.is_some(),
            "{which}: {} is answered as Rust answers it",
            row.name
        );
        let (Some(kotlin_text), Some(rust_text)) = (answered, rust_said) else {
            continue;
        };
        let method = serde_json::from_str::<Value>(&row.text)
            .ok()
            .map_or_else(String::new, |asked| {
                asked["method"].as_str().unwrap_or_default().to_owned()
            });
        assert_answer_valid(&method, kotlin_text);
        assert_eq!(
            observed(row.matches, kotlin_text),
            expected(row.matches, &rust_text),
            "{which}: {} is answered as Rust answers it. Kotlin: {kotlin_text} Rust: {rust_text}",
            row.name
        );
    }
}

#[test]
fn the_kotlin_server_answers_the_bytes_the_rust_dispatcher_answers() {
    let Some(report) = driven() else {
        return;
    };
    let server = &report["server"];
    let rows = server_rows();
    assert_answers("alone", alone_answer, &texts(&server["alone"]), &rows);
    assert_answers(
        "endpoint",
        workspace_answer,
        &texts(&server["workspace"]),
        &rows,
    );
    assert_answers("billing", billing_answer, &texts(&server["billing"]), &rows);
    assert_eq!(
        server["shared"], "two services share the MCP name tool:get-version",
        "an endpoint of two services that share a name throws when it is built"
    );
    assert_eq!(
        server["kept"], "built",
        "a tool and a prompt of one name do not collide"
    );
}

#[test]
fn the_kotlin_server_answers_over_plain_http_terms_as_the_rust_dispatcher_does() {
    let Some(report) = driven() else {
        return;
    };
    let rows = http_rows();
    let answered = report["server"]["http"].as_array().unwrap();
    assert_eq!(answered.len(), rows.len());
    for (row, kotlin) in rows.iter().zip(answered) {
        let (status, headers, body) = rust_http(row);
        assert_eq!(
            kotlin["status"].as_u64().unwrap(),
            u64::from(status),
            "{}: {kotlin}",
            row.name
        );
        let kotlin_headers: Vec<(String, String)> = kotlin["headers"]
            .as_array()
            .unwrap()
            .iter()
            .map(|pair| {
                (
                    pair[0].as_str().unwrap().to_owned(),
                    pair[1].as_str().unwrap().to_owned(),
                )
            })
            .collect();
        assert_eq!(kotlin_headers, headers, "{}", row.name);
        let kotlin_body = kotlin["body"].as_str().unwrap();
        let rust_body = if row.without_id && !body.is_empty() {
            let mut parsed: Value = serde_json::from_str(&body).unwrap();
            parsed.as_object_mut().unwrap().remove("id");
            parsed.to_string()
        } else {
            body
        };
        assert_eq!(kotlin_body, rust_body, "{}", row.name);
    }
}

#[test]
fn the_kotlin_client_reads_the_outcomes_the_design_lists() {
    let Some(report) = driven() else {
        return;
    };
    let client = &report["client"];
    let lines: Vec<&str> = client["lines"]
        .as_array()
        .unwrap()
        .iter()
        .map(|line| line.as_str().unwrap())
        .collect();
    assert_eq!(lines, CLIENT_LINES);
    let sent = client["sent"].as_array().unwrap();
    assert_eq!(sent.len(), client_calls().len());
    for (at, text) in sent.iter().enumerate() {
        let number = u64::try_from(at).unwrap() + 1;
        assert_client_request(number, text.as_str().unwrap());
        assert_eq!(client["methods"][at], "tools/call");
        assert_eq!(client["versions"][at], "2026-07-28");
        assert_eq!(client["names"][at], client_calls()[at].tool);
    }
}

#[test]
fn the_kotlin_server_answers_every_authorization_case_as_the_rust_dispatcher_does() {
    let Some(report) = driven() else {
        return;
    };
    let rows = auth_rows();
    let answers = report["auth"]["answers"].as_array().unwrap();
    assert_eq!(answers.len(), rows.len());
    for (row, kotlin) in rows.iter().zip(answers) {
        let (status, headers, body) = rust_auth_answer!(
            mcp_mobile_document,
            MobileDocumentServiceSchema,
            Archive,
            row,
            block_on
        );
        assert_eq!(
            kotlin["status"].as_u64().unwrap(),
            u64::from(status),
            "{}: {kotlin}",
            row.name
        );
        let kotlin_headers: Vec<(String, String)> = kotlin["headers"]
            .as_array()
            .unwrap()
            .iter()
            .map(|pair| {
                (
                    pair[0].as_str().unwrap().to_owned(),
                    pair[1].as_str().unwrap().to_owned(),
                )
            })
            .collect();
        assert_eq!(kotlin_headers, headers, "{}", row.name);
        assert_eq!(kotlin["body"].as_str().unwrap(), body, "{}", row.name);
        if status != 200 {
            assert_eq!(
                kotlin["reached"], 0_i32,
                "{}: no handler is called",
                row.name
            );
        }
    }
}

#[test]
fn the_scopes_function_answers_what_the_rust_registry_answers() {
    let Some(report) = driven() else {
        return;
    };
    let kotlin = report["auth"]["scopes"].as_array().unwrap();
    assert_eq!(kotlin.len(), SCOPE_QUERIES.len());
    for ((method, name), held) in SCOPE_QUERIES.iter().zip(kotlin) {
        let rust = MobileDocumentServiceSchema::mcp_scopes(method, *name);
        assert_eq!(*held, json!(rust), "{method} {name:?}");
    }
}
