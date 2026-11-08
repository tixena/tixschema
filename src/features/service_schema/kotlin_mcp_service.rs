//! The Kotlin `mcp` server: the catalog of one service, the route that answers its `tools/call`,
//! the function that answers one JSON-RPC message for the services of an endpoint, and the
//! dispatchers over stdio-style text and over plain HTTP terms. It answers the bytes the Rust
//! dispatcher answers.

use super::kotlin_http_client::{kotlin_held, read_serializer};
use super::kotlin_ws_client::{HeaderElement, body_type, header_elements, message_serializer};
use super::mcp_scopes::{ReadUri, ScopeTable};
use super::result::result_name;
use crate::features::kotlin::{
    TupleNaming, kotlin_declarations, kotlin_name, kotlin_quoted, kotlin_tuple_naming,
};
use crate::rename_rule::RenameRule;
use crate::service_schema::mcp_tools::{
    answers_key, is_content_list, is_content_pair, wrapped_ident,
};
use crate::service_schema::parse::{
    HttpShape, OperationDef, OperationInputs, OperationOutcome, PathSegment, ServiceDef,
    is_unit_type, option_inner, wire_key, written,
};
use crate::service_schema::skills::written_frontmatter;
use crate::service_schema::support::fault_fields_typescript_name;
use crate::service_schema::transport::mcp::resources::Resourced;
use core::fmt::Write as _;
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::Type;

const CATALOG: &str = r#"const val @P@McpServerName = "@N@"

val @P@McpInstructions: String? = @MCP_INSTRUCTIONS@

val @P@McpCatalog: Map<String, List<String>> = mapOf<String, List<String>>(
  "names" to listOf<String>(@MCP_NAMES@),
  "tools" to listOf<String>(@MCP_TOOLS@),
  "resources" to listOf<String>(@MCP_RESOURCES@),
  "resourceTemplates" to listOf<String>(@MCP_TEMPLATES@),
  "prompts" to listOf<String>(),
  "skills" to listOf<String>(@MCP_SKILLS@),
  "capabilities" to listOf<String>(@MCP_CAPABILITIES@),
)

private val @P@McpSkillFiles: Map<String, String> = mapOf<String, String>(@MCP_SKILL_FILES@)

private val @P@McpWrapped: Map<String, Boolean> = mapOf<String, Boolean>(
  @WRAPPED_MAP@
)"#;

const ENDPOINT: &str = r#"sealed interface @N@McpAnswer {
  data class Answered(val result: JsonObject) : @N@McpAnswer
  data class Refused(val error: JsonObject) : @N@McpAnswer
}

class @N@McpEndpoint(vararg catalogs: Map<String, List<String>>) {
  val catalogs: List<Map<String, List<String>>> = catalogs.toList()

  init {
    val earlier = mutableSetOf<String>()
    for (catalog in this.catalogs) {
      val names = catalog["names"].orEmpty()
      for (name in names) {
        check(name !in earlier) { "two services share the MCP name $name" }
      }
      earlier.addAll(names)
    }
  }
}"#;

const PROTOCOL: &str = r#"private const val @P@McpVersionText = "2026-07-28"
private const val @P@McpProtocolKey = "io.modelcontextprotocol/protocolVersion"
private const val @P@McpCapabilitiesKey = "io.modelcontextprotocol/clientCapabilities"
private const val @P@McpInfoKey = "io.modelcontextprotocol/serverInfo"
private const val @P@McpListedForMs = 300000
private val @P@McpMethods: Set<String> = setOf(
  "completion/complete", "resources/list", "resources/read", "resources/templates/list",
  "server/discover", "skills/get", "skills/list", "tools/call", "tools/list",
)

private fun @P@McpServeText(element: JsonElement): String {
  val into = StringBuilder()
  @P@McpServeWrite(element, into)
  return into.toString()
}

private fun @P@McpServeWrite(element: JsonElement, into: StringBuilder) {
  when (element) {
    is JsonObject -> {
      into.append('{')
      var first = true
      for (key in element.keys.sorted()) {
        if (!first) into.append(',')
        first = false
        @P@McpServeQuote(key, into)
        into.append(':')
        @P@McpServeWrite(element.getValue(key), into)
      }
      into.append('}')
    }
    is JsonArray -> {
      into.append('[')
      var first = true
      for (item in element) {
        if (!first) into.append(',')
        first = false
        @P@McpServeWrite(item, into)
      }
      into.append(']')
    }
    is JsonPrimitive -> if (element.isString) @P@McpServeQuote(element.content, into) else into.append(element.content)
  }
}

private fun @P@McpServeQuote(text: String, into: StringBuilder) {
  into.append('"')
  for (letter in text) {
    when {
      letter == '"' -> into.append("\\\"")
      letter == '\\' -> into.append("\\\\")
      letter == '\n' -> into.append("\\n")
      letter == '\r' -> into.append("\\r")
      letter == '\t' -> into.append("\\t")
      letter == '\b' -> into.append("\\b")
      letter == '\u000c' -> into.append("\\f")
      letter < ' ' -> into.append("\\u").append(letter.code.toString(16).padStart(4, '0'))
      else -> into.append(letter)
    }
  }
  into.append('"')
}

private fun @P@McpFaultObject(kind: String, operation: String, detail: String, field: String? = null): JsonObject =
  buildJsonObject {
    put("detail", detail)
    if (field != null) put("field", field)
    put("kind", kind)
    put("operation", operation)
  }

private fun @P@McpDone(structured: JsonElement?, failure: String?): @N@McpAnswer {
  val text = failure ?: structured?.let { @P@McpServeText(it) }
  return @N@McpAnswer.Answered(
    buildJsonObject {
      put("resultType", "complete")
      putJsonArray("content") {
        if (text != null) add(buildJsonObject { put("type", "text"); put("text", text) })
      }
      put("isError", failure != null)
      if (failure == null && structured != null) put("structuredContent", structured)
    },
  )
}

private fun @P@McpFaulted(fault: JsonElement): @N@McpAnswer =
  @P@McpDone(null, @P@McpServeText(buildJsonObject { put("fault", fault); put("isServiceFault", true) }))

private fun @P@McpInternal(tool: String, detail: String): @N@McpAnswer =
  @N@McpAnswer.Refused(
    buildJsonObject {
      put("code", -32603)
      put(
        "data",
        buildJsonObject {
          put("fault", @P@McpFaultObject("handler-panic", tool, detail))
          put("isServiceFault", true)
        },
      )
      put("message", "$tool: $detail")
    },
  )

private fun @P@McpPut(members: MutableMap<String, JsonElement>, name: String, value: JsonElement) {
  if (value !is JsonNull) members[name] = value
}

private fun @P@McpErrorObject(code: Int, message: String, data: JsonElement?): JsonObject =
  buildJsonObject {
    put("code", code)
    if (data != null) put("data", data)
    put("message", message)
  }

private fun @P@McpFailed(id: JsonElement?, error: JsonObject): JsonObject =
  buildJsonObject {
    put("error", error)
    if (id != null) put("id", id)
    put("jsonrpc", "2.0")
  }

private fun @P@McpInvalid(detail: String): JsonObject =
  @P@McpFailed(null, @P@McpErrorObject(-32600, "Invalid request: $detail", null))

private fun @P@McpIsNumber(primitive: JsonPrimitive): Boolean =
  !primitive.isString && primitive.content != "null" && primitive.content != "true" && primitive.content != "false"

private fun @P@McpReadRequest(text: String): Pair<JsonObject?, JsonObject?> {
  val parsed = try {
    Json.parseToJsonElement(text)
  } catch (rejected: Exception) {
    return null to @P@McpInvalid(rejected.message ?: rejected.toString())
  }
  if (parsed !is JsonObject) return null to @P@McpInvalid("not a JSON object")
  val version = parsed["jsonrpc"]
  if (version !is JsonPrimitive || !version.isString || version.content != "2.0") {
    return null to @P@McpInvalid("`jsonrpc` is not \"2.0\"")
  }
  val method = parsed["method"]
  if (method !is JsonPrimitive || !method.isString) return null to @P@McpInvalid("no `method`")
  val id = parsed["id"]
  if (id != null && !(id is JsonPrimitive && (id.isString || @P@McpIsNumber(id)))) {
    return null to @P@McpInvalid("`id` is neither a string nor a number")
  }
  return parsed to null
}

private fun @P@McpParams(request: JsonObject): JsonObject = (request["params"] as? JsonObject) ?: JsonObject(emptyMap())

private fun @P@McpChecked(request: JsonObject): JsonObject? {
  val id = request["id"] ?: JsonNull
  val meta = @P@McpParams(request)["_meta"] as? JsonObject
  val missing = listOf(@P@McpProtocolKey, @P@McpCapabilitiesKey).firstOrNull { meta == null || it !in meta }
  if (missing != null) {
    return @P@McpFailed(id, @P@McpErrorObject(-32602, "Missing required _meta key: $missing", null))
  }
  val requested = meta?.get(@P@McpProtocolKey)
  if (!(requested is JsonPrimitive && requested.isString && requested.content == @P@McpVersionText)) {
    return @P@McpFailed(
      id,
      @P@McpErrorObject(
        -32022,
        "Unsupported protocol version",
        buildJsonObject {
          put("requested", requested ?: JsonNull)
          put("supported", buildJsonArray { add(@P@McpVersionText) })
        },
      ),
    )
  }
  return null
}

private fun @P@McpWithInfo(result: JsonObject, serverName: String): JsonObject {
  val members = result.toMutableMap()
  if ("resultType" !in members) members["resultType"] = JsonPrimitive("complete")
  val meta = (members["_meta"] as? JsonObject)?.toMutableMap() ?: mutableMapOf()
  meta[@P@McpInfoKey] = buildJsonObject {
    put("name", serverName)
    put("version", "@MCP_VERSION@")
  }
  members["_meta"] = JsonObject(meta)
  return JsonObject(members)
}

private fun @P@McpDiscovered(instructions: String?, endpoint: @N@McpEndpoint): JsonObject {
  val capabilities = mutableMapOf<String, JsonElement>()
  val extensions = mutableMapOf<String, JsonElement>()
  for (catalog in endpoint.catalogs) {
    for (text in catalog["capabilities"].orEmpty()) {
      val offered = Json.parseToJsonElement(text) as? JsonObject ?: continue
      for ((key, value) in offered) {
        if (key == "extensions" && value is JsonObject) {
          for ((extension, held) in value) extensions.getOrPut(extension) { held }
        } else {
          capabilities.getOrPut(key) { value }
        }
      }
    }
  }
  if (extensions.isNotEmpty()) capabilities["extensions"] = JsonObject(extensions)
  return buildJsonObject {
    put("cacheScope", "public")
    put("capabilities", JsonObject(capabilities))
    if (instructions != null) put("instructions", instructions)
    put("resultType", "complete")
    putJsonArray("supportedVersions") { add(@P@McpVersionText) }
    put("ttlMs", @P@McpListedForMs)
  }
}

private fun @P@McpListedItems(endpoint: @N@McpEndpoint, key: String): JsonObject =
  buildJsonObject {
    put("cacheScope", "public")
    put("resultType", "complete")
    putJsonArray(key) {
      for (catalog in endpoint.catalogs) {
        for (text in catalog[key].orEmpty()) add(Json.parseToJsonElement(text))
      }
    }
    put("ttlMs", @P@McpListedForMs)
  }

