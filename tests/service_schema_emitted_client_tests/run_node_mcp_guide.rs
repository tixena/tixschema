//! The emitted TypeScript `mcp` server and client run by node on `GuideService`: the server's
//! answer to each prompt request, completion, HTTP request and scope query is the one the Rust
//! dispatcher gives, and the client reads the answers back into the text, the messages and the
//! declared error.

use super::GuideServiceSchema;
use super::mcp_guide_rows::{
    GuideError, GuideVersion, assert_client, assert_matches, client_input, input,
};
use super::runtime::{compiled_with_modules, node_modules, ran_with_modules, stand_down_modules};
use serde_json::Value;

const RUNTIME_VAR: &str = "TIXSCHEMA_NODE";
const COMPILER_VAR: &str = "TIXSCHEMA_TSC";

/// The implementation the driver serves: it writes down what reaches it.
const IMPL: &str = r##"
const reached = [];
const panicked = () => { throw new Error("the handler came apart"); };
const impl = {
  async configureOrganization(ctx) {
    reached.push(["configure-organization"]);
    return { ok: true, value: "Apply the entity types in this order." };
  },
  async getGuideVersion(ctx) {
    reached.push(["get-guide-version"]);
    return { ok: true, value: { version: 3 } };
  },
  async greet(ctx, name) {
    reached.push(["greet", name]);
    if (name === "boom") panicked();
    return name === "missing" ? { ok: false, error: { errorCode: "not-found" } } : { ok: true, value: `Say hello to ${name}.` };
  },
  async reviewDocumentType(ctx, req) {
    reached.push(["review-document-type", req.documentType, req.focus ?? null]);
    return {
      ok: true,
      value: [
        { role: "user", content: { type: "text", text: `Review the document type ${req.documentType} against this guide.` } },
        { role: "user", content: { type: "resource", uri: "guide://naming", mimeType: "text/markdown", text: "# Naming" } },
        { role: "assistant", content: { type: "image", data: Uint8Array.from([0x89, 0x50, 0x4e, 0x47]), mimeType: "image/png" } },
        { role: "assistant", content: { type: "resource_link", uri: "guide://naming", name: "naming", title: "Naming", mimeType: "text/markdown" } },
      ],
    };
  },
  async suggestDocumentTypes(ctx, req) {
    reached.push(["suggest-document-types", req.typed, Object.entries(req.chosen).sort()]);
    return {
      ok: true,
      value: ["ID card", "Invoice", "Passport"].filter((name) => name.toLowerCase().startsWith(req.typed.toLowerCase())),
    };
  },
};
"##;

const SERVER_DRIVER: &str = "
const input = JSON.parse(INPUT);
async function main() {
  const dispatch = createGuideServiceMcpDispatcher(impl);
  const inFlight = new GuideServiceMcpInFlight();
  const http = createGuideServiceMcpHttpDispatcher(impl, { allowedOrigins: [] }, async () => ({ ctx: {} }));
  const answers = {};
  const calls = {};
  for (const [name, text] of Object.entries(input.requests)) {
    reached.length = 0;
    const reply = await dispatch(text, () => ({}), async () => {}, inFlight);
    answers[name] = reply === undefined ? null : reply;
    calls[name] = [...reached];
  }
  const answered = {};
  for (const row of input.http) {
    const response = await http({
      method: 'POST',
      path: '/mcp',
      headers: row.headers,
      body: new TextEncoder().encode(row.body),
    });
    answered[row.name] = { status: response.status, body: new TextDecoder().decode(response.body) };
  }
  const scopes = input.queries.map(([method, name]) => guideServiceMcpScopes(method, name ?? undefined));
  console.log(JSON.stringify({ answers, reached: calls, http: answered, scopes }));
  process.exit(0);
}
main().catch((error) => { console.error(error); process.exit(1); });
";

