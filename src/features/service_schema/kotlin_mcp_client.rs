//! The Kotlin `mcp` client: one `tools/call` per operation, sent through the caller's one-method
//! transport and read back as the sealed result `kotlin_http_client()` declares.

use super::kotlin_http_client::{carries_no_value, held_copies, kotlin_held, read_serializer};
use super::kotlin_mcp_service::{VERSION_MARKER, tool_entries, wrapped_replacements};
use super::kotlin_ws_client::{
    body_type, header_elements, kotlin_type_of, message_kotlin_typename, message_serializer,
};
use super::result::result_name;
use super::{left_to_http_rest, on_one_line};
use crate::features::kotlin::{
    TupleNaming, kotlin_declarations, kotlin_name, kotlin_property_name, kotlin_quoted,
    kotlin_tuple_naming,
};
use crate::rename_rule::RenameRule;
use crate::service_schema::mcp_tools::{is_content_list, is_content_pair, is_prompt_messages};
use crate::service_schema::parse::{
    HttpShape, OperationDef, OperationInputs, OperationOutcome, ServiceDef, option_inner, wire_key,
    written,
};
use crate::service_schema::support::fault_fields_typescript_name;
use core::fmt::Write as _;
use proc_macro2::TokenStream;
use quote::quote;
use syn::Type;

const TRANSPORT: &str = "data class @N@McpMessage(val text: String, val protocolVersion: String, val method: String, val name: String?)

interface @N@McpTransport {
  suspend fun send(message: @N@McpMessage): String
}";

const REFUSAL: &str = "class @N@McpRefusal(val fault: @F@) : Exception(fault.detail)";

const CALL_WRAPPED: &str =
    "private val @P@McpCallWrapped: Map<String, Boolean> = mapOf<String, Boolean>(
  @WRAPPED_MAP@
)";

const SUPPORT: &str = r#"private sealed interface @P@McpCalled {
  class Succeeded(val structured: JsonElement?, val content: JsonElement?) : @P@McpCalled
  class Declared(val error: JsonElement) : @P@McpCalled
  class Faulted(val fault: @F@) : @P@McpCalled
}

private fun @P@McpCallFault(kind: @N@FaultKind, operation: String, detail: String): @F@ =
  @F@(detail = detail, kind = kind, operation = operation)

private suspend fun @P@McpCallExchange(
  transport: @N@McpTransport,
  id: Long,
  method: String,
  tool: String,
  arguments: JsonObject,
): @P@McpCalled {
  val request = buildJsonObject {
    put("id", id)
    put("jsonrpc", "2.0")
    put("method", method)
    put(
      "params",
      buildJsonObject {
        put(
          "_meta",
          buildJsonObject {
            put("io.modelcontextprotocol/clientCapabilities", buildJsonObject {})
            put(
              "io.modelcontextprotocol/clientInfo",
              buildJsonObject {
                put("name", "@N@McpClient")
                put("version", "@MCP_VERSION@")
              },
            )
            put("io.modelcontextprotocol/protocolVersion", "2026-07-28")
          },
        )
        put("arguments", arguments)
        put("name", tool)
      },
    )
  }
  val answer = try {
    transport.send(@N@McpMessage(request.toString(), "2026-07-28", method, tool))
  } catch (uncarried: Exception) {
    if (uncarried is CancellationException) throw uncarried
    return @P@McpCalled.Faulted(@P@McpCallFault(@N@FaultKind.TransportFailure, tool, uncarried.toString()))
  }
  val unreadable = { detail: String ->
    @P@McpCalled.Faulted(@P@McpCallFault(@N@FaultKind.UndeserializablePayload, tool, detail))
  }
  val parsed = try {
    Json.parseToJsonElement(answer)
  } catch (rejected: Exception) {
    return unreadable(rejected.message ?: rejected.toString())
  }
  val body = parsed as? JsonObject ?: return unreadable("the answer is not a JSON object")
  val error = body["error"]
  if (error != null) {
    val data = (error as? JsonObject)?.get("data") as? JsonObject
    if (data != null && "isServiceFault" in data) return @P@McpCallReported(tool, data)
    val errorObject = error as? JsonObject
    val carried = errorObject?.get("data")
    val code = (errorObject?.get("code") as? JsonPrimitive)?.content
    if (method == "prompts/get" && code == "-32602" && carried != null && carried !is JsonNull) {
      return @P@McpCalled.Declared(carried)
    }
    val said = (error as? JsonObject)?.get("message")?.let { (it as? JsonPrimitive)?.content }.orEmpty()
    return @P@McpCalled.Faulted(
      @P@McpCallFault(@N@FaultKind.TransportFailure, tool, "${(error as? JsonObject)?.get("code")}: $said"),
    )
  }
  val result = body["result"] as? JsonObject
    ?: return unreadable("the answer holds neither a result nor an error")
  if (method == "prompts/get") return @P@McpCalled.Succeeded(result["messages"], null)
  val flagged = result["isError"]
  if (!(flagged is JsonPrimitive && !flagged.isString && flagged.content == "true")) {
    return @P@McpCalled.Succeeded(result["structuredContent"], result["content"])
  }
  val text = (((result["content"] as? JsonArray)?.firstOrNull() as? JsonObject)?.get("text") as? JsonPrimitive)
    ?.takeIf { it.isString }?.content
    ?: return unreadable("the error holds no text")
  val carried = try {
    Json.parseToJsonElement(text)
  } catch (rejected: Exception) {
    return unreadable(rejected.message ?: rejected.toString())
  }
  if (carried is JsonObject && "isServiceFault" in carried) return @P@McpCallReported(tool, carried)
  return @P@McpCalled.Declared(carried)
}

