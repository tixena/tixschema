//! The TypeScript `mcp` client: a seam that sends one JSON-RPC request and answers the text of
//! the answer, and one method per operation answering the result type its other clients answer.

use super::client::{issues_fault_fn, method_params, outbound_refusal};
use super::fault;
use super::mcp_ts_service::{VERSION_MARKER, filled, wrapped_marker};
use super::message;
use super::result::result_name;
use crate::field_type::get_field_def;
use crate::rename_rule::RenameRule;
use crate::service_schema::mcp_tools::{is_content_list, is_content_pair, is_prompt_messages};
use crate::service_schema::parse::{
    BodyKind, HttpShape, OperationDef, OperationInputs, OperationOutcome, ServiceDef, is_unit_type,
    wire_key,
};
use crate::utils::ts_quoted;
use core::fmt::Write as _;
use core::iter::once;
use syn::Type;

const TRANSPORT: &str = "/**
 * What binds a `__N__` MCP client to a server: one JSON-RPC request in, the text of its answer
 * out. `protocolVersion`, `method` and `name` are the values of the `MCP-Protocol-Version`,
 * `Mcp-Method` and `Mcp-Name` headers an HTTP seam writes; a stdio seam writes `text` alone.
 */
export type __N__McpTransport = {
  send(message: {
    text: string;
    protocolVersion: string;
    method: string;
    name: string | undefined;
  }): Promise<string>;
};";

const FAULTS: &str = r#"function __p__Object(value: unknown): Record<string, unknown> | undefined {
  return typeof value === "object" && value !== null && !Array.isArray(value)
    ? (value as Record<string, unknown>)
    : undefined;
}

/** `value` as the members of an object, none where it is no object. */
function __p__Held(value: unknown): Record<string, unknown> {
  return __p__Object(value) ?? {};
}"#;

const ARGUMENTS: &str = "/** The arguments object of a call: the message itself, or under `value`, with each header beside it. */
function __p__Arguments(
  sent: unknown,
  wrapped: boolean,
  extra: ReadonlyArray<readonly [string, unknown]>,
): Record<string, unknown> {
  const given: Record<string, unknown> = wrapped ? { value: sent } : { ...__p__Held(sent) };
  for (const [name, held] of extra) given[name] = held;
  return given;
}";

const ISSUES: &str = r#"function __p__Issues(issues: ReadonlyArray<{ path: ReadonlyArray<PropertyKey>; message: string }>): string {
  return issues
    .map((issue) => (issue.path.length === 0 ? issue.message : `'${issue.path.join(".")}': ${issue.message}`))
    .join("; ");
}"#;

const CALL: &str = r#"/** What one call came to, before the operation's own schemas read it. */
type __N__McpCalled =
  | { readonly kind: "success"; readonly structured: unknown; readonly blocks: unknown }
  | { readonly kind: "fault"; readonly fault: __N__Fault }
  | { readonly kind: "declared"; readonly error: unknown };