const CLIENT_DRIVER: &str = r#"
const input = JSON.parse(INPUT);
const wire = (block) => {
  const base64 = (bytes) => Buffer.from(bytes).toString("base64");
  switch (block.type) {
    case "image":
    case "audio":
      return { type: block.type, data: base64(block.data), mimeType: block.mimeType };
    case "resource":
      return {
        type: "resource",
        resource: {
          uri: block.uri,
          ...(block.mimeType === undefined ? {} : { mimeType: block.mimeType }),
          ...(block.blob === undefined ? { text: block.text } : { blob: base64(block.blob) }),
        },
      };
    default:
      return Object.fromEntries(Object.entries(block).filter(([, held]) => held !== undefined));
  }
};
const outcome = (result) => {
  if (result.ok) {
    const value = result.value;
    if (Array.isArray(value)) return { ok: value.map((message) => ({ role: message.role, content: wire(message.content) })) };
    return { ok: value };
  }
  if ("isServiceFault" in result.error) return { fault: result.error.fault.kind };
  return { declared: result.error };
};
async function main() {
  const seen = [];
  const queue = [];
  const transport = {
    async send(message) {
      seen.push(JSON.parse(message.text));
      return queue.shift();
    },
  };
  const client = createGuideServiceMcpClient(transport);
  const calls = {
    string: () => client.configureOrganization({}),
    named: () => client.greet("Ana"),
    messages: () => client.reviewDocumentType({ documentType: "ID card", focus: undefined }),
    declared: () => client.greet("missing"),
    panicked: () => client.greet("boom"),
    unknown: () => client.greet("nobody"),
    unreadable: () => client.reviewDocumentType({ documentType: "ID card", focus: "names" }),
  };
  const outcomes = {};
  for (const { label, answer } of input.calls) {
    queue.push(answer);
    outcomes[label] = outcome(await calls[label]());
  }
  console.log(JSON.stringify({ outcomes, seen }));
  process.exit(0);
}
main().catch((error) => { console.error(error); process.exit(1); });
"#;

const USAGE: &str = r#"import {
  createGuideServiceMcpClient,
  createGuideServiceMcpDispatcher,
  guideServiceMcpOperations,
  type GuideServiceImpl,
  type GuideServiceMcpTransport,
  type McpContent,
  type McpPromptMessage,
  type McpRole,
} from "./bundle.js";

declare const impl: GuideServiceImpl<number>;
declare const transport: GuideServiceMcpTransport;

export const dispatcher = createGuideServiceMcpDispatcher(impl);
export const prompts: ReadonlyArray<string> = guideServiceMcpOperations.flatMap((operation) => (operation.kind === "prompt" ? [operation.prompt] : []));
export const role: McpRole = "assistant";
export const message: McpPromptMessage = { role, content: { type: "text", text: "a guide" } as McpContent };

export async function usage() {
  const client = createGuideServiceMcpClient(transport);
  const greeting = await client.greet("Ana");
  if (greeting.ok) {
    const text: string = greeting.value;
    return text.length;
  }
  const messages = await client.reviewDocumentType({ documentType: "ID card", focus: undefined });
  if (messages.ok) {
    const first: McpPromptMessage | undefined = messages.value[0];
    return first === undefined ? 0 : 1;
  }
  return 2;
}
"#;

const REQUIRED_PACKAGES: &[&str] = &["zod"];

fn bundle() -> String {
    [
        "import { z, type ZodType, type $brand } from \"zod\";".to_owned(),
        GuideError::ts_definition(),
        GuideError::zod_schema(),
        GuideVersion::ts_definition(),
        GuideVersion::zod_schema(),
        GuideServiceSchema::ts_definition(),
        GuideServiceSchema::ts_mcp_content(),
        GuideServiceSchema::ts_service(),
        GuideServiceSchema::ts_mcp_service(),
        GuideServiceSchema::ts_mcp_client(),
    ]
    .join("\n\n")
}

fn run(named: &str, body: &str, given: &Value) -> Option<Value> {
    let Some(modules) = node_modules(REQUIRED_PACKAGES) else {
        stand_down_modules(
            REQUIRED_PACKAGES,
            "the emitted TypeScript MCP server and client of the guide service",
        );
        return None;
    };
    let source = format!(
        "{}\n\nconst INPUT = {};\n{IMPL}\n{body}",
        bundle(),
        Value::String(given.to_string())
    );
    let wrote = ran_with_modules(
        named,
        RUNTIME_VAR,
        "node",
        &format!("{named}.mts"),
        &source,
        &modules,
    )?;
    Some(serde_json::from_str(wrote.trim()).unwrap())
}

#[test]
fn the_typescript_server_answers_the_bytes_the_rust_dispatcher_answers() {
    let Some(report) = run("mcp-guide-server", SERVER_DRIVER, &input()) else {
        return;
    };
    assert_matches(&report);
}

#[test]
fn the_typescript_client_reads_the_text_the_messages_and_the_error_the_rust_dispatcher_answers() {
    let Some(report) = run("mcp-guide-client", CLIENT_DRIVER, &client_input()) else {
        return;
    };
    assert_client(&report);
}

#[test]
fn the_typescript_text_of_the_guide_service_type_checks() {
    let Some(modules) = node_modules(REQUIRED_PACKAGES) else {
        stand_down_modules(
            REQUIRED_PACKAGES,
            "the type-check of the guide service's text",
        );
        return;
    };
    let text = bundle();
    let Some((succeeded, said)) = compiled_with_modules(
        "mcp-guide-types",
        COMPILER_VAR,
        &[("bundle.ts", &text), ("usage.ts", USAGE)],
        &modules,
    ) else {
        return;
    };
    assert!(succeeded, "the emitted text does not type-check:\n{said}");
}