private fun @P@McpCallReported(tool: String, tagged: JsonObject): @P@McpCalled.Faulted =
  try {
    @P@McpCalled.Faulted(Json.decodeFromJsonElement(serializer<@F@>(), tagged.getValue("fault")))
  } catch (rejected: Exception) {
    @P@McpCalled.Faulted(
      @P@McpCallFault(@N@FaultKind.UndeserializablePayload, tool, rejected.message ?: rejected.toString()),
    )
  }"#;

const PAIR: &str = r#"/** [element] read as the type the caller expects, which is the pair `kotlin_http_client()` declares. */
private inline fun <reified Value> @P@McpCallDecode(element: JsonElement): Value = Json.decodeFromJsonElement(element)

/** The bytes and the content type an answer's one image or audio block holds, written as the structured pair. */
private fun @P@McpCallPair(content: JsonElement?): JsonElement? {
  val block = (content as? JsonArray)?.firstOrNull() as? JsonObject ?: return null
  val data = (block["data"] as? JsonPrimitive)?.takeIf { it.isString }?.content ?: return null
  val type = (block["mimeType"] as? JsonPrimitive)?.takeIf { it.isString }?.content ?: return null
  val bytes = try {
    java.util.Base64.getDecoder().decode(data)
  } catch (rejected: IllegalArgumentException) {
    return null
  }
  return buildJsonArray {
    add(buildJsonArray { for (byte in bytes) add(byte.toInt() and 0xff) })
    add(type)
  }
}"#;

pub fn emit(declared: &ServiceDef) -> Vec<String> {
    kotlin_tuple_naming(TupleNaming::Named);
    let service = &declared.enveloped();
    let named = service.ident.to_string();
    let prefix = RenameRule::CamelCase.apply_to_variant(&named);
    let fields = fault_fields_typescript_name(&named);
    let mut published = vec![fill(TRANSPORT, &prefix, &named, &fields)];
    if service
        .operations
        .iter()
        .any(|operation| matches!(operation.outcome, OperationOutcome::OneWay))
    {
        published.push(fill(REFUSAL, &prefix, &named, &fields));
    }
    if service.operations.iter().any(OperationDef::is_mcp_tool) {
        published.push(
            fill(CALL_WRAPPED, &prefix, &named, &fields)
                .replace("@WRAPPED_MAP@", &tool_entries(service)),
        );
    }
    published.push(fill(SUPPORT, &prefix, &named, &fields));
    if service.operations.iter().any(is_content_pair) {
        published.push(fill(PAIR, &prefix, &named, &fields));
    }
    published.push(client(service, &prefix, &named));
    published.extend(kotlin_declarations());
    published
}