private fun @P@McpListedSkills(endpoint: @N@McpEndpoint): JsonObject {
  val skills = endpoint.catalogs.flatMap { catalog -> catalog["skills"].orEmpty().map { Json.parseToJsonElement(it) } }
  val written = skills.any { ((it as? JsonObject)?.get("resources") as? JsonPrimitive)?.content == "dynamic" }
  return buildJsonObject {
    put("cacheScope", if (written) "private" else "public")
    put("resultType", "complete")
    put("skills", JsonArray(skills))
    put("ttlMs", if (written) 0 else @P@McpListedForMs)
  }
}

private fun @P@McpNamed(params: JsonObject, method: String): String? {
  val written = when (method) {
    "tools/call" -> params["name"]
    "resources/read" -> params["uri"]
    "skills/get" -> params["name"]?.takeIf { it is JsonPrimitive && it.isString } ?: params["uri"]
    else -> if (((params["argument"] as? JsonObject)?.get("name") as? JsonPrimitive)?.isString == true) {
      (params["ref"] as? JsonObject)?.get("uri")
    } else {
      null
    }
  }
  return (written as? JsonPrimitive)?.takeIf { it.isString }?.content
}

private class @P@McpStart(val part: Int, val own: String?, val first: Boolean)

private fun @P@McpListingStart(request: JsonObject, services: Int): @P@McpStart? {
  val cursor = @P@McpParams(request)["cursor"]
  if (cursor == null || cursor is JsonNull) return @P@McpStart(0, null, true)
  if (cursor !is JsonPrimitive || !cursor.isString) return null
  val written = cursor.content
  val colon = written.indexOf(':')
  if (colon < 0) return null
  val digits = written.substring(0, colon).removePrefix("+")
  if (digits.isEmpty() || !digits.all { it in '0'..'9' }) return null
  val part = digits.toIntOrNull() ?: return null
  if (part >= services) return null
  val own = written.substring(colon + 1)
  return @P@McpStart(part, own.ifEmpty { null }, false)
}

private fun @P@McpListedResources(first: Boolean, endpoint: @N@McpEndpoint, page: Pair<Int, @N@McpAnswer>?): @N@McpAnswer {
  val resources = mutableListOf<JsonElement>()
  if (first) {
    for (catalog in endpoint.catalogs) {
      for (text in catalog["resources"].orEmpty()) resources.add(Json.parseToJsonElement(text))
    }
  }
  val members = mutableMapOf<String, JsonElement>(
    "cacheScope" to JsonPrimitive("private"),
    "resultType" to JsonPrimitive("complete"),
    "ttlMs" to JsonPrimitive(0),
  )
  if (page != null) {
    val (part, answer) = page
    if (answer is @N@McpAnswer.Refused) return answer
    val listed = (answer as @N@McpAnswer.Answered).result
    (listed["resources"] as? JsonArray)?.let { resources.addAll(it) }
    for (key in listOf("ttlMs", "cacheScope")) listed[key]?.let { members[key] = it }
    val own = (listed["nextCursor"] as? JsonPrimitive)?.takeIf { it.isString && it.content.isNotEmpty() }?.content
    val next = when {
      own != null -> "$part:$own"
      part + 1 < endpoint.catalogs.size -> "${part + 1}:"
      else -> null
    }
    if (next != null) members["nextCursor"] = JsonPrimitive(next)
  }
  members["resources"] = JsonArray(resources)
  return @N@McpAnswer.Answered(JsonObject(members))
}"#;

const WRITER: &str = r#"/** Where the transport writes a message beyond the answer: one line on stdio, one event on HTTP. A write that throws means the client has gone. */
typealias @N@McpWriter = suspend (message: String) -> Unit

/**
 * The request in flight, handed to `admit` and to the function that builds the handlers, so that a
 * handler reaches it. [token] is the request's `progressToken`, and [writer] takes what it writes
 * beyond its answer.
 */
class @N@McpCall(private val token: JsonPrimitive? = null, private val writer: @N@McpWriter? = null) {
  @Volatile
  private var gone = false

  /** Whether the client cancelled this request, or went away. */
  val cancelled: Boolean
    get() = gone

  /** Writes `notifications/progress` where the request carries a `progressToken`, and does nothing otherwise. A write that fails cancels the call. */
  suspend fun progress(done: Double, total: Double? = null, message: String? = null) {
    val sink = writer
    val reported = token
    if (sink == null || reported == null || gone) return
    try {
      sink(
        @P@McpServeText(
          buildJsonObject {
            put("jsonrpc", "2.0")
            put("method", "notifications/progress")
            put(
              "params",
              buildJsonObject {
                put("progress", done)
                put("progressToken", reported)
                if (total != null) put("total", total)
                if (message != null) put("message", message)
              },
            )
          },
        ),
      )
    } catch (stopped: CancellationException) {
      throw stopped
    } catch (failed: Exception) {
      gone = true
    }
  }

  internal fun cancel() {
    gone = true
  }
}

/** The requests a stdio host has in flight, so that `notifications/cancelled` reaches the right one. */
class @N@McpInFlight {
  private val calls = java.util.concurrent.ConcurrentHashMap<String, @N@McpCall>()

  internal fun cancel(id: JsonElement?) {
    if (id != null) calls[id.toString()]?.cancel()
  }

  internal fun enter(id: JsonElement, call: @N@McpCall): () -> Unit {
    val key = id.toString()
    calls[key] = call
    return { calls.remove(key, call) }
  }
}

/** What [@P@McpServe] writes to, and the requests it has in flight. */
class @N@McpHost(val writer: @N@McpWriter, val inFlight: @N@McpInFlight)

private fun @P@McpToken(message: JsonObject): JsonPrimitive? {
  val token = (@P@McpParams(message)["_meta"] as? JsonObject)?.get("progressToken") as? JsonPrimitive
  return token?.takeIf { it.isString || @P@McpIsNumber(it) }
}

private fun @P@McpBodyToken(request: @N@McpHttpRequest): JsonPrimitive? =
  try {
    (Json.parseToJsonElement(request.body.decodeToString()) as? JsonObject)?.let { @P@McpToken(it) }
  } catch (rejected: Exception) {
    null
  }"#;

const SERVING: &str = r#"private sealed interface @P@McpBegun {
  class Answered(val text: String?) : @P@McpBegun
  class Notified(val notice: JsonObject) : @P@McpBegun
  class Routed(val request: JsonObject) : @P@McpBegun
}

private fun @P@McpResponseText(id: JsonElement, answered: @N@McpAnswer, serverName: String): String =
  @P@McpServeText(
    when (answered) {
      is @N@McpAnswer.Answered -> buildJsonObject {
        put("id", id)
        put("jsonrpc", "2.0")
        put("result", @P@McpWithInfo(answered.result, serverName))
      }
      is @N@McpAnswer.Refused -> @P@McpFailed(id, answered.error)
    },
  )

private fun @P@McpBegin(
  text: String,
  instructions: String?,
  endpoint: @N@McpEndpoint,
  serverName: String,
): @P@McpBegun {
  val (request, refusal) = @P@McpReadRequest(text)
  if (request == null) return @P@McpBegun.Answered(@P@McpServeText(refusal ?: JsonNull))
  val id = request["id"] ?: return @P@McpBegun.Notified(request)
  val unchecked = @P@McpChecked(request)
  if (unchecked != null) return @P@McpBegun.Answered(@P@McpServeText(unchecked))
  val method = (request["method"] as JsonPrimitive).content
  val answered: @N@McpAnswer = when (method) {
    "server/discover" -> @N@McpAnswer.Answered(@P@McpDiscovered(instructions, endpoint))
    "tools/list" -> @N@McpAnswer.Answered(@P@McpListedItems(endpoint, "tools"))
    "resources/templates/list" -> @N@McpAnswer.Answered(@P@McpListedItems(endpoint, "resourceTemplates"))
    "skills/list" -> @N@McpAnswer.Answered(@P@McpListedSkills(endpoint))
    "resources/list" -> return @P@McpBegun.Routed(request)
    "tools/call", "resources/read", "completion/complete", "skills/get" -> {
      if (@P@McpNamed(@P@McpParams(request), method) != null) return @P@McpBegun.Routed(request)
      @N@McpAnswer.Refused(@P@McpErrorObject(-32602, "Invalid params: $method names nothing to answer", null))
    }
    else -> @N@McpAnswer.Refused(@P@McpErrorObject(-32601, "Method not found: $method", null))
  }
  return @P@McpBegun.Answered(@P@McpResponseText(id, answered, serverName))
}

private fun @P@McpRoutedText(request: JsonObject, answered: @N@McpAnswer?, serverName: String): String {
  val method = (request["method"] as JsonPrimitive).content
  val name = @P@McpNamed(@P@McpParams(request), method).orEmpty()
  val unknown = when (method) {
    "tools/call" -> "Unknown tool"
    "resources/read" -> "Resource not found"
    "skills/get" -> "Skill not found"
    else -> "Unknown completion"
  }
  val resolved = answered ?: @N@McpAnswer.Refused(
    @P@McpErrorObject(
      -32602,
      "$unknown: $name",
      buildJsonObject {
        put("fault", @P@McpFaultObject("unknown-operation", name, "the service answers to no operation by that name"))
        put("isServiceFault", true)
      },
    ),
  )
  return @P@McpResponseText(request["id"] ?: JsonNull, resolved, serverName)
}

/** Asks the services of the endpoint for what the routed [request] wants, and answers its text. */
private suspend fun @P@McpAsk(
  request: JsonObject,
  endpoint: @N@McpEndpoint,
  serverName: String,
  route: suspend (part: Int, method: String, params: JsonObject) -> @N@McpAnswer?,
): String {
  val method = (request["method"] as JsonPrimitive).content
  if (method == "resources/list") {
    val start = @P@McpListingStart(request, endpoint.catalogs.size)
    val listed: @N@McpAnswer = if (start == null) {
      @N@McpAnswer.Refused(@P@McpErrorObject(-32602, "Invalid cursor", null))
    } else {
      var page: Pair<Int, @N@McpAnswer>? = null
      for (part in start.part until endpoint.catalogs.size) {
        val asked = if (part == start.part && start.own != null) {
          buildJsonObject { put("cursor", start.own) }
        } else {
          JsonObject(emptyMap())
        }
        val answered = route(part, method, asked)
        if (answered != null) {
          page = part to answered
          break
        }
      }
      @P@McpListedResources(start.first, endpoint, page)
    }
    return @P@McpResponseText(request["id"] ?: JsonNull, listed, serverName)
  }
  var answered: @N@McpAnswer? = null
  for (part in endpoint.catalogs.indices) {
    answered = route(part, method, @P@McpParams(request))
    if (answered != null) break
  }
  return @P@McpRoutedText(request, answered, serverName)
}

/**
 * One JSON-RPC message in, one out, for the services of [endpoint]. [route] answers for the
 * service at that place in the endpoint, and receives the [@N@McpCall] of the request in flight
 * to build its handlers from. A notification answers `null`, `notifications/cancelled` marks the
 * request it names in [host], and the answer of a cancelled request is dropped.
 */
suspend fun @P@McpServe(
  text: String,
  serverName: String,
  instructions: String?,
  endpoint: @N@McpEndpoint,
  host: @N@McpHost,
  route: suspend (call: @N@McpCall, part: Int, method: String, params: JsonObject) -> @N@McpAnswer?,
): String? {
  val request = when (val begun = @P@McpBegin(text, instructions, endpoint, serverName)) {
    is @P@McpBegun.Answered -> return begun.text
    is @P@McpBegun.Notified -> {
      if ((begun.notice["method"] as JsonPrimitive).content == "notifications/cancelled") {
        host.inFlight.cancel(@P@McpParams(begun.notice)["requestId"])
      }
      return null
    }
    is @P@McpBegun.Routed -> begun.request
  }
  val call = @N@McpCall(@P@McpToken(request), host.writer)
  val leave = host.inFlight.enter(request["id"] ?: JsonNull, call)
  try {
    val answered = @P@McpAsk(request, endpoint, serverName) { part, method, params -> route(call, part, method, params) }
    return if (call.cancelled) null else answered
  } finally {
    leave()
  }
}"#;

