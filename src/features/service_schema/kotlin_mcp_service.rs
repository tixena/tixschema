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

private fun @P@McpListedTools(endpoint: @N@McpEndpoint): JsonObject =
  buildJsonObject {
    put("cacheScope", "public")
    put("resultType", "complete")
    putJsonArray("tools") {
      for (catalog in endpoint.catalogs) {
        for (text in catalog["tools"].orEmpty()) add(Json.parseToJsonElement(text))
      }
    }
    put("ttlMs", @P@McpListedForMs)
  }"#;

const SERVING: &str = r#"private sealed interface @P@McpBegun {
  class Answered(val text: String?) : @P@McpBegun
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
  val id = request["id"] ?: return @P@McpBegun.Answered(null)
  val unchecked = @P@McpChecked(request)
  if (unchecked != null) return @P@McpBegun.Answered(@P@McpServeText(unchecked))
  val method = (request["method"] as JsonPrimitive).content
  val answered: @N@McpAnswer = when (method) {
    "server/discover" -> @N@McpAnswer.Answered(@P@McpDiscovered(instructions, endpoint))
    "tools/list" -> @N@McpAnswer.Answered(@P@McpListedTools(endpoint))
    "tools/call" -> {
      val named = @P@McpParams(request)["name"]
      if (named is JsonPrimitive && named.isString) return @P@McpBegun.Routed(request)
      @N@McpAnswer.Refused(@P@McpErrorObject(-32602, "Invalid params: tools/call names no tool", null))
    }
    else -> @N@McpAnswer.Refused(@P@McpErrorObject(-32601, "Method not found: $method", null))
  }
  return @P@McpBegun.Answered(@P@McpResponseText(id, answered, serverName))
}

private fun @P@McpRoutedText(request: JsonObject, answered: @N@McpAnswer?, serverName: String): String {
  val name = ((@P@McpParams(request)["name"]) as JsonPrimitive).content
  val resolved = answered ?: @N@McpAnswer.Refused(
    @P@McpErrorObject(
      -32602,
      "Unknown tool: $name",
      buildJsonObject {
        put("fault", @P@McpFaultObject("unknown-operation", name, "the service answers to no operation by that name"))
        put("isServiceFault", true)
      },
    ),
  )
  return @P@McpResponseText(request["id"] ?: JsonNull, resolved, serverName)
}

suspend fun @P@McpServe(
  text: String,
  serverName: String,
  instructions: String?,
  endpoint: @N@McpEndpoint,
  route: suspend (part: Int, method: String, params: JsonObject) -> @N@McpAnswer?,
): String? {
  val request = when (val begun = @P@McpBegin(text, instructions, endpoint, serverName)) {
    is @P@McpBegun.Answered -> return begun.text
    is @P@McpBegun.Routed -> begun.request
  }
  var answered: @N@McpAnswer? = null
  for (part in endpoint.catalogs.indices) {
    answered = route(part, "tools/call", @P@McpParams(request))
    if (answered != null) break
  }
  return @P@McpRoutedText(request, answered, serverName)
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

const DISPATCHERS: &str = "fun create@N@McpDispatcher(handlers: @N@Handlers): suspend (text: String) -> String? {
  val route = create@N@McpRoute(handlers)
  val endpoint = @N@McpEndpoint(@P@McpCatalog)
  return { text ->
    @P@McpServe(text, @P@McpServerName, @P@McpInstructions, endpoint) { _, method, params -> route(method, params) }
  }
}";

const HTTP: &str = r#"data class @N@McpHttpRequest(val method: String, val headers: List<Pair<String, String>>, val body: ByteArray) {
  fun header(name: String): String? = headers.firstOrNull { it.first.equals(name, ignoreCase = true) }?.second
}

data class @N@McpHttpResponse(val status: Int, val headers: List<Pair<String, String>>, val body: ByteArray)

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

private fun @P@McpAdmitted(request: @N@McpHttpRequest, allowedOrigins: List<String>): Pair<String?, @N@McpHttpResponse?> {
  if (request.method != "POST") return null to @P@McpWithoutBody(405, listOf("Allow" to "POST"))
  val origin = request.header("origin")
  if (origin != null && origin !in allowedOrigins) return null to @P@McpWithoutBody(403)
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
  if (method != "server/discover" && method != "tools/call" && method != "tools/list") {
    return null to @P@McpJsonResponse(
      404,
      @P@McpFailed(message["id"], @P@McpErrorObject(-32601, "Method not found: $method", null)),
    )
  }
  return text to null
}

private fun @P@McpHttpAnswer(answered: String?): @N@McpHttpResponse =
  if (answered == null) {
    @P@McpWithoutBody(202)
  } else {
    @N@McpHttpResponse(200, listOf("Content-Type" to "application/json"), answered.encodeToByteArray())
  }

suspend fun @P@McpServeHttp(
  request: @N@McpHttpRequest,
  allowedOrigins: List<String>,
  serverName: String,
  instructions: String?,
  endpoint: @N@McpEndpoint,
  route: suspend (part: Int, method: String, params: JsonObject) -> @N@McpAnswer?,
): @N@McpHttpResponse {
  val (text, refused) = @P@McpAdmitted(request, allowedOrigins)
  if (text == null) return refused ?: @P@McpWithoutBody(400)
  return @P@McpHttpAnswer(@P@McpServe(text, serverName, instructions, endpoint, route))
}

fun create@N@McpHttpDispatcher(
  handlers: @N@Handlers,
  allowedOrigins: List<String>,
): suspend (@N@McpHttpRequest) -> @N@McpHttpResponse {
  val route = create@N@McpRoute(handlers)
  val endpoint = @N@McpEndpoint(@P@McpCatalog)
  return { request ->
    @P@McpServeHttp(request, allowedOrigins, @P@McpServerName, @P@McpInstructions, endpoint) { _, method, params ->
      route(method, params)
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
        fill(SERVING, &prefix, &named, &fields),
        fill(RECEIVING, &prefix, &named, &fields),
        route(service, &prefix, &named, &fields),
        fill(DISPATCHERS, &prefix, &named, &fields),
        fill(HTTP, &prefix, &named, &fields),
    ];
    published.extend(kotlin_declarations());
    published
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