/// `kotlin_mcp_client()` on the service's registry: the text above, with the wrapping of each
/// message and the crate's version, which only the compiled service can give, written over their
/// markers when it runs in the consuming program.
pub fn accessor(service: &ServiceDef) -> TokenStream {
    let text = emit(service).join("\n\n");
    let wrapped = wrapped_replacements(service);
    quote! {
        #[doc = " The service's generated Kotlin `mcp` client: the one-method transport, the client"]
        #[doc = " class answering `kotlin_http_client()`'s own sealed results, and the one-way refusal."]
        pub fn kotlin_mcp_client() -> ::std::string::String {
            let mut text = #text.replace(#VERSION_MARKER, env!("CARGO_PKG_VERSION"));
            #(#wrapped)*
            text
        }
    }
}

fn fill(template: &str, prefix: &str, named: &str, fields: &str) -> String {
    template
        .replace("@P@", prefix)
        .replace("@N@", named)
        .replace("@F@", fields)
}

fn client(service: &ServiceDef, prefix: &str, named: &str) -> String {
    let note = left_to_http_rest(service, "method", on_one_line)
        .map_or_else(String::new, |said| format!("// {said}\n"));
    let methods = service
        .operations
        .iter()
        .filter(|operation| operation.is_mcp_tool() || operation.is_mcp_prompt())
        .map(|operation| method(operation, prefix, named))
        .collect::<Vec<_>>()
        .join("\n\n");
    format!(
        "// A `{named}` caller over `mcp`.\n{note}\
         class {named}McpClient(private val transport: {named}McpTransport) {{\n  \
         private var nextId = 0L\n\n{methods}\n}}"
    )
}

fn parameters(operation: &OperationDef, shape: &HttpShape) -> String {
    let mut params = vec![format!("req: {}", message_kotlin_typename(operation))];
    for header in &shape.header_in {
        params.push(format!(
            "{}: {}",
            kotlin_property_name(&written(&header.parameter)),
            kotlin_type_of(&header.ty)
        ));
    }
    params.join(", ")
}

/// The `arguments` map of the call: the message flat or under `value`, then each header argument
/// under its header's name, a `null` one left out.
fn arguments_stmt(
    operation: &OperationDef,
    shape: &HttpShape,
    prefix: &str,
    faulted: &str,
) -> String {
    let tool = kotlin_quoted(operation.mcp_tool_name());
    let read_by = message_serializer(operation);
    if operation.is_mcp_prompt() {
        let single = matches!(operation.inputs, OperationInputs::Named(_)).then(|| {
            let key = operation
                .mcp
                .prompt
                .iter()
                .filter_map(|prompt| prompt.arguments.first())
                .map(|(argument, _)| wire_key(argument))
                .next()
                .unwrap_or_default();
            format!(
                "      if (message is JsonPrimitive && message.isString) sent[{}] = message\n",
                kotlin_quoted(&key)
            )
        });
        return format!(
            "    val arguments = try {{\n      \
             val message = Json.encodeToJsonElement({read_by}, req)\n      \
             val sent = mutableMapOf<String, JsonElement>()\n      \
             if (message is JsonObject) {{\n        \
             for (member in message) if (member.value is JsonPrimitive && (member.value as JsonPrimitive).isString) sent[member.key] = member.value\n      \
             }}\n{}      \
             JsonObject(sent)\n    }} catch (rejected: Exception) {{\n      \
             {faulted}\n    }}\n",
            single.unwrap_or_default()
        );
    }
    let mut text = format!(
        "    val arguments = try {{\n      \
         val message = Json.encodeToJsonElement({read_by}, req)\n      \
         val sent = if ({prefix}McpCallWrapped.getValue({tool})) mutableMapOf(\"value\" to message) \
         else (message as? JsonObject)?.toMutableMap() ?: mutableMapOf()\n"
    );
    for header in &shape.header_in {
        let held = kotlin_held(&header.parameter);
        let name = kotlin_quoted(&header.name);
        if let Some(inner) = option_inner(&header.ty) {
            let _ = writeln!(
                text,
                "      {held}?.let {{ sent[{name}] = Json.encodeToJsonElement({}, it) }}",
                read_serializer(inner)
            );
        } else {
            let _ = writeln!(
                text,
                "      sent[{name}] = Json.encodeToJsonElement({}, {held})",
                read_serializer(&header.ty)
            );
        }
    }
    let _ = write!(
        text,
        "      JsonObject(sent)\n    }} catch (rejected: Exception) {{\n      \
         {faulted}\n    }}\n"
    );
    text
}