/** Sends one `tools/call` and sorts the answer into a success, a fault, or a declared error. */
async function __p__Call(
  transport: __N__McpTransport,
  id: number,
  operation: string,
  tool: string,
  args: unknown,
): Promise<__N__McpCalled> {
  const text = JSON.stringify({
    id,
    jsonrpc: "2.0",
    method: "tools/call",
    params: {
      _meta: {
        "io.modelcontextprotocol/clientCapabilities": {},
        "io.modelcontextprotocol/clientInfo": { name: "__N__", version: __p__Version },
        "io.modelcontextprotocol/protocolVersion": "2026-07-28",
      },
      arguments: args,
      name: tool,
    },
  });
  let answered: string;
  try {
    answered = await transport.send({ text, protocolVersion: "2026-07-28", method: "tools/call", name: tool });
  } catch (thrown) {
    const detail = thrown instanceof Error ? thrown.message : String(thrown);
    return { kind: "fault", fault: __p__TransportFailure(operation, detail) };
  }
  let read: unknown;
  try {
    read = __p__Read(answered);
  } catch (rejected) {
    return { kind: "fault", fault: __p__Undeserializable(operation, `the answer is no JSON: ${String(rejected)}`) };
  }
  const message = __p__Object(read);
  if (message === undefined) {
    return { kind: "fault", fault: __p__Undeserializable(operation, "the answer is no JSON object") };
  }
  const failure = __p__Object(message.error);
  if (failure !== undefined) {
    const data = __p__Object(failure.data);
    const sealed = data?.isServiceFault === true ? __p__FaultOf(data.fault) : undefined;
    if (sealed !== undefined) return { kind: "fault", fault: sealed };
    return {
      kind: "fault",
      fault: __p__TransportFailure(operation, `${String(failure.code)}: ${String(failure.message)}`),
    };
  }
  const result = __p__Object(message.result);
  if (result === undefined) {
    return { kind: "fault", fault: __p__Undeserializable(operation, "the answer holds neither a result nor an error") };
  }
  if (result.isError !== true) return { kind: "success", structured: result.structuredContent, blocks: result.content };
  const blocks: ReadonlyArray<unknown> = Array.isArray(result.content) ? result.content : [];
  const errorText = blocks
    .map((block) => __p__Object(block))
    .find((block) => block?.type === "text" && typeof block.text === "string")?.text;
  if (typeof errorText !== "string") {
    return { kind: "fault", fault: __p__Undeserializable(operation, "the error answer holds no text") };
  }
  let carried: unknown;
  try {
    carried = __p__Read(errorText);
  } catch (rejected) {
    return { kind: "fault", fault: __p__Undeserializable(operation, `the error text is no JSON: ${String(rejected)}`) };
  }
  if (__p__Object(carried)?.isServiceFault === true) {
    const sealed = __p__FaultOf(__p__Object(carried)?.fault);
    return {
      kind: "fault",
      fault: sealed ?? __p__Undeserializable(operation, "a fault did not match the expected shape"),
    };
  }
  return { kind: "declared", error: carried };
}"#;

const REPLY_HEADER: &str =
    "/** Reads one declared header off the members of an answer; an absent one reads as `null`. */
function __p__ReplyHeader<Parsed>(
  operation: string,
  received: Record<string, unknown>,
  name: string,
  schema: ZodType<Parsed>,
): { ok: true; value: Parsed } | { ok: false; fault: __N__Fault } {
  const carried = received[name];
  const parsed = schema.safeParse(carried === undefined ? null : carried);
  if (parsed.success) return { ok: true, value: parsed.data };
  return {
    ok: false,
    fault: __p__Undeserializable(operation, `'${name}': ${__p__Issues(parsed.error.issues)}`),
  };
}";

const UNBASE64: &str = r#"/** `text` decoded from base64 as the array of its bytes, `undefined` where it is none. */
function __p__Unbase64(text: unknown): Array<number> | undefined {
  if (typeof text !== "string") return undefined;
  try {
    return Array.from(Uint8Array.from(atob(text), (letter) => letter.charCodeAt(0)));
  } catch {
    return undefined;
  }
}"#;

const PAIR_SOURCE: &str = r#"/** The structured pair of an answer: its own, or its one image or audio block written as one. */
function __p__PairSource(structured: unknown, blocks: unknown): unknown {
  if (structured !== undefined) return structured;
  const block = Array.isArray(blocks) ? __p__Object(blocks[0]) : undefined;
  const bytes = __p__Unbase64(block?.data);
  return bytes === undefined || typeof block?.mimeType !== "string" ? undefined : [bytes, block.mimeType];
}"#;