const RECEIVING: &str = r#"private class @P@McpRefused(val answer: @N@McpAnswer) : Exception()

private fun @P@McpRefuse(kind: String, tool: String, detail: String, field: String? = null): Nothing =
  throw @P@McpRefused(@P@McpFaulted(@P@McpFaultObject(kind, tool, detail, field)))

private fun @P@McpArguments(tool: String, params: JsonObject): MutableMap<String, JsonElement> {
  val given = params["arguments"]
  return when (given) {
    is JsonObject -> given.toMutableMap()
    null, is JsonNull -> mutableMapOf()
    else -> @P@McpRefuse("undeserializable-payload", tool, "the arguments are not a JSON object")
  }
}

private fun @P@McpSource(tool: String, arguments: MutableMap<String, JsonElement>, wrapped: Boolean): JsonElement =
  if (wrapped) {
    arguments.remove("value") ?: @P@McpRefuse("undeserializable-payload", tool, "missing field `value`")
  } else {
    JsonObject(arguments)
  }

private fun @P@McpNamedField(detail: String): String? {
  if (!detail.startsWith("'")) return null
  val closing = detail.indexOf('\'', 1)
  return if (closing < 0) null else detail.substring(1, closing)
}

private fun <T> @P@McpReceive(
  tool: String,
  serializer: DeserializationStrategy<T>,
  source: JsonElement,
  refuse: (kind: String, detail: String, field: String?) -> Nothing = { kind, detail, field ->
    @P@McpRefuse(kind, tool, detail, field)
  },
): T {
  try {
    return Json.decodeFromJsonElement(serializer, source)
  } catch (refused: IllegalArgumentException) {
    val unjudged = (serializer as? Function0<*>)?.invoke() as? DeserializationStrategy<*>
    val readable = unjudged != null && runCatching { Json.decodeFromJsonElement(unjudged, source) }.isSuccess
    val detail = refused.message ?: refused.toString()
    if (readable) refuse("failed-validation", detail, @P@McpNamedField(detail))
    refuse("undeserializable-payload", detail, null)
  }
}

private fun <T> @P@McpHeader(tool: String, name: String, serializer: DeserializationStrategy<T>, raw: JsonElement?): T =
  try {
    Json.decodeFromJsonElement(serializer, raw ?: JsonNull)
  } catch (rejected: IllegalArgumentException) {
    @P@McpRefuse("failed-validation", tool, rejected.message ?: rejected.toString(), name)
  }

private fun <T : Any> @P@McpOptionalHeader(
  tool: String,
  name: String,
  serializer: DeserializationStrategy<T>,
  raw: JsonElement?,
): T? = if (raw == null || raw is JsonNull) null else @P@McpHeader(tool, name, serializer, raw)"#;

const INTERVIEW: &str = r#"private fun @P@McpPlain(text: String): String = text.replace(Regex("[-_ ]"), "").lowercase()

/** Whether the client declared the elicitation [mode]: an empty `elicitation` is a form. */
private fun @P@McpElicits(params: JsonObject, mode: String): Boolean {
  val meta = params["_meta"] as? JsonObject
  val held = (meta?.get(@P@McpCapabilitiesKey) as? JsonObject)?.get("elicitation") as? JsonObject ?: return false
  if (mode == "url") return held["url"] is JsonObject
  return held["form"] is JsonObject || "url" !in held
}

/** Whether the user declined or dismissed the request keyed [key]. */
private fun @P@McpDismissed(params: JsonObject, key: String): Boolean {
  val response = (params["inputResponses"] as? JsonObject)?.get(key) as? JsonObject
  val action = (response?.get("action") as? JsonPrimitive)?.content
  return action == "decline" || action == "cancel"
}

/** The result that asks the client for the elicitation [request], keyed [key]. */
private fun @P@McpInputRequired(key: String, request: JsonObject): @N@McpAnswer =
  @N@McpAnswer.Answered(
    buildJsonObject {
      put(
        "inputRequests",
        buildJsonObject {
          put(key, buildJsonObject { put("method", "elicitation/create"); put("params", request) })
        },
      )
      put("resultType", "input_required")
    },
  )

/** The result that asks the user where [error] is the arm that asks and the client can be asked, `null` otherwise. */
private fun @P@McpAsking(
  arm: String,
  answers: String?,
  form: String?,
  doc: String,
  error: JsonElement,
  params: JsonObject,
  declined: Boolean,
): @N@McpAnswer? {
  val body = error as? JsonObject ?: return null
  val code = (body["errorCode"] as? JsonPrimitive)?.takeIf { it.isString }?.content ?: return null
  if (@P@McpPlain(code) != @P@McpPlain(arm)) return null
  val message = (body["message"] as? JsonPrimitive)?.takeIf { it.isString }?.content
  if (answers != null) {
    if (declined || !@P@McpElicits(params, "form")) return null
    return @P@McpInputRequired(
      answers,
      buildJsonObject {
        put("message", message ?: doc)
        put("mode", "form")
        put("requestedSchema", Json.parseToJsonElement(form ?: "{}"))
      },
    )
  }
  val url = (body["url"] as? JsonPrimitive)?.takeIf { it.isString }?.content ?: return null
  if (message == null) return null
  if (@P@McpDismissed(params, code) || !@P@McpElicits(params, "url")) return null
  return @P@McpInputRequired(code, buildJsonObject { put("message", message); put("mode", "url"); put("url", url) })
}"#;

const CONTENT: &str = r#"/** The result of a tool that answers blocks. */
private fun @P@McpBlocks(content: List<JsonElement>): @N@McpAnswer =
  @N@McpAnswer.Answered(
    buildJsonObject {
      put("resultType", "complete")
      put("content", JsonArray(content))
      put("isError", false)
    },
  )"#;

const MEDIA: &str = r#"/** The image or audio block of [bytes], where [contentType] says they are one. */
private fun @P@McpMedia(bytes: List<UByte>, contentType: String): @N@McpAnswer? {
  val kind = if (contentType.startsWith("image/")) "image" else if (contentType.startsWith("audio/")) "audio" else return null
  val data = java.util.Base64.getEncoder().encodeToString(ByteArray(bytes.size) { bytes[it].toByte() })
  return @P@McpBlocks(listOf(buildJsonObject { put("data", data); put("mimeType", contentType); put("type", kind) }))
}"#;

const ROUTE_HEAD: &str = "fun create@N@McpRoute(handlers: @N@Handlers): suspend (method: String, params: JsonObject) -> @N@McpAnswer? =
  { mcpMethod, mcpParams -> @P@McpCall(handlers, mcpMethod, mcpParams) }

private suspend fun @P@McpCall(handlers: @N@Handlers, mcpMethod: String, mcpParams: JsonObject): @N@McpAnswer? {
  try {
    return when (mcpMethod) {
@DISPATCH@      else -> null
    }
  } catch (mcpRefused: @P@McpRefused) {
    return mcpRefused.answer
  }
}
";

const TOOL_HEAD: &str = r#"
private suspend fun @P@McpTool(handlers: @N@Handlers, mcpParams: JsonObject): @N@McpAnswer? {
  val mcpNamed = mcpParams["name"]
  if (mcpNamed !is JsonPrimitive || !mcpNamed.isString) return null
  return when (mcpNamed.content) {
"#;

const TOOL_TAIL: &str = "    else -> null
  }
}";

const RESOURCE_SUPPORT: &str = r#"

private fun @P@McpCaptured(segments: List<Pair<String, Boolean>>, uri: String): List<String>? {
  var rest = uri
  val captured = mutableListOf<String>()
  for ((text, placeholder) in segments) {
    if (placeholder) {
      val slash = rest.indexOf('/')
      val end = if (slash < 0) rest.length else slash
      if (end == 0) return null
      captured.add(rest.substring(0, end))
      rest = rest.substring(end)
    } else {
      if (!rest.startsWith(text)) return null
      rest = rest.substring(text.length)
    }
  }
  return if (rest.isEmpty()) captured else null
}

private fun @P@McpUnread(operation: String, kind: String, detail: String, field: String?): Nothing =
  throw @P@McpRefused(
    @N@McpAnswer.Refused(
      @P@McpErrorObject(
        -32602,
        "Invalid params",
        buildJsonObject {
          put("fault", @P@McpFaultObject(kind, operation, detail, field))
          put("isServiceFault", true)
        },
      ),
    ),
  )

private fun @P@McpDeclined(error: JsonElement, refusal: String): @N@McpAnswer =
  @N@McpAnswer.Refused(@P@McpErrorObject(-32602, refusal, error))

private fun @P@McpAnswerOf(scope: String, ttl: Int, key: String, items: JsonElement): @N@McpAnswer =
  @N@McpAnswer.Answered(
    buildJsonObject {
      put("cacheScope", scope)
      put(key, items)
      put("resultType", "complete")
      put("ttlMs", ttl)
    },
  )

private fun @P@McpFaultedRequest(fault: JsonElement): @N@McpAnswer =
  @N@McpAnswer.Refused(
    @P@McpErrorObject(
      -32602,
      "Invalid params",
      buildJsonObject {
        put("fault", fault)
        put("isServiceFault", true)
      },
    ),
  )"#;

const DISPATCHERS: &str = "/**
 * Answers the requests of this service alone, one text in and one out. [handlers] builds the
 * handlers from the request in flight, [writer] takes what the request writes beyond its answer,
 * and [inFlight] lists the requests `notifications/cancelled` can reach.
 */
fun create@N@McpDispatcher(
  handlers: (@N@McpCall) -> @N@Handlers,
): suspend (text: String, writer: @N@McpWriter, inFlight: @N@McpInFlight) -> String? {
  val endpoint = @N@McpEndpoint(@P@McpCatalog)
  return { text, writer, inFlight ->
    @P@McpServe(text, @P@McpServerName, @P@McpInstructions, endpoint, @N@McpHost(writer, inFlight)) { call, _, method, params ->
      create@N@McpRoute(handlers(call))(method, params)
    }
  }
}";

const HTTP: &str = r#"/** An HTTP request in plain terms. `path` is the request's path, with or without its query. */
data class @N@McpHttpRequest(val method: String, val path: String, val headers: List<Pair<String, String>>, val body: ByteArray) {
  /** The value of the header called [name], whatever its case. */
  fun header(name: String): String? = headers.firstOrNull { it.first.equals(name, ignoreCase = true) }?.second

  /** The token of an `Authorization: Bearer` header, whatever the case of the scheme. */
  fun bearerToken(): String? {
    val value = header("authorization") ?: return null
    val at = value.indexOf(' ')
    if (at < 0) return null
    val token = value.substring(at + 1).trim()
    return if (value.substring(0, at).equals("bearer", ignoreCase = true) && token.isNotEmpty()) token else null
  }

  /** The `Mcp-Name` header, decoded from `=?base64?...?=` where it is written so. */
  fun mcpName(): String? = header("mcp-name")?.let { @P@McpDecodedHeader(it) }
}

data class @N@McpHttpResponse(val status: Int, val headers: List<Pair<String, String>>, val body: ByteArray)

/** The protected-resource metadata of a server that authorizes by OAuth. */
data class @N@McpOAuth(
  val authorizationServers: List<String>,
  val resource: String,
  val resourceMetadataUrl: String,
  val scopesSupported: List<String>,
)