fn method(operation: &OperationDef, prefix: &str, named: &str) -> String {
    let shape = HttpShape::of(operation);
    let tool = kotlin_quoted(operation.mcp_tool_name());
    let call = kotlin_name(&operation.ts_name);
    let params = parameters(operation, &shape);
    let copies = held_copies(shape.header_in.iter().map(|header| &header.parameter));
    let doc = format!(
        "  // Calls `{}` over `mcp`.",
        on_one_line(operation.mcp_tool_name())
    );
    let failure = format!(
        "{prefix}McpCallFault({named}FaultKind.FailedValidation, {tool}, rejected.message ?: rejected.toString())"
    );
    let verb = if operation.is_mcp_prompt() {
        "prompts/get"
    } else {
        "tools/call"
    };
    let exchange = format!(
        "    val outcome = {prefix}McpCallExchange(this.transport, synchronized(this) {{ ++this.nextId }}, \"{verb}\", {tool}, arguments)\n"
    );
    match &operation.outcome {
        OperationOutcome::OneWay => {
            let arguments = arguments_stmt(
                operation,
                &shape,
                prefix,
                &format!("throw {named}McpRefusal({failure})"),
            );
            format!(
                "{doc}\n  suspend fun {call}({params}) {{\n{copies}{arguments}{exchange}    \
                 when (outcome) {{\n      \
                 is {prefix}McpCalled.Faulted -> throw {named}McpRefusal(outcome.fault)\n      \
                 is {prefix}McpCalled.Declared -> throw {named}McpRefusal(\n        \
                 {prefix}McpCallFault({named}FaultKind.UndeserializablePayload, {tool}, \"a one-way operation answered an error\"),\n      \
                 )\n      \
                 is {prefix}McpCalled.Succeeded -> Unit\n    }}\n  }}"
            )
        }
        OperationOutcome::Reply { error, success } => {
            let result = result_name(named, operation).unwrap_or_default();
            let arguments = arguments_stmt(
                operation,
                &shape,
                prefix,
                &format!("return {result}.Fault({failure})"),
            );
            let succeeded = succeeded_read(operation, &shape, &result, prefix, named, success);
            let declared = declared_read(&shape, &result, prefix, named, &tool, error);
            format!(
                "{doc}\n  suspend fun {call}({params}): {result} {{\n{copies}{arguments}{exchange}    \
                 return when (outcome) {{\n      \
                 is {prefix}McpCalled.Faulted -> {result}.Fault(outcome.fault)\n\
{declared}{succeeded}    }}\n  }}"
            )
        }
    }
}

/// The text of a `try` that reads, and the catch that answers an unreadable value.
fn guarded(read: &str, result: &str, prefix: &str, named: &str, tool: &str) -> String {
    format!(
        "try {{\n{read}        }} catch (rejected: Exception) {{\n        \
         {result}.Fault({prefix}McpCallFault({named}FaultKind.UndeserializablePayload, {tool}, \
         rejected.message ?: rejected.toString()))\n      }}"
    )
}