const LIST: &str = r#"/** The blocks of `content`, each read into a `McpContent`. */
function __p__Blocks(content: unknown): { ok: true; value: Array<McpContent> } | { ok: false; detail: string } {
  if (!Array.isArray(content)) return { ok: false, detail: "the answer holds no content" };
  const text = (from: Record<string, unknown>, key: string): string => {
    const held = from[key];
    if (typeof held !== "string") throw new Error(`a block holds no \`${key}\``);
    return held;
  };
  const optional = (from: Record<string, unknown>, key: string): string | undefined => {
    const held = from[key];
    return typeof held === "string" ? held : undefined;
  };
  const bytes = (from: Record<string, unknown>, key: string): Uint8Array => {
    const decoded = __p__Unbase64(from[key]);
    if (decoded === undefined) throw new Error(`\`${key}\` is not base64`);
    return Uint8Array.from(decoded);
  };
  const read: Array<McpContent> = [];
  try {
    for (const entry of content as ReadonlyArray<unknown>) {
      const block = __p__Held(entry);
      switch (block.type) {
        case "text":
          read.push({ type: "text", text: text(block, "text") });
          break;
        case "image":
          read.push({ type: "image", data: bytes(block, "data"), mimeType: text(block, "mimeType") });
          break;
        case "audio":
          read.push({ type: "audio", data: bytes(block, "data"), mimeType: text(block, "mimeType") });
          break;
        case "resource_link":
          read.push({
            type: "resource_link",
            uri: text(block, "uri"),
            name: text(block, "name"),
            title: optional(block, "title"),
            description: optional(block, "description"),
            mimeType: optional(block, "mimeType"),
          });
          break;
        case "resource": {
          const resource = __p__Held(block.resource);
          read.push(
            resource.blob === undefined
              ? { type: "resource", uri: text(resource, "uri"), mimeType: optional(resource, "mimeType"), text: text(resource, "text") }
              : { type: "resource", uri: text(resource, "uri"), mimeType: optional(resource, "mimeType"), blob: bytes(resource, "blob") },
          );
          break;
        }
        default:
          throw new Error("a block of a type this client does not read");
      }
    }
  } catch (rejected) {
    return { ok: false, detail: rejected instanceof Error ? rejected.message : String(rejected) };
  }
  return { ok: true, value: read };
}"#;

const PROMPT: &str = r#"/** What one prompt request came to, before the operation's own schemas read it. */
type __N__McpPrompted =
  | { readonly kind: "success"; readonly messages: unknown }
  | { readonly kind: "fault"; readonly fault: __N__Fault }
  | { readonly kind: "declared"; readonly error: unknown };

/** The arguments of a `prompts/get`: the members of the message that are text, or the one argument the message is. */
function __p__PromptArguments(sent: unknown, key: string): Record<string, string> {
  if (typeof sent === "string") return { [key]: sent };
  const given: Record<string, string> = {};
  for (const [name, held] of Object.entries(__p__Held(sent))) {
    if (typeof held === "string") given[name] = held;
  }
  return given;
}

/** Sends one `prompts/get` and sorts the answer into the messages, a fault, or a declared error. */
async function __p__Prompt(
  transport: __N__McpTransport,
  id: number,
  operation: string,
  prompt: string,
  args: Record<string, string>,
): Promise<__N__McpPrompted> {
  const text = JSON.stringify({
    id,
    jsonrpc: "2.0",
    method: "prompts/get",
    params: {
      _meta: {
        "io.modelcontextprotocol/clientCapabilities": {},
        "io.modelcontextprotocol/clientInfo": { name: "__N__", version: __p__Version },
        "io.modelcontextprotocol/protocolVersion": "2026-07-28",
      },
      arguments: args,
      name: prompt,
    },
  });
  let answered: string;
  try {
    answered = await transport.send({ text, protocolVersion: "2026-07-28", method: "prompts/get", name: prompt });
  } catch (thrown) {
    const detail = thrown instanceof Error ? thrown.message : String(thrown);
    return { kind: "fault", fault: __p__TransportFailure(operation, detail) };
  }
  let read: unknown;
  try {
    read = __p__Read(answered);
  } catch (rejected) {
    return { kind: "fault", fault: __p__Undeserializable(operation, `the answer is no JSON: ${String(rejected)}`) };
  }
  const message = __p__Object(read);
  if (message === undefined) {
    return { kind: "fault", fault: __p__Undeserializable(operation, "the answer is no JSON object") };
  }
  const failure = __p__Object(message.error);
  if (failure !== undefined) {
    const data = __p__Object(failure.data);
    const sealed = data?.isServiceFault === true ? __p__FaultOf(data.fault) : undefined;
    if (sealed !== undefined) return { kind: "fault", fault: sealed };
    if (failure.code === -32602 && failure.data !== undefined && failure.data !== null && data?.isServiceFault !== true) {
      return { kind: "declared", error: failure.data };
    }
    return {
      kind: "fault",
      fault: __p__TransportFailure(operation, `${String(failure.code)}: ${String(failure.message)}`),
    };
  }
  const result = __p__Object(message.result);
  if (result === undefined) {
    return { kind: "fault", fault: __p__Undeserializable(operation, "the answer holds neither a result nor an error") };
  }
  return { kind: "success", messages: result.messages };
}"#;