/** What an HTTP server of this endpoint is configured with. Without [oauth] it serves no metadata document and writes no challenge. */
data class @N@McpHttpSettings(val allowedOrigins: List<String>, val oauth: @N@McpOAuth? = null)

/** Why `admit` refused a request. */
sealed interface @N@McpDenied {
  /** `401`: no credentials, or credentials that do not hold. [scope] names what the called operation needs. */
  data class NoValidCredentials(val scope: String? = null) : @N@McpDenied

  /** `403` with a challenge: the credentials hold, but not the scopes [scope] names. */
  data class InsufficientScope(val scope: String) : @N@McpDenied

  /** `403` with no challenge. */
  object Forbidden : @N@McpDenied
}

/** What `admit` answers: the handlers built for the caller, or why the request is refused. */
sealed interface @N@McpAdmitted {
  data class Caller(val handlers: @N@Handlers) : @N@McpAdmitted
  data class Denied(val denied: @N@McpDenied) : @N@McpAdmitted
}

/** The scopes a caller needs for the request [method] and [name] name, none for any other. */
fun @P@McpScopes(method: String, name: String?): List<String> {
@SCOPES@  return emptyList()
}

private fun @P@McpWithoutBody(status: Int, headers: List<Pair<String, String>> = emptyList()): @N@McpHttpResponse =
  @N@McpHttpResponse(status, headers, ByteArray(0))

private fun @P@McpJsonResponse(status: Int, body: JsonElement): @N@McpHttpResponse =
  @N@McpHttpResponse(
    status,
    listOf("Content-Type" to "application/json"),
    @P@McpServeText(body).encodeToByteArray(),
  )

private fun @P@McpDecodedHeader(value: String): String? {
  if (!value.startsWith("=?base64?") || !value.endsWith("?=")) return value
  return try {
    val decoded = java.util.Base64.getDecoder().decode(value.removePrefix("=?base64?").removeSuffix("?="))
    Charsets.UTF_8.newDecoder().decode(java.nio.ByteBuffer.wrap(decoded)).toString()
  } catch (rejected: Exception) {
    null
  }
}

private fun @P@McpHeaderMismatch(request: @N@McpHttpRequest, message: JsonObject): @N@McpHttpResponse? {
  val method = (message["method"] as JsonPrimitive).content
  val params = @P@McpParams(message)
  val named: JsonElement? = when (method) {
    "tools/call", "prompts/get" -> params["name"] ?: JsonNull
    "resources/read" -> params["uri"] ?: JsonNull
    else -> null
  }
  val mismatched = { header: String ->
    @P@McpJsonResponse(
      400,
      @P@McpFailed(message["id"], @P@McpErrorObject(-32020, "Header mismatch: $header", null)),
    )
  }
  val bodyVersion = (params["_meta"] as? JsonObject)?.get(@P@McpProtocolKey) ?: JsonNull
  val version = request.header("mcp-protocol-version")
  if (version == null || (bodyVersion !is JsonNull && (bodyVersion as? JsonPrimitive)?.takeIf { it.isString }?.content != version)) {
    return mismatched("MCP-Protocol-Version")
  }
  if (request.header("mcp-method") != method) return mismatched("Mcp-Method")
  if (named != null) {
    val header = request.header("mcp-name")?.let { @P@McpDecodedHeader(it) }
    val bodyName = (named as? JsonPrimitive)?.takeIf { it.isString }?.content
    if (header == null || header != bodyName) return mismatched("Mcp-Name")
  }
  return null
}

private fun @P@McpUrlPath(url: String): String {
  val after = url.substringAfter("://", url)
  val at = after.indexOf('/')
  val fromPath = if (at < 0) "/" else after.substring(at)
  return fromPath.split('?', '#').first()
}

private fun @P@McpDenial(denied: @N@McpDenied, settings: @N@McpHttpSettings, request: @N@McpHttpRequest): @N@McpHttpResponse {
  val oauth = settings.oauth
  val (status, challenge) = when (denied) {
    is @N@McpDenied.NoValidCredentials -> 401 to oauth?.let {
      val parts = mutableListOf("resource_metadata=\"${it.resourceMetadataUrl}\"")
      denied.scope?.let { scope -> parts.add("scope=\"$scope\"") }
      if (request.bearerToken() != null) parts.add("error=\"invalid_token\"")
      "Bearer ${parts.joinToString(", ")}"
    }
    is @N@McpDenied.InsufficientScope -> 403 to oauth?.let {
      "Bearer error=\"insufficient_scope\", scope=\"${denied.scope}\", resource_metadata=\"${it.resourceMetadataUrl}\""
    }
    is @N@McpDenied.Forbidden -> 403 to null
  }
  return @P@McpWithoutBody(status, if (challenge == null) emptyList() else listOf("WWW-Authenticate" to challenge))
}

private fun @P@McpMetadata(oauth: @N@McpOAuth): @N@McpHttpResponse =
  @P@McpJsonResponse(
    200,
    buildJsonObject {
      put("authorization_servers", JsonArray(oauth.authorizationServers.map { JsonPrimitive(it) }))
      put("bearer_methods_supported", JsonArray(listOf(JsonPrimitive("header"))))
      put("resource", oauth.resource)
      if (oauth.scopesSupported.isNotEmpty()) {
        put("scopes_supported", JsonArray(oauth.scopesSupported.map { JsonPrimitive(it) }))
      }
    },
  )

private fun @P@McpBeforeAdmit(request: @N@McpHttpRequest, settings: @N@McpHttpSettings): @N@McpHttpResponse? {
  val oauth = settings.oauth
  if (oauth != null && request.method == "GET" &&
    request.path.split('?', '#').first() == @P@McpUrlPath(oauth.resourceMetadataUrl)
  ) {
    return @P@McpMetadata(oauth)
  }
  if (request.method != "POST") return @P@McpWithoutBody(405, listOf("Allow" to "POST"))
  val origin = request.header("origin")
  if (origin != null && origin !in settings.allowedOrigins) return @P@McpWithoutBody(403)
  return null
}

private fun @P@McpAfterAdmit(request: @N@McpHttpRequest): Pair<String?, @N@McpHttpResponse?> {
  val text = try {
    Charsets.UTF_8.newDecoder().decode(java.nio.ByteBuffer.wrap(request.body)).toString()
  } catch (rejected: Exception) {
    return null to @P@McpJsonResponse(400, @P@McpInvalid("the body is not UTF-8 text"))
  }
  val (message, refusal) = @P@McpReadRequest(text)
  if (message == null) return null to @P@McpJsonResponse(400, refusal ?: JsonNull)
  if (message["id"] == null) return null to @P@McpWithoutBody(202)
  val mismatch = @P@McpHeaderMismatch(request, message)
  if (mismatch != null) return null to mismatch
  val unchecked = @P@McpChecked(message)
  if (unchecked != null) return null to @P@McpJsonResponse(400, unchecked)
  val method = (message["method"] as JsonPrimitive).content
  if (method !in @P@McpMethods) {
    return null to @P@McpJsonResponse(
      404,
      @P@McpFailed(message["id"], @P@McpErrorObject(-32601, "Method not found: $method", null)),
    )
  }
  return text to null
}

private fun @P@McpEventStream(events: Collection<String>, answered: String?): @N@McpHttpResponse =
  @N@McpHttpResponse(
    200,
    listOf("Content-Type" to "text/event-stream", "X-Accel-Buffering" to "no"),
    (events + listOfNotNull(answered)).joinToString("") { "event: message\ndata: $it\n\n" }.encodeToByteArray(),
  )

private fun @P@McpHttpAnswer(answered: String?): @N@McpHttpResponse =
  if (answered == null) {
    @P@McpWithoutBody(202)
  } else {
    @N@McpHttpResponse(200, listOf("Content-Type" to "application/json"), answered.encodeToByteArray())
  }

/**
 * The Streamable HTTP rules around `@P@McpServe`: the metadata document, the method, the `Origin`,
 * `admit`, the body and the headers that must agree with it. [route] also receives the handlers
 * that `admit` built for the caller.
 */
suspend fun @P@McpServeHttp(
  request: @N@McpHttpRequest,
  settings: @N@McpHttpSettings,
  serverName: String,
  instructions: String?,
  endpoint: @N@McpEndpoint,
  admit: suspend (@N@McpHttpRequest, @N@McpCall) -> @N@McpAdmitted,
  route: suspend (caller: @N@Handlers, part: Int, method: String, params: JsonObject) -> @N@McpAnswer?,
): @N@McpHttpResponse {
  val before = @P@McpBeforeAdmit(request, settings)
  if (before != null) return before
  val events = java.util.concurrent.ConcurrentLinkedQueue<String>()
  val call = @N@McpCall(@P@McpBodyToken(request)) { event -> events.add(event) }
  val caller = when (val admitted = admit(request, call)) {
    is @N@McpAdmitted.Denied -> return @P@McpDenial(admitted.denied, settings, request)
    is @N@McpAdmitted.Caller -> admitted.handlers
  }
  val (text, refused) = @P@McpAfterAdmit(request)
  if (text == null) return refused ?: @P@McpWithoutBody(400)
  val asked = when (val begun = @P@McpBegin(text, instructions, endpoint, serverName)) {
    is @P@McpBegun.Answered -> return @P@McpHttpAnswer(begun.text)
    is @P@McpBegun.Notified -> return @P@McpHttpAnswer(null)
    is @P@McpBegun.Routed -> begun.request
  }
  val answered = @P@McpAsk(asked, endpoint, serverName) { part, method, params -> route(caller, part, method, params) }
  val written = if (call.cancelled) null else answered
  return if (@P@McpToken(asked) != null) @P@McpEventStream(events, written) else @P@McpHttpAnswer(written)
}

/** Answers the HTTP requests of this service alone. [admit] answers the handlers built for a request, or why it is refused. */
fun create@N@McpHttpDispatcher(
  settings: @N@McpHttpSettings,
  admit: suspend (@N@McpHttpRequest, @N@McpCall) -> @N@McpAdmitted,
): suspend (@N@McpHttpRequest) -> @N@McpHttpResponse {
  val endpoint = @N@McpEndpoint(@P@McpCatalog)
  return { request ->
    @P@McpServeHttp(request, settings, @P@McpServerName, @P@McpInstructions, endpoint, admit) { caller, _, method, params ->
      create@N@McpRoute(caller)(method, params)
    }
  }
}"#;

/// The type an `mcp(content)` operation answers a list of, as the service module declares it in
/// Rust: one block of a tool's `content`, read and written as the specification writes it.
pub const CONTENT_TYPE: &str = r#"/** One block of a tool's `content`: what an `mcp(content)` operation answers a list of. */
@Serializable(with = McpContentSerializer::class)
sealed interface McpContent {
  /** Text. */
  data class Text(val text: String) : McpContent

  /** An image: its bytes and its content type. */
  data class Image(val data: List<UByte>, val mimeType: String) : McpContent

  /** Audio: its bytes and its content type. */
  data class Audio(val data: List<UByte>, val mimeType: String) : McpContent

  /** A link to a resource the client can read. */
  data class ResourceLink(
    val uri: String,
    val name: String,
    val title: String? = null,
    val description: String? = null,
    val mimeType: String? = null,
  ) : McpContent

  /** A resource carried in the result. */
  data class Resource(val uri: String, val mimeType: String? = null, val contents: McpResourceContents) : McpContent
}

/** What an embedded resource holds. */
sealed interface McpResourceContents {
  /** Text. */
  data class Text(val text: String) : McpResourceContents