fn succeeded_read(
    operation: &OperationDef,
    shape: &HttpShape,
    result: &str,
    prefix: &str,
    named: &str,
    success: &Type,
) -> String {
    let tool = kotlin_quoted(operation.mcp_tool_name());
    if carries_no_value(operation, shape) {
        return format!("      is {prefix}McpCalled.Succeeded -> {result}.Ok\n");
    }
    let read = if operation.is_mcp_prompt() {
        if is_prompt_messages(operation) {
            format!(
                "          {result}.Ok(Json.decodeFromJsonElement({}, outcome.structured ?: JsonNull))\n",
                read_serializer(success)
            )
        } else {
            format!(
                "          {result}.Ok(\n            ((((outcome.structured as? JsonArray)?.firstOrNull() as? JsonObject)?.get(\"content\") as? JsonObject)\n              \
                 ?.get(\"text\") as? JsonPrimitive)?.takeIf {{ it.isString }}?.content\n              \
                 ?: throw SerializationException(\"the first message holds no text\"),\n          )\n"
            )
        }
    } else if shape.header_out.is_empty() {
        if is_content_pair(operation) {
            format!(
                "          {result}.Ok({prefix}McpCallDecode(outcome.structured ?: {prefix}McpCallPair(outcome.content) ?: JsonNull))\n"
            )
        } else {
            let source = if is_content_list(operation) {
                "outcome.content"
            } else {
                "outcome.structured"
            };
            format!(
                "          {result}.Ok(Json.decodeFromJsonElement({}, {source} ?: JsonNull))\n",
                read_serializer(success)
            )
        }
    } else {
        let extra = header_elements(&shape.header_out, success, "headerOut");
        let mut text = format!(
            "          val members = outcome.structured as? JsonObject\n            \
             ?: throw SerializationException(\"the answer holds no object\")\n          \
             val value = Json.decodeFromJsonElement({}, members[\"value\"] ?: JsonNull)\n",
            read_serializer(body_type(shape.header_out.len(), success))
        );
        let mut slots = vec!["value".to_owned()];
        for field in &extra {
            let name = kotlin_quoted(&field.wire_name);
            let prop = &field.kotlin_prop;
            if let Some(inner) = option_inner(&field.ty) {
                let _ = writeln!(
                    text,
                    "          val {prop} = members[{name}]?.takeIf {{ it !is JsonNull }}\
                     ?.let {{ Json.decodeFromJsonElement({}, it) }}",
                    read_serializer(inner)
                );
            } else {
                let _ = writeln!(
                    text,
                    "          val {prop} = Json.decodeFromJsonElement({}, members[{name}] ?: JsonNull)",
                    read_serializer(&field.ty)
                );
            }
            slots.push(prop.clone());
        }
        let _ = writeln!(
            text,
            "          {result}.Ok({result}Value({}))",
            slots.join(", ")
        );
        text
    };
    format!(
        "      is {prefix}McpCalled.Succeeded -> {}\n",
        guarded(&read, result, prefix, named, &tool)
    )
}

fn declared_read(
    shape: &HttpShape,
    result: &str,
    prefix: &str,
    named: &str,
    tool: &str,
    error: &Type,
) -> String {
    let read = if shape.error_header_out.is_empty() {
        format!(
            "          {result}.Declared(Json.decodeFromJsonElement({}, outcome.error))\n",
            read_serializer(error)
        )
    } else {
        let extra = header_elements(&shape.error_header_out, error, "errorHeaderOut");
        let mut text = format!(
            "          val members = outcome.error as? JsonObject\n            \
             ?: throw SerializationException(\"the error holds no object\")\n          \
             val declaredHead = Json.decodeFromJsonElement({}, members[\"value\"] ?: JsonNull)\n",
            read_serializer(body_type(shape.error_header_out.len(), error))
        );
        let mut slots = vec!["declaredHead".to_owned()];
        for field in &extra {
            let name = kotlin_quoted(&field.wire_name);
            let prop = &field.kotlin_prop;
            if let Some(inner) = option_inner(&field.ty) {
                let _ = writeln!(
                    text,
                    "          val {prop} = members[{name}]?.takeIf {{ it !is JsonNull }}\
                     ?.let {{ Json.decodeFromJsonElement({}, it) }}",
                    read_serializer(inner)
                );
            } else {
                let _ = writeln!(
                    text,
                    "          val {prop} = Json.decodeFromJsonElement({}, members[{name}] ?: JsonNull)",
                    read_serializer(&field.ty)
                );
            }
            slots.push(prop.clone());
        }
        let _ = writeln!(
            text,
            "          {result}.Declared({result}Declared({}))",
            slots.join(", ")
        );
        text
    };
    format!(
        "      is {prefix}McpCalled.Declared -> {}\n",
        guarded(&read, result, prefix, named, tool)
    )
}
