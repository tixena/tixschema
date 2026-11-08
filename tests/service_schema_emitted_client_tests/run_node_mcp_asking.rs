//! The emitted TypeScript `mcp` server and client run by node on `AskingService`: the server's
//! answer to each request of the form, link and content flows is the one the Rust dispatcher
//! gives, and the client reads the answers back into the pair, the list and the declared arm.

use super::mcp_asking_rows::{
    AskingAdjacentError, AskingConfirmation, AskingError, AskingKindError, AskingLinkError,
    AskingLinked, AskingMiss, AskingPriority, AskingRenamedError, AskingReportRequest,
    AskingShipping, assert_client, assert_matches, client_input, input,
};
use super::runtime::{compiled_with_modules, node_modules, ran_with_modules, stand_down_modules};
use super::{AskingServiceSchema, PreviewServiceSchema};
use serde_json::Value;

const RUNTIME_VAR: &str = "TIXSCHEMA_NODE";
const COMPILER_VAR: &str = "TIXSCHEMA_TSC";

/// The implementation both drivers serve: it writes down what reaches it.
const IMPL: &str = r#"
const reached = [];
const impl = {
  async buildReport(ctx, req) {
    reached.push(["build-report", req.title]);
    return {
      ok: true,
      value: [
        { type: "text", text: req.title },
        { type: "image", data: Uint8Array.from([0x89, 0x50, 0x4e, 0x47]), mimeType: "image/png" },
        { type: "audio", data: Uint8Array.from([1, 2, 3, 4, 5]), mimeType: "audio/mpeg" },
        { type: "resource_link", uri: "workspace://documents/doc-1", name: "id-card.pdf", title: "Id card", mimeType: "application/pdf" },
        { type: "resource", uri: "workspace://documents/doc-2", mimeType: "text/plain", text: "passport" },
        { type: "resource", uri: "workspace://documents/doc-3", blob: Uint8Array.from([9, 8, 7]) },
      ],
    };
  },
  async connectRenamed(ctx, id) {
    reached.push(["connect-renamed", id]);
    return {
      ok: false,
      error: {
        errorCode: "connect-first",
        url: "https://mcp.example.com/connect/drive?request=7f3a",
        message: "Connect your Google Drive so this server can import the file.",
      },
    };
  },
  async connectAdjacent(ctx, id) {
    reached.push(["connect-adjacent", id]);
    return {
      ok: false,
      error: {
        kind: "needs-connection",
        data: {
          url: "https://mcp.example.com/connect/drive?request=7f3a",
          message: "Connect your Google Drive so this server can import the file.",
        },
      },
    };
  },
  async deleteAdjacent(ctx, req) {
    reached.push(["delete-adjacent", req.documentId, req.confirmation ?? null]);
    if (req.documentId === "missing") return { ok: false, error: { kind: "not-found" } };
    if (req.confirmation === undefined) {
      return { ok: false, error: { kind: "needs-confirmation", data: { message: "Delete this document for good?" } } };
    }
    return req.confirmation.confirm ? { ok: true } : { ok: false, error: { kind: "refused" } };
  },
  async connectByKind(ctx, id) {
    reached.push(["connect-by-kind", id]);
    return {
      ok: false,
      error: {
        kind: "needs_connection",
        url: "https://mcp.example.com/connect/drive?request=7f3a",
        message: "Connect your Google Drive so this server can import the file.",
      },
    };
  },
  async deleteDocument(ctx, req) {
    reached.push(["delete-document", req.documentId, req.confirmation ?? null]);
    if (req.documentId === "missing") return { ok: false, error: { errorCode: "not-found" } };
    if (req.confirmation === undefined) {
      return { ok: false, error: { errorCode: "needs-confirmation", message: "Delete this document for good?" } };
    }
    return req.confirmation.confirm ? { ok: true } : { ok: false, error: { errorCode: "refused" } };
  },
  async importFromDrive(ctx, id) {
    reached.push(["import-from-drive", id]);
    if (id === "connected") return { ok: true, value: { document_id: `doc-${id}` } };
    return {
      ok: false,
      error: {
        errorCode: "needs-connection",
        url: "https://mcp.example.com/connect/drive?request=7f3a",
        message: "Connect your Google Drive so this server can import the file.",
      },
    };
  },
  async shipDocument(ctx, req) {
    reached.push(["ship-document", req.documentId, req.shipping ?? null]);
    if (req.shipping === undefined) {
      return { ok: false, error: { errorCode: "needs-confirmation", message: "How should it ship?" } };
    }
    return { ok: true };
  },
};
const previewing = {
  async documentPreview(ctx, id) {
    reached.push(["document-preview", id]);
    if (id === "doc-1") return { ok: true, value: [Uint8Array.from([0x89, 0x50, 0x4e, 0x47]), "image/png"] };
    if (id === "voice") return { ok: true, value: [Uint8Array.from([1, 2, 3, 4, 5]), "audio/mpeg"] };
    if (id === "plain") return { ok: true, value: [Uint8Array.from([37, 80, 68, 70, 45]), "application/pdf"] };
    return { ok: false, error: { errorCode: "not-found" } };
  },
};
"#;