  /** Bytes. */
  data class Blob(val bytes: List<UByte>) : McpResourceContents
}

/** Reads and writes a [McpContent] as the block the specification writes. */
object McpContentSerializer : KSerializer<McpContent> {
  override val descriptor: SerialDescriptor = buildClassSerialDescriptor("McpContent")

  private fun encoded(bytes: List<UByte>): String =
    java.util.Base64.getEncoder().encodeToString(ByteArray(bytes.size) { bytes[it].toByte() })

  private fun decoded(from: JsonObject, key: String): List<UByte> =
    try {
      java.util.Base64.getDecoder().decode(text(from, key)).map { it.toUByte() }
    } catch (rejected: IllegalArgumentException) {
      throw SerializationException("`" + key + "` is not base64")
    }

  private fun text(from: JsonObject, key: String): String =
    optional(from, key) ?: throw SerializationException("a block holds no `" + key + "`")

  private fun optional(from: JsonObject, key: String): String? =
    (from[key] as? JsonPrimitive)?.takeIf { it.isString }?.content

  override fun serialize(encoder: Encoder, value: McpContent) {
    val output = encoder as JsonEncoder
    output.encodeJsonElement(
      when (value) {
        is McpContent.Text -> buildJsonObject { put("text", value.text); put("type", "text") }
        is McpContent.Image ->
          buildJsonObject { put("data", encoded(value.data)); put("mimeType", value.mimeType); put("type", "image") }
        is McpContent.Audio ->
          buildJsonObject { put("data", encoded(value.data)); put("mimeType", value.mimeType); put("type", "audio") }
        is McpContent.ResourceLink -> buildJsonObject {
          put("name", value.name)
          put("type", "resource_link")
          put("uri", value.uri)
          value.title?.let { put("title", it) }
          value.description?.let { put("description", it) }
          value.mimeType?.let { put("mimeType", it) }
        }
        is McpContent.Resource -> buildJsonObject {
          put(
            "resource",
            buildJsonObject {
              put("uri", value.uri)
              value.mimeType?.let { put("mimeType", it) }
              when (val held = value.contents) {
                is McpResourceContents.Text -> put("text", held.text)
                is McpResourceContents.Blob -> put("blob", encoded(held.bytes))
              }
            },
          )
          put("type", "resource")
        }
      },
    )
  }

  override fun deserialize(decoder: Decoder): McpContent {
    val block = (decoder as JsonDecoder).decodeJsonElement() as? JsonObject
      ?: throw SerializationException("a block is not an object")
    return when (optional(block, "type")) {
      "text" -> McpContent.Text(text(block, "text"))
      "image" -> McpContent.Image(decoded(block, "data"), text(block, "mimeType"))
      "audio" -> McpContent.Audio(decoded(block, "data"), text(block, "mimeType"))
      "resource_link" -> McpContent.ResourceLink(
        text(block, "uri"),
        text(block, "name"),
        optional(block, "title"),
        optional(block, "description"),
        optional(block, "mimeType"),
      )
      "resource" -> {
        val resource = block["resource"] as? JsonObject ?: throw SerializationException("a resource block holds no resource")
        McpContent.Resource(
          text(resource, "uri"),
          optional(resource, "mimeType"),
          if (resource["blob"] != null) McpResourceContents.Blob(decoded(resource, "blob")) else McpResourceContents.Text(text(resource, "text")),
        )
      }
      else -> throw SerializationException("a block of a type this client does not read")
    }
  }
}"#;

/// Where the accessor writes the tools, as the comma-joined Kotlin literals of their JSON text.
pub const TOOLS_MARKER: &str = "@MCP_TOOLS@";
/// Where the accessor writes the names, as `kind:name` Kotlin literals.
pub const NAMES_MARKER: &str = "@MCP_NAMES@";
/// Where the accessor writes the instructions: a Kotlin literal or `null`.
pub const INSTRUCTIONS_MARKER: &str = "@MCP_INSTRUCTIONS@";
/// Where the accessor writes the version of the crate that declares the service.
pub const VERSION_MARKER: &str = "@MCP_VERSION@";
/// Where the accessor writes the resources, as `Kotlin` literals of their JSON text.
pub const RESOURCES_MARKER: &str = "@MCP_RESOURCES@";
/// Where the accessor writes the resource templates, as `Kotlin` literals of their JSON text.
pub const TEMPLATES_MARKER: &str = "@MCP_TEMPLATES@";

/// The text the accessor replaces with the skills' JSON texts, each a Kotlin literal.
pub const SKILLS_MARKER: &str = "@MCP_SKILLS@";

/// The text the accessor replaces with the files of the skills folder, each `"uri" to "json"`.
pub const SKILL_FILES_MARKER: &str = "@MCP_SKILL_FILES@";
/// Where the emitted text lists the capabilities, as Kotlin literals of their JSON text.
const CAPABILITIES_MARKER: &str = "@MCP_CAPABILITIES@";

pub fn emit(declared: &ServiceDef) -> Vec<String> {
    kotlin_tuple_naming(TupleNaming::Named);
    let service = &declared.enveloped();
    let named = service.ident.to_string();
    let prefix = RenameRule::CamelCase.apply_to_variant(&named);
    let fields = fault_fields_typescript_name(&named);
    let held = Resourced::of(service);
    let mut published = vec![
        fill(CATALOG, &prefix, &named, &fields)
            .replace("@WRAPPED_MAP@", &tool_entries(service))
            .replace(
                CAPABILITIES_MARKER,
                &capabilities_literal(&held, skilled(service)),
            ),
        page_types(service),
        fill(ENDPOINT, &prefix, &named, &fields),
        fill(PROTOCOL, &prefix, &named, &fields),
        fill(WRITER, &prefix, &named, &fields),
        fill(SERVING, &prefix, &named, &fields),
        fill(RECEIVING, &prefix, &named, &fields),
        asking_text(service, &prefix, &named, &fields),
        blocks_text(service, &prefix, &named, &fields),
        route(service, &held, &prefix, &named, &fields),
        fill(DISPATCHERS, &prefix, &named, &fields),
        fill(HTTP, &prefix, &named, &fields)
            .replace("@SCOPES@", &scopes_table(&ScopeTable::of(service))),
    ];
    published.retain(|text| !text.is_empty());
    published.extend(kotlin_declarations());
    published.retain(|text| !text.is_empty());
    published
}

fn asking_text(service: &ServiceDef, prefix: &str, named: &str, fields: &str) -> String {
    if service
        .operations
        .iter()
        .any(|operation| operation.mcp.interview.is_some())
    {
        fill(INTERVIEW, prefix, named, fields)
    } else {
        String::new()
    }
}

fn blocks_text(service: &ServiceDef, prefix: &str, named: &str, fields: &str) -> String {
    let mut texts = Vec::new();
    if service
        .operations
        .iter()
        .any(|operation| operation.mcp.content.is_some())
    {
        texts.push(fill(CONTENT, prefix, named, fields));
    }
    if service.operations.iter().any(is_content_pair) {
        texts.push(fill(MEDIA, prefix, named, fields));
    }
    texts.join("\n\n")
}

/// Stands in the emitted text for the JSON text of the form `mcp_form_{operation}()` answers.
pub fn form_marker(operation: &OperationDef) -> String {
    format!("@MCP_FORM_{}@", written(&operation.ident))
}

/// The statements that read the answers of a retried form call into the arguments, ahead of the
/// message's own read.
fn answers_stmt(prefix: &str, operation: &OperationDef) -> String {
    let Some(wire) = answers_key(operation) else {
        return String::new();
    };
    let key = kotlin_quoted(&wire);
    let tool = kotlin_quoted(operation.mcp_tool_name());
    let read = match &operation.inputs {
        OperationInputs::Generated(fields) => fields
            .iter()
            .find(|(field, _)| wire_key(field) == wire)
            .and_then(|(_, ty)| option_inner(ty)),
        OperationInputs::Empty | OperationInputs::Named(_) => None,
    }
    .map_or_else(String::new, |form| {
        format!(
            "          try {{\n            \
             Json.decodeFromJsonElement({}, mcpAnswered)\n          \
             }} catch (rejected: IllegalArgumentException) {{\n            \
             {prefix}McpRefuse(\"failed-validation\", {tool}, rejected.message ?: rejected.toString(), {key})\n          \
             }}\n",
            read_serializer(form)
        )
    });
    format!(
        "      var mcpDeclined = false\n      \
         val mcpResponse = (mcpParams[\"inputResponses\"] as? JsonObject)?.get({key}) as? JsonObject\n      \
         if (mcpResponse != null) {{\n        \
         if ((mcpResponse[\"action\"] as? JsonPrimitive)?.content == \"accept\") {{\n          \
         val mcpAnswered = mcpResponse[\"content\"] ?: JsonObject(emptyMap())\n{read}          \
         mcpArguments[{key}] = mcpAnswered\n        \
         }} else {{\n          \
         mcpArguments.remove({key})\n          \
         mcpDeclined = true\n        \
         }}\n      \
         }}\n"
    )
}

/// `wrapped_entries` for the operations that are tools.
fn tool_entries(service: &ServiceDef) -> String {
    wrapped_entries_of(service, true)
}

fn scopes_table(table: &ScopeTable) -> String {
    let list = |scopes: &[String]| {
        format!(
            "listOf({})",
            scopes
                .iter()
                .map(|scope| kotlin_quoted(scope))
                .collect::<Vec<_>>()
                .join(", ")
        )
    };
    let mut text = String::new();
    if table.has_templates() {
        text.push_str(
            "  fun matches(uri: String, template: List<Pair<String, Boolean>>): Boolean {\n    \
             var rest = uri\n    \
             for ((text, placeholder) in template) {\n      \
             if (placeholder) {\n        \
             val slash = rest.indexOf('/')\n        \
             val end = if (slash < 0) rest.length else slash\n        \
             if (end == 0) return false\n        \
             rest = rest.substring(end)\n      \
             } else {\n        \
             if (!rest.startsWith(text)) return false\n        \
             rest = rest.substring(text.length)\n      \
             }\n    \
             }\n    \
             return rest.isEmpty()\n  \
             }\n\n",
        );
    }
    if !table.tools.is_empty() {
        text.push_str("  if (method == \"tools/call\") {\n    when (name) {\n");
        for (tool, scopes) in &table.tools {
            let _ = writeln!(
                text,
                "      {} -> return {}",
                kotlin_quoted(tool),
                list(scopes)
            );
        }
        text.push_str("    }\n  }\n");
    }
    if !table.reads.is_empty() {
        text.push_str("  if (method == \"resources/read\" && name != null) {\n");
        for (uri, scopes) in &table.reads {
            match uri {
                ReadUri::Fixed(fixed) => {
                    let _ = writeln!(
                        text,
                        "    if (name == {}) return {}",
                        kotlin_quoted(fixed),
                        list(scopes)
                    );
                }
                ReadUri::Template(parts) => {
                    let pairs: Vec<String> = parts
                        .iter()
                        .map(|(part, placeholder)| {
                            format!("{} to {placeholder}", kotlin_quoted(part))
                        })
                        .collect();
                    let _ = writeln!(
                        text,
                        "    if (matches(name, listOf({}))) return {}",
                        pairs.join(", "),
                        list(scopes)
                    );
                }
            }
        }
        text.push_str("    return emptyList()\n  }\n");
    }
    if let Some(scopes) = &table.listing {
        let _ = writeln!(
            text,
            "  if (method == \"resources/list\") return {}",
            list(scopes)
        );
    }
    if !table.completions.is_empty() {
        text.push_str("  if (method == \"completion/complete\" && name != null) {\n");
        for (uri, scopes) in &table.completions {
            let _ = writeln!(
                text,
                "    if (name == {}) return {}",
                kotlin_quoted(uri),
                list(scopes)
            );
        }
        text.push_str("  }\n");
    }
    text
}

