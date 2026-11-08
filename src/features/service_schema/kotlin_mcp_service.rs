//! The Kotlin `mcp` server: the catalog of one service, the route that answers its `tools/call`,
//! the function that answers one JSON-RPC message for the services of an endpoint, and the
//! dispatchers over stdio-style text and over plain HTTP terms. It answers the bytes the Rust
//! dispatcher answers.

use super::kotlin_http_client::{kotlin_held, read_serializer};
use super::kotlin_ws_client::{HeaderElement, body_type, header_elements, message_serializer};
use super::result::result_name;
use crate::features::kotlin::{kotlin_declarations, kotlin_name, kotlin_quoted};
use crate::rename_rule::RenameRule;
use crate::service_schema::mcp_tools::wrapped_ident;
use crate::service_schema::parse::{
    HttpShape, OperationDef, OperationOutcome, ServiceDef, is_unit_type, option_inner,
};
use crate::service_schema::support::fault_fields_typescript_name;
use core::fmt::Write as _;
use proc_macro2::TokenStream;
use quote::quote;
use syn::Type;

const CATALOG: &str = r#"const val @P@McpServerName = "@N@"

val @P@McpInstructions: String? = @MCP_INSTRUCTIONS@

val @P@McpCatalog: Map<String, List<String>> = mapOf<String, List<String>>(
  "names" to listOf<String>(@MCP_NAMES@),
  "tools" to listOf<String>(@MCP_TOOLS@),
  "resources" to listOf<String>(),
  "resourceTemplates" to listOf<String>(),
  "prompts" to listOf<String>(),
  "skills" to listOf<String>(),
  "capabilities" to listOf<String>("{\"tools\":{}}"),
)

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
  "server/discover", "tools/call", "tools/list",
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

private fun @P@McpNamed(params: JsonObject, method: String): String? {
  val written = when (method) {
    "tools/call" -> params["name"]
    "resources/read" -> params["uri"]
    else -> if (((params["argument"] as? JsonObject)?.get("name") as? JsonPrimitive)?.isString == true) {
      (params["ref"] as? JsonObject)?.get("uri")
    } else {
      null
    }
  }
  return (written as? JsonPrimitive)?.takeIf { it.isString }?.content
}

private fun @P@McpListingStart(request: JsonObject, services: Int): Result<Boolean> {
  val invalid = Result.failure<Boolean>(IllegalArgumentException("Invalid cursor"))
  val cursor = @P@McpParams(request)["cursor"]
  if (cursor == null || cursor is JsonNull) return Result.success(true)
  if (cursor !is JsonPrimitive || !cursor.isString) return invalid
  val written = cursor.content
  val colon = written.indexOf(':')
  if (colon < 0) return invalid
  val part = written.substring(0, colon).toIntOrNull()?.takeIf { it >= 0 } ?: return invalid
  return if (part >= services) invalid else Result.success(false)
}