const PROMPT_TEXT: &str = r#"/** The text of the first message of a prompt's result, `undefined` where it holds none. */
function __p__PromptText(messages: unknown): string | undefined {
  const first = Array.isArray(messages) ? __p__Object(messages[0]) : undefined;
  const text = __p__Object(first?.content)?.text;
  return typeof text === "string" ? text : undefined;
}"#;

const MESSAGES: &str = r#"/** The messages of a prompt's result, each read into a `McpPromptMessage`. */
function __p__Messages(
  messages: unknown,
): { ok: true; value: Array<McpPromptMessage> } | { ok: false; detail: string } {
  if (!Array.isArray(messages)) return { ok: false, detail: "the answer holds no messages" };
  const read: Array<McpPromptMessage> = [];
  for (const entry of messages as ReadonlyArray<unknown>) {
    const held = __p__Held(entry);
    const role = held.role;
    if (role !== "user" && role !== "assistant") {
      return { ok: false, detail: "a message of a role this client does not read" };
    }
    const block = __p__Blocks([held.content]);
    if (!block.ok) return block;
    const content = block.value[0];
    if (content === undefined) return { ok: false, detail: "a message holds no content" };
    read.push({ role, content });
  }
  return { ok: true, value: read };
}"#;

pub fn emit(declared: &ServiceDef) -> Vec<String> {
    let service = &declared.enveloped();
    let named = service.ident.to_string();
    let prefix = format!(
        "{}McpClient",
        RenameRule::CamelCase.apply_to_variant(&named)
    );
    let fill = |template: &str| filled(template, &named, &prefix);
    let replies = service
        .operations
        .iter()
        .any(|operation| !matches!(operation.outcome, OperationOutcome::OneWay));
    let one_way = service
        .operations
        .iter()
        .any(|operation| matches!(operation.outcome, OperationOutcome::OneWay));
    let header_out = service.operations.iter().any(|operation| {
        let shape = HttpShape::of(operation);
        !shape.header_out.is_empty() || !shape.error_header_out.is_empty()
    });
    let mut published = vec![
        fill(TRANSPORT),
        client_type(service, &named),
        format!("const {prefix}Version: string = {VERSION_MARKER};"),
        message::envelope_read_fn(&format!("{prefix}Read")),
        message::wire_fn(&format!("{prefix}Sent")),
        message::infinite_fn(&format!("{prefix}Infinite")),
        message::checked_fn(
            &format!("{prefix}Checked"),
            &format!("{prefix}Sent"),
            &format!("{prefix}Infinite"),
        ),
        issues_fault_fn(
            &named,
            &format!("{prefix}OutboundFault"),
            &format!(
                " * The fault a `{named}` MCP client answers with when the message it was about \
                 to send failed its\n \
                 * own schema. The operation never ran, and the transport was never reached."
            ),
        ),
        fill(FAULTS),
        minted_fn(&named, &prefix, "TransportFailure", "transport-failure"),
        minted_fn(
            &named,
            &prefix,
            "Undeserializable",
            "undeserializable-payload",
        ),
        fault_of_fn(&named, &prefix),
    ];
    if service.operations.iter().any(OperationDef::is_mcp_tool) {
        published.push(fill(ARGUMENTS));
        published.push(fill(CALL));
    }
    if service.operations.iter().any(OperationDef::is_mcp_prompt) {
        published.push(fill(PROMPT));
    }
    if service
        .operations
        .iter()
        .any(|operation| operation.is_mcp_prompt() && !is_prompt_messages(operation))
    {
        published.push(fill(PROMPT_TEXT));
    }
    if header_out {
        published.push(fill(REPLY_HEADER));
    }
    let messages = service.operations.iter().any(is_prompt_messages);
    if service
        .operations
        .iter()
        .any(|operation| operation.mcp.content.is_some())
        || messages
    {
        published.push(fill(UNBASE64));
    }
    if service.operations.iter().any(is_content_pair) {
        published.push(fill(PAIR_SOURCE));
    }
    if service.operations.iter().any(is_content_list) || messages {
        published.push(fill(LIST));
    }
    if messages {
        published.push(fill(MESSAGES));
    }
    if one_way {
        published.push(refusal_type(&named));
        published.push(refused_fn(&named, &prefix));
    }
    if replies {
        published.push(fill(ISSUES));
    }
    published.push(factory(service, &named, &prefix));
    published
}