const SERVER_DRIVER: &str = "
const input = JSON.parse(INPUT);
async function main() {
  const asking = createAskingServiceMcpDispatcher(impl);
  const previews = createPreviewServiceMcpDispatcher(previewing);
  const inFlight = new AskingServiceMcpInFlight();
  const previewFlight = new PreviewServiceMcpInFlight();
  const answers = {};
  const calls = {};
  for (const [name, text] of Object.entries(input.requests)) {
    reached.length = 0;
    const reply = input.previews.includes(name)
      ? await previews(text, () => ({}), async () => {}, previewFlight)
      : await asking(text, () => ({}), async () => {}, inFlight);
    answers[name] = reply === undefined ? null : reply;
    calls[name] = [...reached];
  }
  console.log(JSON.stringify({ answers, reached: calls }));
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
    if (value === undefined) return { ok: null };
    if (Array.isArray(value) && value.length === 2 && value[0] instanceof Uint8Array) {
      return { ok: { bytes: Array.from(value[0]), contentType: value[1] } };
    }
    if (Array.isArray(value)) return { ok: value.map(wire) };
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
  const client = createAskingServiceMcpClient(transport);
  const previewer = createPreviewServiceMcpClient(transport);
  const calls = {
    declared: () => client.deleteDocument({ documentId: "doc-1", confirmation: undefined }),
    confirmed: () => client.deleteDocument({ documentId: "doc-1", confirmation: { confirm: true } }),
    image: () => previewer.documentPreview("doc-1"),
    audio: () => previewer.documentPreview("voice"),
    plain: () => previewer.documentPreview("plain"),
    missing: () => previewer.documentPreview("nothing"),
    report: () => client.buildReport({ title: "Q3" }),
    link: () => client.importFromDrive("1A2b3C"),
    linked: () => client.importFromDrive("connected"),
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
  createAskingServiceMcpClient,
  createAskingServiceMcpDispatcher,
  createPreviewServiceMcpClient,
  askingServiceMcpOperations,
  type AskingServiceImpl,
  type AskingServiceMcpTransport,
  type McpContent,
  type PreviewServiceMcpTransport,
} from "./bundle.js";

declare const impl: AskingServiceImpl<number>;
declare const transport: AskingServiceMcpTransport;
declare const previewTransport: PreviewServiceMcpTransport;

export const dispatcher = createAskingServiceMcpDispatcher(impl);
export const kinds: ReadonlyArray<string> = askingServiceMcpOperations.flatMap((operation) => (operation.kind === "tool" ? [operation.tool] : []));
export const blocks: ReadonlyArray<McpContent> = [
  { type: "text", text: "a chart" },
  { type: "image", data: new Uint8Array(), mimeType: "image/png" },
  { type: "resource", uri: "workspace://documents/doc-3", blob: new Uint8Array() },
];

export async function usage() {
  const client = createAskingServiceMcpClient(transport);
  const first = await client.deleteDocument({ documentId: "doc-1", confirmation: undefined });
  if (!first.ok && !("isServiceFault" in first.error) && first.error.errorCode === "needs-confirmation") {
    const message: string = first.error.message;
    await client.deleteDocument({ documentId: "doc-1", confirmation: { confirm: message.length > 0 } });
  }
  const preview = await createPreviewServiceMcpClient(previewTransport).documentPreview("doc-1");
  if (preview.ok) {
    const bytes: Uint8Array = preview.value[0];
    const contentType: string = preview.value[1];
    return [bytes.length, contentType.length];
  }
  const report = await client.buildReport({ title: "Q3" });
  if (report.ok) {
    const block: McpContent | undefined = report.value[0];
    return block === undefined ? 0 : 1;
  }
  return 2;
}
"#;

const REQUIRED_PACKAGES: &[&str] = &["zod"];

fn bundle() -> String {
    [
        "import { z, type ZodType, type $brand } from \"zod\";".to_owned(),
        AskingPriority::ts_definition(),
        AskingPriority::zod_schema(),
        AskingShipping::ts_definition(),
        AskingShipping::zod_schema(),
        AskingConfirmation::ts_definition(),
        AskingConfirmation::zod_schema(),
        AskingAdjacentError::ts_definition(),
        AskingAdjacentError::zod_schema(),
        AskingError::ts_definition(),
        AskingError::zod_schema(),
        AskingLinkError::ts_definition(),
        AskingLinkError::zod_schema(),
        AskingRenamedError::ts_definition(),
        AskingRenamedError::zod_schema(),
        AskingKindError::ts_definition(),
        AskingKindError::zod_schema(),
        AskingMiss::ts_definition(),
        AskingMiss::zod_schema(),
        AskingReportRequest::ts_definition(),
        AskingReportRequest::zod_schema(),
        AskingLinked::ts_definition(),
        AskingLinked::zod_schema(),
        AskingServiceSchema::ts_definition(),
        AskingServiceSchema::ts_mcp_content(),
        AskingServiceSchema::ts_service(),
        AskingServiceSchema::ts_mcp_service(),
        AskingServiceSchema::ts_mcp_client(),
        PreviewServiceSchema::ts_definition(),
        PreviewServiceSchema::ts_service(),
        PreviewServiceSchema::ts_mcp_service(),
        PreviewServiceSchema::ts_mcp_client(),
    ]
    .join("\n\n")
}

fn run(named: &str, body: &str, given: &Value) -> Option<Value> {
    let Some(modules) = node_modules(REQUIRED_PACKAGES) else {
        stand_down_modules(
            REQUIRED_PACKAGES,
            "the emitted TypeScript MCP server and client of the asking service",
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
    let Some(report) = run("mcp-asking-server", SERVER_DRIVER, &input()) else {
        return;
    };
    assert_matches(&report);
}

#[test]
fn the_typescript_client_reads_the_blocks_and_the_arm_the_rust_dispatcher_answers() {
    let Some(report) = run("mcp-asking-client", CLIENT_DRIVER, &client_input()) else {
        return;
    };
    assert_client(&report);
}

#[test]
fn the_typescript_text_of_the_asking_service_type_checks() {
    let Some(modules) = node_modules(REQUIRED_PACKAGES) else {
        stand_down_modules(
            REQUIRED_PACKAGES,
            "the type-check of the asking service's text",
        );
        return;
    };
    let text = bundle();
    let Some((succeeded, said)) = compiled_with_modules(
        "mcp-asking-types",
        COMPILER_VAR,
        &[("bundle.ts", &text), ("usage.ts", USAGE)],
        &modules,
    ) else {
        return;
    };
    assert!(succeeded, "the emitted text does not type-check:\n{said}");
}