private fun @P@McpListedResources(first: Boolean, endpoint: @N@McpEndpoint): JsonObject =
  buildJsonObject {
    put("cacheScope", "private")
    put("resultType", "complete")
    putJsonArray("resources") {
      if (first) {
        for (catalog in endpoint.catalogs) {
          for (text in catalog["resources"].orEmpty()) add(Json.parseToJsonElement(text))
        }
      }
    }
    put("ttlMs", 0)
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
    "resources/list" -> return @P@McpBegun.Routed(request)
    "tools/call", "resources/read", "completion/complete" -> {
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
    val listed: @N@McpAnswer = @P@McpListingStart(request, endpoint.catalogs.size).fold(
      { @N@McpAnswer.Answered(@P@McpListedResources(it, endpoint)) },
      { @N@McpAnswer.Refused(@P@McpErrorObject(-32602, "Invalid cursor", null)) },
    )
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

private fun <T> @P@McpReceive(tool: String, serializer: DeserializationStrategy<T>, source: JsonElement): T {
  try {
    return Json.decodeFromJsonElement(serializer, source)
  } catch (refused: IllegalArgumentException) {
    val unjudged = (serializer as? Function0<*>)?.invoke() as? DeserializationStrategy<*>
    val readable = unjudged != null && runCatching { Json.decodeFromJsonElement(unjudged, source) }.isSuccess
    val detail = refused.message ?: refused.toString()
    if (readable) @P@McpRefuse("failed-validation", tool, detail, @P@McpNamedField(detail))
    @P@McpRefuse("undeserializable-payload", tool, detail)
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

const ROUTE_HEAD: &str = r#"fun create@N@McpRoute(handlers: @N@Handlers): suspend (method: String, params: JsonObject) -> @N@McpAnswer? =
  { mcpMethod, mcpParams -> @P@McpCall(handlers, mcpMethod, mcpParams) }

private suspend fun @P@McpCall(handlers: @N@Handlers, mcpMethod: String, mcpParams: JsonObject): @N@McpAnswer? {
  if (mcpMethod != "tools/call") return null
  val mcpNamed = mcpParams["name"]
  if (mcpNamed !is JsonPrimitive || !mcpNamed.isString) return null
  try {
    return when (mcpNamed.content) {
"#;

const ROUTE_TAIL: &str = "      else -> null
    }
  } catch (mcpRefused: @P@McpRefused) {
    return mcpRefused.answer
  }
}";

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

/// Where the accessor writes the tools, as the comma-joined Kotlin literals of their JSON text.
pub const TOOLS_MARKER: &str = "@MCP_TOOLS@";
/// Where the accessor writes the names, as `kind:name` Kotlin literals.
pub const NAMES_MARKER: &str = "@MCP_NAMES@";
/// Where the accessor writes the instructions: a Kotlin literal or `null`.
pub const INSTRUCTIONS_MARKER: &str = "@MCP_INSTRUCTIONS@";
/// Where the accessor writes the version of the crate that declares the service.
pub const VERSION_MARKER: &str = "@MCP_VERSION@";

pub fn emit(declared: &ServiceDef) -> Vec<String> {
    let service = &declared.enveloped();
    let named = service.ident.to_string();
    let prefix = RenameRule::CamelCase.apply_to_variant(&named);
    let fields = fault_fields_typescript_name(&named);
    let mut published = vec![
        fill(CATALOG, &prefix, &named, &fields).replace("@WRAPPED_MAP@", &wrapped_entries(service)),
        fill(ENDPOINT, &prefix, &named, &fields),
        fill(PROTOCOL, &prefix, &named, &fields),
        fill(WRITER, &prefix, &named, &fields),
        fill(SERVING, &prefix, &named, &fields),
        fill(RECEIVING, &prefix, &named, &fields),
        route(service, &prefix, &named, &fields),
        fill(DISPATCHERS, &prefix, &named, &fields),
        fill(HTTP, &prefix, &named, &fields).replace("@SCOPES@", &scopes_table(service)),
    ];
    published.extend(kotlin_declarations());
    published
}

fn scopes_table(service: &ServiceDef) -> String {
    let cases: Vec<String> = service
        .operations
        .iter()
        .filter(|operation| operation.is_mcp_tool() && !operation.mcp.scopes.is_empty())
        .map(|operation| {
            let needed: Vec<String> = operation
                .mcp
                .scopes
                .iter()
                .map(|(scope, _)| kotlin_quoted(scope))
                .collect();
            format!(
                "      {} -> return listOf({})\n",
                kotlin_quoted(operation.mcp_tool_name()),
                needed.join(", ")
            )
        })
        .collect();
    if cases.is_empty() {
        return String::new();
    }
    format!(
        "  if (method == \"tools/call\") {{\n    when (name) {{\n{}    }}\n  }}\n",
        cases.concat()
    )
}

/// One `"tool" to <marker>,` entry per tool: the accessor writes `true` or `false` over each
/// marker, from `wrapped_{operation}()`.
pub(super) fn wrapped_entries(service: &ServiceDef) -> String {
    service
        .operations
        .iter()
        .enumerate()
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
            let instructions = Self::MCP_INSTRUCTIONS.map_or_else(|| "null".to_owned(), literal);
            let mut text = #text
                .replace(#TOOLS_MARKER, &tools)
                .replace(#NAMES_MARKER, &names)
                .replace(#INSTRUCTIONS_MARKER, &instructions)
                .replace(#VERSION_MARKER, env!("CARGO_PKG_VERSION"));
            #(#wrapped)*
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

fn route(service: &ServiceDef, prefix: &str, named: &str, fields: &str) -> String {
    let arms = service
        .operations
        .iter()
        .map(|operation| arm(operation, prefix, named, fields))
        .collect::<Vec<_>>()
        .join("\n");
    let head = fill(ROUTE_HEAD, prefix, named, fields);
    format!("{head}{arms}\n{ROUTE_TAIL}")
        .replace("@P@", prefix)
        .replace("@N@", named)
}

fn arm(operation: &OperationDef, prefix: &str, named: &str, fields: &str) -> String {
    let tool = kotlin_quoted(operation.mcp_tool_name());
    let shape = HttpShape::of(operation);
    let read_by = message_serializer(operation);
    let mut text = format!(
        "    {tool} -> {{\n      \
         val mcpArguments = {prefix}McpArguments({tool}, mcpParams)\n"
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
            let ok = ok_arm(&result, prefix, &shape, success);
            let declared = declared_arm(&result, prefix, &shape, error);
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
fn ok_arm(result: &str, prefix: &str, shape: &HttpShape, success: &Type) -> String {
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
fn declared_arm(result: &str, prefix: &str, shape: &HttpShape, error: &Type) -> String {
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