/// The operations the client has a method for: the tools and the prompts.
fn served(service: &ServiceDef) -> impl Iterator<Item = &OperationDef> {
    service
        .operations
        .iter()
        .filter(|operation| operation.is_mcp_tool() || operation.is_mcp_prompt())
}

fn client_type(service: &ServiceDef, named: &str) -> String {
    let methods = served(service)
        .map(|operation| {
            format!(
                "{}\n  {}({}): Promise<{}>;",
                message::method_jsdoc(
                    &method_summary(named, operation),
                    &throws_clause(named, operation),
                    operation
                ),
                operation.ts_name,
                method_params(operation),
                result_name(named, operation).unwrap_or_else(|| "void".to_owned()),
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    format!(
        "/**\n \
         * A `{named}` caller over MCP: one method per operation, taking the message and then one \
         argument\n \
         * per `header_in` binding. A request-and-reply operation answers its own result type.\n \
         */\n\
         export type {named}McpClient = {{\n\
         {methods}\n\
         }};"
    )
}

fn method_summary(service: &str, operation: &OperationDef) -> String {
    let tool = message::commented(operation.mcp_tool_name());
    match &operation.outcome {
        OperationOutcome::OneWay => {
            format!("Calls the tool `{tool}` of `{service}`, which expects no reply.")
        }
        OperationOutcome::Reply {
            success: _success,
            error: _error,
        } => {
            format!("Calls the tool `{tool}` of `{service}` and waits for the answer.")
        }
    }
}

fn throws_clause(service: &str, operation: &OperationDef) -> String {
    match &operation.outcome {
        OperationOutcome::OneWay => format!(
            "   * @throws {{{service}McpRefusal}} when the message fails its own schema, or the \
             call is answered with a fault."
        ),
        OperationOutcome::Reply {
            success: _success,
            error: _error,
        } => String::new(),
    }
}

fn minted_fn(named: &str, prefix: &str, suffix: &str, kind: &str) -> String {
    format!(
        "/** A `{named}Fault` of kind `{kind}`. */\n\
         function {prefix}{suffix}(operation: string, detail: string): {named}Fault {{\n\
         {minted}\n\
         }}",
        minted = fault::minted(
            named,
            &format!("    detail,\n    field: undefined,\n    kind: \"{kind}\",\n    operation,")
        ),
    )
}

fn fault_of_fn(named: &str, prefix: &str) -> String {
    let members = "      detail,\n      field,\n      kind: kind as __N__FaultKind,\n      operation,\n      \
                   ...(listed === undefined ? {} : { issues: listed }),"
        .replace("__N__", named);
    format!(
        "/** The problems a fault lists, `undefined` where one of them is no problem. */\n\
         function {prefix}IssuesOf(value: unknown): Array<{named}FaultIssue> | undefined {{\n  \
         if (!Array.isArray(value)) return undefined;\n  \
         const listed: Array<{named}FaultIssue> = [];\n  \
         for (const entry of value) {{\n    \
         const held = {prefix}Object(entry);\n    \
         if (held === undefined) return undefined;\n    \
         const {{ detail, field }} = held;\n    \
         if (typeof detail !== \"string\" || (field !== undefined && typeof field !== \"string\")) return undefined;\n    \
         listed.push({{ detail, field }});\n  \
         }}\n  \
         return listed;\n\
         }}\n\n\
         /** A fault read back from the text of an answer, `undefined` where it is none. */\n\
         function {prefix}FaultOf(value: unknown): {named}Fault | undefined {{\n  \
         const held = {prefix}Object(value);\n  \
         if (held === undefined) return undefined;\n  \
         const {{ detail, field, issues, kind, operation }} = held;\n  \
         const listed = issues === undefined ? undefined : {prefix}IssuesOf(issues);\n  \
         const kinds: ReadonlyArray<unknown> = [\n    \
         \"failed-validation\",\n    \
         \"handler-panic\",\n    \
         \"transport-failure\",\n    \
         \"undeserializable-payload\",\n    \
         \"unknown-operation\",\n  \
         ];\n  \
         if (\n    \
         typeof detail === \"string\" &&\n    \
         (field === undefined || typeof field === \"string\") &&\n    \
         (issues === undefined || listed !== undefined) &&\n    \
         typeof kind === \"string\" &&\n    \
         kinds.includes(kind) &&\n    \
         typeof operation === \"string\"\n  \
         ) {{\n\
         {minted}\n  \
         }}\n  \
         return undefined;\n\
         }}",
        minted = fault::minted(named, &members),
    )
}

fn refusal_type(named: &str) -> String {
    format!(
        "/**\n \
         * What a one-way `{named}` MCP method throws when it cannot carry out its call. \
         `Promise<void>` has no\n \
         * failure arm, so the fault rides on the thrown error: narrow a caught error with \
         `\"fault\" in caught`.\n \
         */\n\
         export type {named}McpRefusal = Error & {{ fault: {named}Fault }};"
    )
}

fn refused_fn(named: &str, prefix: &str) -> String {
    format!(
        "function {prefix}Refused(fault: {named}Fault): {named}McpRefusal {{\n  \
         return Object.assign(\n    \
         new Error(`${{fault.kind}} in operation \\`${{fault.operation}}\\`: ${{fault.detail}}`),\n    \
         {{ fault }},\n  \
         );\n\
         }}"
    )
}

fn factory(service: &ServiceDef, named: &str, prefix: &str) -> String {
    let methods = served(service)
        .map(|operation| method(service, operation))
        .collect::<Vec<_>>()
        .join("\n");
    format!(
        "/** Binds a `{named}` client to a transport. */\n\
         export function create{named}McpClient(transport: {named}McpTransport): {named}McpClient {{\n  \
         let next = 0;\n  \
         return {{\n\
         {methods}\n  \
         }};\n\
         }}"
    )
    .replace("__p__", prefix)
}

fn method(service: &ServiceDef, operation: &OperationDef) -> String {
    let named = service.ident.to_string();
    let prefix = format!(
        "{}McpClient",
        RenameRule::CamelCase.apply_to_variant(&named)
    );
    let wire = ts_quoted(&operation.wire_name);
    let tool = ts_quoted(operation.mcp_tool_name());
    let shape = HttpShape::of(operation);
    let arguments = once("req".to_owned())
        .chain(
            shape
                .header_in
                .iter()
                .map(|header| message::parameter_name(&header.parameter)),
        )
        .collect::<Vec<_>>()
        .join(", ");
    let wrapped = match &operation.inputs {
        OperationInputs::Named(_) => wrapped_marker(operation),
        OperationInputs::Empty | OperationInputs::Generated(_) => "false".to_owned(),
    };
    let refusal = outbound_refusal(&prefix, operation, &wire, "validated.error.issues");
    let schema = message::schema(operation);
    if operation.is_mcp_prompt() {
        return prompt_method(&prefix, operation, &wire, &tool, &schema, &refusal);
    }
    let (header_checks, extra) = header_checks(&prefix, operation, &shape, &wire);
    let tail = match &operation.outcome {
        OperationOutcome::OneWay => {
            format!("      if (called.kind === \"fault\") throw {prefix}Refused(called.fault);")
        }
        OperationOutcome::Reply { error, success } => {
            reply_tail(&prefix, &wire, operation, &shape, error, success)
        }
    };
    format!(
        "    async {call}({arguments}) {{\n      \
         const validated = {prefix}Checked({schema}, req);\n      \
         if (!validated.success) {{\n{refusal}\n      \
         }}\n\
         {header_checks}      \
         const called = await {prefix}Call(transport, ++next, {wire}, {tool}, \
         {prefix}Arguments(validated.sent, {wrapped}, {extra}));\n\
         {tail}\n    \
         }},",
        call = operation.ts_name,
    )
}

/// One prompt's method: check the message, send `prompts/get` with its text members, and read the
/// messages.
fn prompt_method(
    prefix: &str,
    operation: &OperationDef,
    wire: &str,
    prompt: &str,
    schema: &str,
    refusal: &str,
) -> String {
    let key = operation
        .mcp
        .prompt
        .iter()
        .filter_map(|held| held.arguments.first())
        .map(|(argument, _)| wire_key(argument))
        .next()
        .unwrap_or_default();
    let OperationOutcome::Reply {
        error,
        success: _success,
    } = &operation.outcome
    else {
        return String::new();
    };
    let shape = HttpShape::of(operation);
    let fault_arm = |fault: &str| {
        format!("return {{ ok: false, error: {{ isServiceFault: true, fault: {fault} }} }};")
    };
    let mut tail = format!(
        "      if (called.kind === \"fault\") {{\n        {}\n      }}\n",
        fault_arm("called.fault")
    );
    tail.push_str(&declared_arm(prefix, wire, &shape, error, &fault_arm));
    if is_prompt_messages(operation) {
        let _ = write!(
            tail,
            "      const read = {prefix}Messages(called.messages);\n      \
             if (!read.ok) {{\n        {}\n      }}\n      \
             return {{ ok: true, value: read.value }};",
            fault_arm(&format!("{prefix}Undeserializable({wire}, read.detail)"))
        );
    } else {
        let _ = write!(
            tail,
            "      const text = {prefix}PromptText(called.messages);\n      \
             if (text === undefined) {{\n        {}\n      }}\n      \
             return {{ ok: true, value: text }};",
            fault_arm(&format!(
                "{prefix}Undeserializable({wire}, \"the first message holds no text\")"
            ))
        );
    }
    format!(
        "    async {call}(req) {{\n      \
         const validated = {prefix}Checked({schema}, req);\n      \
         if (!validated.success) {{\n{refusal}\n      \
         }}\n      \
         const called = await {prefix}Prompt(transport, ++next, {wire}, {prompt}, \
         {prefix}PromptArguments(validated.sent, {key}));\n\
         {tail}\n    \
         }},",
        call = operation.ts_name,
        key = ts_quoted(&key),
    )
}

/// The statements that check each `header_in` argument as the message is checked, and the name of
/// the list of those that go out beside it.
fn header_checks(
    prefix: &str,
    operation: &OperationDef,
    shape: &HttpShape,
    wire: &str,
) -> (String, &'static str) {
    if shape.header_in.is_empty() {
        return (String::new(), "[]");
    }
    let mut stmt = String::from("      const extra: Array<[string, unknown]> = [];\n");
    for header in &shape.header_in {
        let name = ts_quoted(&header.name);
        let parameter = message::parameter_name(&header.parameter);
        let schema = get_field_def(&parameter, &header.ty, "").zod_type();
        let refusal = outbound_refusal(
            prefix,
            operation,
            wire,
            &format!(
                "held.error.issues.map((issue) => ({{ path: [{name}, ...issue.path], message: \
                 issue.message }}))"
            ),
        );
        let _ = write!(
            stmt,
            "      {{\n        \
             const held = {prefix}Checked({schema}, {parameter});\n        \
             if (!held.success) {{\n{}        \
             }}\n        \
             if (held.sent !== undefined) extra.push([{name}, held.sent]);\n      \
             }}\n",
            message::deeper(&refusal, 2)
        );
    }
    (stmt, "extra")
}

fn reply_tail(
    prefix: &str,
    wire: &str,
    operation: &OperationDef,
    shape: &HttpShape,
    error: &Type,
    success: &Type,
) -> String {
    let fault_arm = |fault: &str| {
        format!("return {{ ok: false, error: {{ isServiceFault: true, fault: {fault} }} }};")
    };
    let mut stmt = format!(
        "      if (called.kind === \"fault\") {{\n        {}\n      }}\n",
        fault_arm("called.fault")
    );
    stmt.push_str(&declared_arm(prefix, wire, shape, error, &fault_arm));
    stmt.push_str(&success_arm(
        prefix, wire, operation, shape, success, &fault_arm,
    ));
    stmt.truncate(stmt.trim_end().len());
    stmt
}

/// What a declared error answers: its body read by the error type's own schema, then the elements
/// of `error_header_out` rejoined onto it.
fn declared_arm(
    prefix: &str,
    wire: &str,
    shape: &HttpShape,
    error: &Type,
    fault_arm: &dyn Fn(&str) -> String,
) -> String {
    let body = message::body_type(shape.error_header_out.len(), error);
    let schema = get_field_def("error", body, "").zod_type();
    let source = if shape.error_header_out.is_empty() {
        "called.error"
    } else {
        "received.value"
    };
    let mut stmt = String::from("      if (called.kind === \"declared\") {\n");
    if !shape.error_header_out.is_empty() {
        let _ = writeln!(stmt, "        const received = {prefix}Held(called.error);");
    }
    let _ = write!(
        stmt,
        "        const declared = {schema}.safeParse({source});\n        \
         if (!declared.success) {{\n          {}\n        }}\n",
        fault_arm(&format!(
            "{prefix}Undeserializable({wire}, {prefix}Issues(declared.error.issues))"
        ))
    );
    let mut joined = vec!["declared.data".to_owned()];
    for (index, (name, ty)) in shape
        .error_header_out
        .iter()
        .zip(message::header_types(shape.error_header_out.len(), error))
        .enumerate()
    {
        let ident = format!("errorHeaderOut{index}");
        let slot = get_field_def("value", ty, "").zod_slot_type();
        let _ = write!(
            stmt,
            "        const {ident} = {prefix}ReplyHeader({wire}, received, {}, {slot});\n        \
             if (!{ident}.ok) {{\n          {}\n        }}\n",
            ts_quoted(name),
            fault_arm(&format!("{ident}.fault"))
        );
        joined.push(format!("{ident}.value"));
    }
    let rejoined = if shape.error_header_out.is_empty() {
        "declared.data".to_owned()
    } else {
        format!("[{}]", joined.join(", "))
    };
    let _ = writeln!(
        stmt,
        "        return {{ ok: false, error: {rejoined} }};\n      }}"
    );
    stmt
}

/// What a success answers: the unit as `undefined`, otherwise its body read by the success
/// schema, with the elements of `header_out` rejoined onto it.
fn success_arm(
    prefix: &str,
    wire: &str,
    operation: &OperationDef,
    shape: &HttpShape,
    success: &Type,
    fault_arm: &dyn Fn(&str) -> String,
) -> String {
    if is_content_list(operation) {
        return format!(
            "      const read = {prefix}Blocks(called.blocks);\n      \
             if (!read.ok) {{\n        {}\n      }}\n      \
             return {{ ok: true, value: read.value }};\n",
            fault_arm(&format!("{prefix}Undeserializable({wire}, read.detail)"))
        );
    }
    let body = message::success_body(shape, success);
    let unit = is_unit_type(&body);
    let width: usize = if matches!(shape.body_kind, BodyKind::Bytes) {
        2
    } else {
        1
    };
    if unit && shape.header_out.is_empty() {
        return "      return { ok: true, value: undefined };\n".to_owned();
    }
    let pair_source = format!("{prefix}PairSource(called.structured, called.blocks)");
    let source = if !shape.header_out.is_empty() {
        "received.value"
    } else if is_content_pair(operation) {
        pair_source.as_str()
    } else {
        "called.structured"
    };
    let mut stmt = String::new();
    if !shape.header_out.is_empty() {
        let _ = writeln!(
            stmt,
            "      const received = {prefix}Held(called.structured);"
        );
    }
    let mut joined: Vec<String> = Vec::new();
    if unit {
        joined.push("undefined".to_owned());
    } else {
        let schema = get_field_def("value", &body, "").zod_type();
        let _ = write!(
            stmt,
            "      const parsed = {schema}.safeParse({source});\n      \
             if (!parsed.success) {{\n        {}\n      }}\n",
            fault_arm(&format!(
                "{prefix}Undeserializable({wire}, {prefix}Issues(parsed.error.issues))"
            ))
        );
        if width == 1 {
            joined.push("parsed.data".to_owned());
        } else {
            joined.extend((0..width).map(|index| format!("parsed.data[{index}]")));
        }
    }
    for (index, (name, ty)) in shape
        .header_out
        .iter()
        .zip(message::header_types(shape.header_out.len(), success))
        .enumerate()
    {
        let ident = format!("headerOut{index}");
        let slot = get_field_def("value", ty, "").zod_slot_type();
        let _ = write!(
            stmt,
            "      const {ident} = {prefix}ReplyHeader({wire}, received, {}, {slot});\n      \
             if (!{ident}.ok) {{\n        {}\n      }}\n",
            ts_quoted(name),
            fault_arm(&format!("{ident}.fault"))
        );
        joined.push(format!("{ident}.value"));
    }
    let rejoined = if shape.header_out.is_empty() {
        joined.remove(0)
    } else {
        format!("[{}]", joined.join(", "))
    };
    let _ = writeln!(stmt, "      return {{ ok: true, value: {rejoined} }};");
    stmt
}