/// One `"tool" to <marker>,` entry per tool: the accessor writes `true` or `false` over each
/// marker, from `wrapped_{operation}()`.
pub(super) fn wrapped_entries(service: &ServiceDef) -> String {
    wrapped_entries_of(service, false)
}

fn wrapped_entries_of(service: &ServiceDef, tools_only: bool) -> String {
    service
        .operations
        .iter()
        .enumerate()
        .filter(|(_, operation)| !tools_only || operation.is_mcp_tool())
        .map(|(at, operation)| {
            format!(
                "{} to {},",
                kotlin_quoted(operation.mcp_tool_name()),
                wrapped_marker(at)
            )
        })
        .collect::<Vec<_>>()
        .join("\n  ")
}

/// The marker the accessor replaces with the wrapping of the operation at `at`.
pub fn wrapped_marker(at: usize) -> String {
    format!("@MCP_WRAPPED_{at}@")
}

/// `template` with the service's names written over their placeholders.
fn fill(template: &str, prefix: &str, named: &str, fields: &str) -> String {
    template
        .replace("@P@", prefix)
        .replace("@N@", named)
        .replace("@F@", fields)
}

/// `kotlin_mcp_service()` on the service's registry: the text above, with the tools, the names and
/// the instructions, which only the compiled service can give, written over their markers when it
/// runs in the consuming program.
pub fn accessor(service: &ServiceDef) -> TokenStream {
    let text = emit(service).join("\n\n");
    let wrapped = wrapped_replacements(service);
    let literal = literal_fn();
    let forms: Vec<TokenStream> = service
        .operations
        .iter()
        .filter(|operation| {
            operation
                .mcp
                .interview
                .as_ref()
                .is_some_and(|interview| interview.answers.is_some())
        })
        .map(|operation| {
            let marker = form_marker(operation);
            let method = format_ident!(
                "mcp_form_{}",
                written(&operation.ident),
                span = operation.ident.span()
            );
            quote! { text = text.replace(#marker, &literal(&Self::#method().to_string())); }
        })
        .collect();
    quote! {
        #[doc = " The service's generated Kotlin `mcp` server: its catalog, the route that answers"]
        #[doc = " its `tools/call`, the function that answers one message for the services of an"]
        #[doc = " endpoint, and the dispatchers over text and over plain HTTP terms."]
        pub fn kotlin_mcp_service() -> ::std::string::String {
            #literal
            let tools = Self::mcp_tools()
                .iter()
                .map(|tool| literal(&::std::string::ToString::to_string(tool)))
                .collect::<::std::vec::Vec<_>>()
                .join(", ");
            let names = Self::MCP_NAMES
                .iter()
                .map(|(kind, name)| literal(&format!("{kind}:{name}")))
                .collect::<::std::vec::Vec<_>>()
                .join(", ");
            let resources = Self::mcp_resources()
                .iter()
                .map(|resource| literal(&::std::string::ToString::to_string(resource)))
                .collect::<::std::vec::Vec<_>>()
                .join(", ");
            let templates = Self::mcp_resource_templates()
                .iter()
                .map(|template| literal(&::std::string::ToString::to_string(template)))
                .collect::<::std::vec::Vec<_>>()
                .join(", ");
            let skills = Self::mcp_skills()
                .iter()
                .map(|skill| literal(&::std::string::ToString::to_string(skill)))
                .collect::<::std::vec::Vec<_>>()
                .join(", ");
            let skill_files = Self::mcp_skill_files()
                .iter()
                .map(|file| {
                    format!(
                        "{} to {}",
                        literal(file["uri"].as_str().unwrap_or_default()),
                        literal(&::std::string::ToString::to_string(file))
                    )
                })
                .collect::<::std::vec::Vec<_>>()
                .join(", ");
            let instructions = Self::MCP_INSTRUCTIONS.map_or_else(|| "null".to_owned(), literal);
            let mut text = #text
                .replace(#TOOLS_MARKER, &tools)
                .replace(#RESOURCES_MARKER, &resources)
                .replace(#TEMPLATES_MARKER, &templates)
                .replace(#SKILLS_MARKER, &skills)
                .replace(#SKILL_FILES_MARKER, &skill_files)
                .replace(#NAMES_MARKER, &names)
                .replace(#INSTRUCTIONS_MARKER, &instructions)
                .replace(#VERSION_MARKER, env!("CARGO_PKG_VERSION"));
            #(#wrapped)*
            #(#forms)*
            text
        }
    }
}

/// One statement per tool that writes whether its message travels under `value`.
pub(super) fn wrapped_replacements(service: &ServiceDef) -> Vec<TokenStream> {
    service
        .enveloped()
        .operations
        .iter()
        .enumerate()
        .map(|(at, operation)| {
            let marker = wrapped_marker(at);
            let wrapped = wrapped_ident(operation);
            quote! { text = text.replace(#marker, &::std::string::ToString::to_string(&Self::#wrapped())); }
        })
        .collect()
}

/// The function that writes a text as a Kotlin string literal, in the consuming program.
fn literal_fn() -> TokenStream {
    quote! {
        fn literal(text: &str) -> ::std::string::String {
            let mut written = ::std::string::String::from("\"");
            for letter in text.chars() {
                match letter {
                    '\\' => written.push_str("\\\\"),
                    '"' => written.push_str("\\\""),
                    '$' => written.push_str("\\$"),
                    '\n' => written.push_str("\\n"),
                    '\r' => written.push_str("\\r"),
                    '\t' => written.push_str("\\t"),
                    other if other < ' ' => {
                        written.push_str(&format!("\\u{:04x}", ::core::primitive::u32::from(other)));
                    }
                    other => written.push(other),
                }
            }
            written.push('"');
            written
        }
    }
}

fn route(
    service: &ServiceDef,
    held: &Resourced<'_>,
    prefix: &str,
    named: &str,
    fields: &str,
) -> String {
    let tools: Vec<&OperationDef> = service
        .operations
        .iter()
        .filter(|operation| operation.is_mcp_tool())
        .collect();
    let mut dispatch = String::new();
    if !tools.is_empty() {
        dispatch.push_str("      \"tools/call\" -> @P@McpTool(handlers, mcpParams)\n");
    }
    if held.reads().next().is_some() || skilled(service) {
        dispatch.push_str("      \"resources/read\" -> @P@McpRead(handlers, mcpParams)\n");
    }
    if skilled(service) {
        dispatch.push_str("      \"skills/get\" -> @P@McpSkillGet(mcpParams)\n");
    }
    if held.listing.is_some() {
        dispatch.push_str("      \"resources/list\" -> @P@McpList(handlers, mcpParams)\n");
    }
    if !held.completions.is_empty() {
        dispatch.push_str("      \"completion/complete\" -> @P@McpComplete(handlers, mcpParams)\n");
    }
    let mut text = fill(ROUTE_HEAD, prefix, named, fields)
        .replace("@DISPATCH@", &fill(&dispatch, prefix, named, fields));
    if !tools.is_empty() {
        let arms = tools
            .iter()
            .map(|operation| arm(operation, prefix, named, fields))
            .collect::<Vec<_>>()
            .join("\n");
        text.push_str(&fill(TOOL_HEAD, prefix, named, fields));
        text.push_str(&arms);
        text.push('\n');
        text.push_str(TOOL_TAIL);
    }
    text.push_str(&resource_functions(service, held, prefix, named, fields));
    text
}

/// The `capabilities` the catalog lists, as the Kotlin literals of their JSON text.
fn capabilities_literal(held: &Resourced<'_>, skilled: bool) -> String {
    let mut capabilities = vec![kotlin_quoted("{\"tools\":{}}")];
    if held.is_served() || skilled {
        capabilities.push(kotlin_quoted("{\"resources\":{}}"));
    }
    if skilled {
        capabilities.push(kotlin_quoted(
            "{\"extensions\":{\"io.modelcontextprotocol/skills\":{}}}",
        ));
    }
    if !held.completions.is_empty() {
        capabilities.push(kotlin_quoted("{\"completions\":{}}"));
    }
    capabilities.join(", ")
}

/// Whether the service ships a skills folder or has an operation that writes a skill.
fn skilled(service: &ServiceDef) -> bool {
    service.skills.is_some() || !dynamic_skills(service).is_empty()
}

/// The operations that write a skill at run time.
fn dynamic_skills(service: &ServiceDef) -> Vec<&OperationDef> {
    service
        .operations
        .iter()
        .filter(|operation| operation.mcp.skill.is_some())
        .collect()
}

/// `McpResource` and `McpResourcePage`, which a listing answers and the Rust service module
/// declares.
fn page_types(service: &ServiceDef) -> String {
    if !service
        .operations
        .iter()
        .any(|operation| operation.mcp.list_resources)
    {
        return String::new();
    }
    "/** One resource of a listing, as `resources/list` writes it. */
@Serializable
data class McpResource(
  val uri: String,
  val name: String,
  val title: String? = null,
  val description: String? = null,
  val mimeType: String? = null,
  val size: Long? = null,
)

/** One page of `resources/list`, with the cursor of the next where there is one. */
@Serializable
data class McpResourcePage(val resources: List<McpResource>, val nextCursor: String? = null)"
        .to_owned()
}

/// The functions that answer the requests the resources, the listing and the completions own.
fn resource_functions(
    service: &ServiceDef,
    held: &Resourced<'_>,
    prefix: &str,
    named: &str,
    fields: &str,
) -> String {
    if !held.any() && !skilled(service) {
        return String::new();
    }
    let mut text = fill(RESOURCE_SUPPORT, prefix, named, fields);
    if held.reads().next().is_some() || skilled(service) {
        text.push_str(&read_dispatch(service, held, prefix, named));
        for operation in held.reads() {
            text.push_str(&resource_read(operation, prefix, named, fields));
        }
        for operation in dynamic_skills(service) {
            text.push_str(&skill_read(operation, prefix, named, fields));
        }
    }
    if skilled(service) {
        text.push_str(&skill_get(prefix, named));
    }
    if let Some(operation) = held.listing {
        text.push_str(&resource_listing(operation, prefix, named, fields));
    }
    if !held.completions.is_empty() {
        text.push_str(&complete_dispatch(service, held, prefix, named));
        for operation in &held.completions {
            text.push_str(&resource_completion(operation, prefix, named, fields));
        }
    }
    text
}

/// The read of a file of a skill an operation writes: the frontmatter ahead of the manifest's text.
fn skill_read(operation: &OperationDef, prefix: &str, named: &str, fields: &str) -> String {
    let Some((skill, _)) = operation.mcp.skill.as_ref() else {
        return String::new();
    };
    let blob = matches!(
        &operation.outcome,
        OperationOutcome::Reply { success, error: _error } if matches!(success.as_ref(), Type::Tuple(_))
    );
    let content = if blob {
        "          val mcpBytes = answered.value.slot0.map { it.toByte() }.toByteArray()\n          \
         put(\"blob\", java.util.Base64.getEncoder().encodeToString(mcpBytes))\n          \
         put(\"mimeType\", answered.value.slot1)"
            .to_owned()
    } else {
        let frontmatter = kotlin_quoted(&written_frontmatter(
            skill,
            operation.doc.as_deref().unwrap_or_default(),
        ));
        format!(
            "          put(\"mimeType\", \"text/markdown\")\n          \
             put(\"text\", (if (mcpPath == \"SKILL.md\") {frontmatter} else \"\") + mcpEncoded.jsonPrimitive.content)"
        )
    };
    let success = format!(
        "        {prefix}McpAnswerOf(\n          \"private\",\n          0,\n          \"contents\",\n          \
         buildJsonArray {{\n            add(\n              buildJsonObject {{\n          put(\"uri\", mcpUri)\n{content}\n              }},\n            )\n          }},\n        )"
    );
    let head = format!(
        "\n\nprivate suspend fun {prefix}McpReadSkill{op}(handlers: {named}Handlers, mcpUri: String, mcpPath: String): {named}McpAnswer {{\n",
        op = pascal(operation),
    );
    format!(
        "{head}{}",
        resource_body(
            operation,
            prefix,
            named,
            fields,
            "JsonPrimitive(mcpPath)",
            "\"Skill file not found\"",
            &success,
        )
    )
}

/// The `skills/get` of the service: the entry its catalog lists under the name or the URI asked.
fn skill_get(prefix: &str, named: &str) -> String {
    format!(
        "\n\nprivate fun {prefix}McpSkillGet(mcpParams: JsonObject): {named}McpAnswer? {{\n  \
         val mcpName = (mcpParams[\"name\"] as? JsonPrimitive)?.takeIf {{ it.isString }}?.content\n  \
         val mcpUri = (mcpParams[\"uri\"] as? JsonPrimitive)?.takeIf {{ it.isString }}?.content\n  \
         for (text in {prefix}McpCatalog[\"skills\"].orEmpty()) {{\n    \
         val entry = Json.parseToJsonElement(text) as JsonObject\n    \
         val frontmatter = entry[\"frontmatter\"] as? JsonObject\n    \
         val held = (mcpName != null && (frontmatter?.get(\"name\") as? JsonPrimitive)?.content == mcpName) ||\n      \
         (mcpUri != null && (entry[\"uri\"] as? JsonPrimitive)?.content == mcpUri)\n    \
         if (!held) continue\n    \
         val written = (entry[\"resources\"] as? JsonPrimitive)?.takeIf {{ it.isString }}?.content == \"dynamic\"\n    \
         return {named}McpAnswer.Answered(\n      buildJsonObject {{\n        \
         put(\"cacheScope\", if (written) \"private\" else \"public\")\n        \
         put(\"resultType\", \"complete\")\n        \
         put(\"skill\", entry)\n        \
         put(\"ttlMs\", if (written) 0 else {prefix}McpListedForMs)\n      }},\n    )\n  }}\n  \
         return null\n}}"
    )
}

fn read_dispatch(service: &ServiceDef, held: &Resourced<'_>, prefix: &str, named: &str) -> String {
    let mut text = format!(
        "\n\nprivate suspend fun {prefix}McpRead(handlers: {named}Handlers, mcpParams: JsonObject): {named}McpAnswer? {{\n  \
         val mcpUri = (mcpParams[\"uri\"] as? JsonPrimitive)?.takeIf {{ it.isString }}?.content ?: return null\n"
    );
    if service.skills.is_some() {
        let _ = write!(
            text,
            "  val mcpFolder = {prefix}McpSkillFiles[mcpUri]\n  \
             if (mcpFolder != null) {{\n    \
             return {prefix}McpAnswerOf(\"public\", {prefix}McpListedForMs, \"contents\", buildJsonArray {{ add(Json.parseToJsonElement(mcpFolder)) }})\n  }}\n"
        );
    }
    for operation in dynamic_skills(service) {
        let Some((skill, _)) = operation.mcp.skill.as_ref() else {
            continue;
        };
        let root = kotlin_quoted(&format!("skill://{skill}/"));
        let _ = write!(
            text,
            "  if (mcpUri.startsWith({root}) && mcpUri.length > {root}.length) {{\n    \
             return {prefix}McpReadSkill{op}(handlers, mcpUri, mcpUri.substring({root}.length))\n  }}\n",
            op = pascal(operation),
        );
    }
    for (at, operation) in held.reads().enumerate() {
        let Some(resource) = operation.mcp.resource.as_ref() else {
            continue;
        };
        let segments = if resource.is_template() {
            resource
                .segments
                .iter()
                .map(|segment| match segment {
                    PathSegment::Literal(literal) => format!("{} to false", kotlin_quoted(literal)),
                    PathSegment::Placeholder(name) => format!("{} to true", kotlin_quoted(name)),
                })
                .collect::<Vec<_>>()
                .join(", ")
        } else {
            format!("{} to false", kotlin_quoted(&resource.uri))
        };
        let uses_captured = !resource.placeholder_names().is_empty()
            && !matches!(operation.inputs, OperationInputs::Empty);
        let local = if uses_captured {
            format!("mcpCaptured{at}")
        } else {
            format!("mcpMatched{at}")
        };
        let arguments = if uses_captured {
            format!("handlers, mcpUri, {local}")
        } else {
            "handlers, mcpUri".to_owned()
        };
        let _ = write!(
            text,
            "  val {local} = {prefix}McpCaptured(listOf({segments}), mcpUri)\n  \
             if ({local} != null) return {prefix}McpRead{op}({arguments})\n",
            op = pascal(operation),
        );
    }
    text.push_str("  return null\n}");
    text
}

fn pascal(operation: &OperationDef) -> String {
    RenameRule::PascalCase.apply_to_field(&written(&operation.ident))
}

/// The statements that read the message of `operation` from `mcpSource`, call its handler and
/// answer what `answered` holds through `success`, where `success` has the JSON of the answer as
/// `mcpEncoded`.
fn resource_body(
    operation: &OperationDef,
    prefix: &str,
    named: &str,
    fields: &str,
    source: &str,
    refusal: &str,
    success: &str,
) -> String {
    let OperationOutcome::Reply { error, success: ty } = &operation.outcome else {
        return String::new();
    };
    let wire = kotlin_quoted(&operation.wire_name);
    let read_by = message_serializer(operation);
    let result = result_name(named, operation).unwrap_or_default();
    let call = format!("handlers.{}(decoded)", kotlin_name(&operation.ts_name));
    let written_error = read_serializer(error);
    let encoded = if matches!(ty.as_ref(), Type::Tuple(_)) {
        String::new()
    } else {
        format!(
            "        val mcpEncoded = Json.encodeToJsonElement({}, answered.value)\n",
            read_serializer(ty)
        )
    };
    format!(
        "  val mcpSource: JsonElement = {source}\n  \
         val decoded = {prefix}McpReceive({wire}, {read_by}, mcpSource) {{ kind, detail, field ->\n    \
         {prefix}McpUnread({wire}, kind, detail, field)\n  }}\n  \
         val answered = try {{\n    {call}\n  }} catch (unexpected: Throwable) {{\n    \
         if (unexpected is CancellationException) throw unexpected\n    \
         throw {prefix}McpRefused({prefix}McpInternal({wire}, unexpected.message ?: unexpected.toString()))\n  }}\n  \
         return try {{\n    when (answered) {{\n      \
         is {result}.Ok -> {{\n{encoded}{success}\n      }}\n      \
         is {result}.Declared -> {prefix}McpDeclined(Json.encodeToJsonElement({written_error}, answered.error), {refusal})\n      \
         is {result}.Fault -> {prefix}McpFaultedRequest(Json.encodeToJsonElement(serializer<{fields}>(), answered.fault))\n    \
         }}\n  }} catch (unwritten: SerializationException) {{\n    \
         throw {prefix}McpRefused({prefix}McpInternal({wire}, \"the answer would not serialize: ${{unwritten.message}}\"))\n  }}\n}}"
    )
}

fn resource_read(operation: &OperationDef, prefix: &str, named: &str, fields: &str) -> String {
    let Some(resource) = operation.mcp.resource.as_ref() else {
        return String::new();
    };
    let slots = resource.placeholder_names();
    let uses_captured = !slots.is_empty() && !matches!(operation.inputs, OperationInputs::Empty);
    let source = match &operation.inputs {
        OperationInputs::Empty => "JsonObject(emptyMap())".to_owned(),
        OperationInputs::Named(_) if slots.is_empty() => "JsonNull".to_owned(),
        OperationInputs::Named(_) => "JsonPrimitive(mcpCaptured[0])".to_owned(),
        OperationInputs::Generated(declared) => {
            let members = slots
                .iter()
                .enumerate()
                .filter_map(|(at, name)| {
                    let (field, _) = declared.iter().find(|(field, _)| written(field) == *name)?;
                    Some(format!(
                        "put({}, mcpCaptured[{at}])",
                        kotlin_quoted(&wire_key(field))
                    ))
                })
                .collect::<Vec<_>>()
                .join("; ");
            format!("buildJsonObject {{ {members} }}")
        }
    };
    let ttl = operation.mcp.ttl_ms.unwrap_or(0);
    let scope = operation.mcp.cache_scope.as_deref().unwrap_or("private");
    let blob = matches!(
        &operation.outcome,
        OperationOutcome::Reply { success, error: _error } if matches!(success.as_ref(), Type::Tuple(_))
    );
    let content = if blob {
        "          val mcpBytes = answered.value.slot0.map { it.toByte() }.toByteArray()\n          \
         put(\"blob\", java.util.Base64.getEncoder().encodeToString(mcpBytes))\n          \
         put(\"mimeType\", answered.value.slot1)"
            .to_owned()
    } else {
        let mime = operation
            .mcp
            .mime_type
            .as_ref()
            .map(|mime| format!("          put(\"mimeType\", {})\n", kotlin_quoted(mime)))
            .unwrap_or_default();
        format!("{mime}          put(\"text\", mcpEncoded.jsonPrimitive.content)")
    };
    let success = format!(
        "        {prefix}McpAnswerOf(\n          {scope},\n          {ttl},\n          \"contents\",\n          \
         buildJsonArray {{\n            add(\n              buildJsonObject {{\n          put(\"uri\", mcpUri)\n{content}\n              }},\n            )\n          }},\n        )",
        scope = kotlin_quoted(scope),
    );
    let captured = if uses_captured {
        ", mcpCaptured: List<String>"
    } else {
        ""
    };
    let head = format!(
        "\n\nprivate suspend fun {prefix}McpRead{op}(handlers: {named}Handlers, mcpUri: String{captured}): {named}McpAnswer {{\n",
        op = pascal(operation),
    );
    format!(
        "{head}{}",
        resource_body(
            operation,
            prefix,
            named,
            fields,
            &source,
            "\"Resource not found\"",
            &success,
        )
    )
}

fn resource_listing(operation: &OperationDef, prefix: &str, named: &str, fields: &str) -> String {
    let source = if matches!(operation.inputs, OperationInputs::Named(_)) {
        "(mcpCursor as? JsonPrimitive)?.takeIf { it.isString } ?: JsonNull"
    } else {
        "JsonObject(emptyMap())"
    };
    let ttl = operation.mcp.ttl_ms.unwrap_or(0);
    let scope = operation.mcp.cache_scope.as_deref().unwrap_or("private");
    let success = format!(
        "        {named}McpAnswer.Answered(\n          JsonObject(\n            mcpEncoded.jsonObject + mapOf(\n              \"cacheScope\" to JsonPrimitive({scope}),\n              \"resultType\" to JsonPrimitive(\"complete\"),\n              \"ttlMs\" to JsonPrimitive({ttl}),\n            ),\n          ),\n        )",
        scope = kotlin_quoted(scope),
    );
    let head = format!(
        "\n\nprivate suspend fun {prefix}McpList(handlers: {named}Handlers, mcpParams: JsonObject): {named}McpAnswer {{\n  \
         val mcpCursor = mcpParams[\"cursor\"]\n"
    );
    format!(
        "{head}{}",
        resource_body(
            operation,
            prefix,
            named,
            fields,
            source,
            "\"Resources could not be listed\"",
            &success,
        )
    )
}

fn resource_completion(
    operation: &OperationDef,
    prefix: &str,
    named: &str,
    fields: &str,
) -> String {
    let OperationInputs::Generated(members) = &operation.inputs else {
        return String::new();
    };
    let (Some(typed), Some(chosen)) = (members.first(), members.get(1)) else {
        return String::new();
    };
    let source = format!(
        "buildJsonObject {{ put({}, mcpTyped); put({}, mcpChosen) }}",
        kotlin_quoted(&wire_key(&typed.0)),
        kotlin_quoted(&wire_key(&chosen.0)),
    );
    let success = format!(
        "        val mcpSuggested = mcpEncoded.jsonArray\n        \
         {named}McpAnswer.Answered(\n          buildJsonObject {{\n            \
         put(\n              \"completion\",\n              buildJsonObject {{\n                \
         put(\"hasMore\", mcpSuggested.size > 100)\n                \
         if (mcpSuggested.size > 100) put(\"total\", mcpSuggested.size)\n                \
         put(\"values\", JsonArray(mcpSuggested.take(100)))\n              }},\n            )\n            \
         put(\"resultType\", \"complete\")\n          }},\n        )"
    );
    let head = format!(
        "\n\nprivate suspend fun {prefix}McpComplete{op}(handlers: {named}Handlers, mcpTyped: String, mcpChosen: JsonElement): {named}McpAnswer {{\n",
        op = pascal(operation),
    );
    format!(
        "{head}{}",
        resource_body(
            operation,
            prefix,
            named,
            fields,
            &source,
            "\"Completion refused\"",
            &success,
        )
    )
}

fn complete_dispatch(
    service: &ServiceDef,
    held: &Resourced<'_>,
    prefix: &str,
    named: &str,
) -> String {
    let mut text = format!(
        "\n\nprivate suspend fun {prefix}McpComplete(handlers: {named}Handlers, mcpParams: JsonObject): {named}McpAnswer? {{\n  \
         val mcpReference = mcpParams[\"ref\"] as? JsonObject ?: return null\n  \
         if ((mcpReference[\"type\"] as? JsonPrimitive)?.takeIf {{ it.isString }}?.content != \"ref/resource\") return null\n  \
         val mcpUri = (mcpReference[\"uri\"] as? JsonPrimitive)?.takeIf {{ it.isString }}?.content ?: return null\n  \
         val mcpArgument = mcpParams[\"argument\"] as? JsonObject ?: return null\n  \
         val mcpParameter = (mcpArgument[\"name\"] as? JsonPrimitive)?.takeIf {{ it.isString }}?.content ?: return null\n  \
         val mcpTyped = (mcpArgument[\"value\"] as? JsonPrimitive)?.takeIf {{ it.isString }}?.content ?: \"\"\n  \
         val mcpContext = mcpParams[\"context\"] as? JsonObject\n  \
         val mcpChosen: JsonElement = if (mcpContext != null && \"arguments\" in mcpContext) mcpContext.getValue(\"arguments\") else JsonObject(emptyMap())\n"
    );
    for operation in &held.completions {
        let Some(completes) = operation.mcp.completes.as_ref() else {
            continue;
        };
        let Some(target) = service
            .operations
            .iter()
            .find(|candidate| written(&candidate.ident) == completes.resource)
        else {
            continue;
        };
        let Some(resource) = target.mcp.resource.as_ref() else {
            continue;
        };
        let _ = writeln!(
            text,
            "  if (mcpUri == {} && mcpParameter == {}) return {prefix}McpComplete{}(handlers, mcpTyped, mcpChosen)",
            kotlin_quoted(&resource.uri),
            kotlin_quoted(&completes.parameter),
            pascal(operation),
        );
    }
    text.push_str("  return null\n}");
    text
}

fn arm(operation: &OperationDef, prefix: &str, named: &str, fields: &str) -> String {
    let tool = kotlin_quoted(operation.mcp_tool_name());
    let shape = HttpShape::of(operation);
    let read_by = message_serializer(operation);
    let mut text = format!(
        "    {tool} -> {{\n      \
         val mcpArguments = {prefix}McpArguments({tool}, mcpParams)\n{}",
        answers_stmt(prefix, operation)
    );
    if !shape.header_in.is_empty() {
        let header_names = shape
            .header_in
            .iter()
            .map(|header| kotlin_quoted(&header.name))
            .collect::<Vec<_>>()
            .join(", ");
        let _ = writeln!(
            text,
            "      val mcpHeaders = listOf({header_names}).associateWith {{ mcpArguments.remove(it) }}"
        );
    }
    let _ = write!(
        text,
        "      val mcpSource = {prefix}McpSource({tool}, mcpArguments, \
         {prefix}McpWrapped.getValue({tool}))\n      \
         val decoded = {prefix}McpReceive({tool}, {read_by}, mcpSource)\n"
    );
    let mut call_arguments = vec!["decoded".to_owned()];
    for header in &shape.header_in {
        let held = kotlin_held(&header.parameter);
        let name = kotlin_quoted(&header.name);
        let line = option_inner(&header.ty).map_or_else(
            || {
                format!(
                    "      val {held} = {prefix}McpHeader({tool}, {name}, {}, mcpHeaders[{name}])\n",
                    read_serializer(&header.ty)
                )
            },
            |inner| {
                format!(
                    "      val {held} = {prefix}McpOptionalHeader({tool}, {name}, {}, mcpHeaders[{name}])\n",
                    read_serializer(inner)
                )
            },
        );
        text.push_str(&line);
        call_arguments.push(held);
    }
    let call = format!(
        "handlers.{}({})",
        kotlin_name(&operation.ts_name),
        call_arguments.join(", ")
    );
    match &operation.outcome {
        OperationOutcome::OneWay => {
            let _ = write!(
                text,
                "      try {{\n        {call}\n      }} catch (unexpected: Throwable) {{\n        \
                 if (unexpected is CancellationException) throw unexpected\n      }}\n      \
                 {prefix}McpDone(null, null)\n    }}\n"
            );
        }
        OperationOutcome::Reply { error, success } => {
            let result = result_name(named, operation).unwrap_or_default();
            let ok = ok_arm(operation, &result, prefix, &shape, success);
            let declared = declared_arm(operation, &result, prefix, &shape, error);
            let _ = write!(
                text,
                "      val answered = try {{\n        {call}\n      }} catch (unexpected: Throwable) {{\n        \
                 if (unexpected is CancellationException) throw unexpected\n        \
                 throw {prefix}McpRefused({prefix}McpInternal({tool}, unexpected.message ?: unexpected.toString()))\n      }}\n      \
                 try {{\n        when (answered) {{\n{ok}{declared}          \
                 is {result}.Fault -> {prefix}McpFaulted(Json.encodeToJsonElement(serializer<{fields}>(), answered.fault))\n        \
                 }}\n      }} catch (unwritten: SerializationException) {{\n        \
                 throw {prefix}McpRefused({prefix}McpInternal({tool}, \"the answer would not serialize: ${{unwritten.message}}\"))\n      }}\n    }}\n"
            );
        }
    }
    text
}

/// The arm that answers the declared success.
fn ok_arm(
    operation: &OperationDef,
    result: &str,
    prefix: &str,
    shape: &HttpShape,
    success: &Type,
) -> String {
    if is_content_list(operation) {
        return format!(
            "          is {result}.Ok -> {prefix}McpBlocks(answered.value.map {{ Json.encodeToJsonElement(McpContentSerializer, it) }})\n"
        );
    }
    if is_content_pair(operation) {
        return format!(
            "          is {result}.Ok -> {prefix}McpMedia(answered.value.slot0, answered.value.slot1)\n            \
             ?: {prefix}McpDone(Json.encodeToJsonElement(answered.value), null)\n"
        );
    }
    if shape.header_out.is_empty() {
        if is_unit_type(success) {
            return format!("          is {result}.Ok -> {prefix}McpDone(null, null)\n");
        }
        let written = read_serializer(success);
        return format!(
            "          is {result}.Ok -> {prefix}McpDone(Json.encodeToJsonElement({written}, answered.value), null)\n"
        );
    }
    let extra = header_elements(&shape.header_out, success, "headerOut");
    let written = read_serializer(body_type(shape.header_out.len(), success));
    let puts = header_puts(prefix, &extra, "answered.value");
    format!(
        "          is {result}.Ok -> {{\n            \
         val members = mutableMapOf<String, JsonElement>(\"value\" to Json.encodeToJsonElement({written}, answered.value.value))\n\
{puts}            \
         {prefix}McpDone(JsonObject(members), null)\n          }}\n"
    )
}

/// The arm that answers the declared error.
fn declared_arm(
    operation: &OperationDef,
    result: &str,
    prefix: &str,
    shape: &HttpShape,
    error: &Type,
) -> String {
    if let Some(interview) = operation.mcp.interview.as_ref() {
        let written = read_serializer(error);
        let answers =
            answers_key(operation).map_or_else(|| "null".to_owned(), |key| kotlin_quoted(&key));
        let form = if interview.answers.is_some() {
            form_marker(operation)
        } else {
            "null".to_owned()
        };
        let declined = if interview.answers.is_some() {
            "mcpDeclined"
        } else {
            "false"
        };
        return format!(
            "          is {result}.Declared -> {{\n            \
             val mcpBody = Json.encodeToJsonElement({written}, answered.error)\n            \
             {prefix}McpAsking({}, {answers}, {form}, {}, mcpBody, mcpParams, {declined})\n              \
             ?: {prefix}McpDone(null, {prefix}McpServeText(mcpBody))\n          }}\n",
            kotlin_quoted(&interview.arm.to_string()),
            kotlin_quoted(operation.doc.as_deref().unwrap_or_default()),
        );
    }
    if shape.error_header_out.is_empty() {
        let written = read_serializer(error);
        return format!(
            "          is {result}.Declared -> {prefix}McpDone(null, {prefix}McpServeText(Json.encodeToJsonElement({written}, answered.error)))\n"
        );
    }
    let extra = header_elements(&shape.error_header_out, error, "errorHeaderOut");
    let written = read_serializer(body_type(shape.error_header_out.len(), error));
    let puts = header_puts(prefix, &extra, "answered.error");
    format!(
        "          is {result}.Declared -> {{\n            \
         val members = mutableMapOf<String, JsonElement>(\"value\" to Json.encodeToJsonElement({written}, answered.error.error))\n\
{puts}            \
         {prefix}McpDone(null, {prefix}McpServeText(JsonObject(members)))\n          }}\n"
    )
}

/// One statement per header written beside the value; a `null` one is left out.
fn header_puts(prefix: &str, extra: &[HeaderElement], root: &str) -> String {
    let mut text = String::new();
    for field in extra {
        let accessor = format!("{root}.{}", field.kotlin_prop);
        let name = kotlin_quoted(&field.wire_name);
        if let Some(inner) = option_inner(&field.ty) {
            let _ = writeln!(
                text,
                "            {accessor}?.let {{ {prefix}McpPut(members, {name}, Json.encodeToJsonElement({}, it)) }}",
                read_serializer(inner)
            );
        } else {
            let _ = writeln!(
                text,
                "            {prefix}McpPut(members, {name}, Json.encodeToJsonElement({}, {accessor}))",
                read_serializer(&field.ty)
            );
        }
    }
    text
}
