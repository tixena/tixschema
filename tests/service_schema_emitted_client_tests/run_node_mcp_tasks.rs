//! The emitted TypeScript `mcp` server and client run by node on `JobService`: the server's answer
//! to each task request, HTTP request, scope query and `subscriptions/listen` is the one the Rust
//! dispatcher gives, and the client reads the handle and the state of a task back as ordinary
//! successes.

use super::JobServiceSchema;
use super::mcp_task_rows::{
    ExportReport, ExportRequest, JobError, assert_client, assert_matches, client_input, input,
};
use super::runtime::{compiled_with_modules, node_modules, ran_with_modules, stand_down_modules};
use crate::mcp_streams::NODE_RESPOND;
use serde_json::Value;

const RUNTIME_VAR: &str = "TIXSCHEMA_NODE";
const COMPILER_VAR: &str = "TIXSCHEMA_TSC";

/// The implementation the driver serves: it writes down what reaches it.
const IMPL: &str = r#"
const reached = [];
const task = (taskId) => ({ taskId, createdAt: "2026-11-01T20:00:00Z", lastUpdatedAt: "2026-11-01T20:00:00Z", ttlMs: 3600000, pollIntervalMs: 2000 });
const impl = {
  async startExport(ctx, req) {
    reached.push(["start-export", req.folder]);
    return req.folder === "missing" ? { ok: false, error: { errorCode: "not-found" } } : { ok: true, value: task("7f3a") };
  },
  async exportStatus(ctx, id) {
    reached.push(["export-status", id]);
    if (id === "boom") throw new Error("the handler came apart");
    switch (id) {
      case "7f3a":
        return { ok: true, value: { ...task(id), status: "working" } };
      case "done":
        return { ok: true, value: { ...task(id), lastUpdatedAt: "2026-11-01T20:04:10Z", status: "completed", result: { files: 3 } } };
      case "needs":
        return {
          ok: true,
          value: {
            ...task(id),
            status: "input_required",
            inputRequests: {
              overwrite: {
                method: "elicitation/create",
                params: { mode: "form", message: "Overwrite the folder?", requestedSchema: { type: "object", properties: {} } },
              },
            },
          },
        };
      case "broken":
        return { ok: true, value: { ...task(id), status: "failed", error: { code: -32603, message: "the folder vanished" } } };
      case "stopped":
        return { ok: true, value: { ...task(id), status: "cancelled" } };
      default:
        return { ok: false, error: { errorCode: "not-found" } };
    }
  },
  async cancelExport(ctx, id) {
    reached.push(["cancel-export", id]);
    return id === "gone" ? { ok: false, error: { errorCode: "not-found" } } : { ok: true };
  },
  async answerExport(ctx, req) {
    reached.push(["answer-export", req.taskId, req.key, req.action, req.content ?? null]);
    return req.taskId === "gone" ? { ok: false, error: { errorCode: "not-found" } } : { ok: true };
  },
};
"#;

const SERVER_DRIVER: &str = "
const input = JSON.parse(INPUT);
const changesOf = (script) => {
  const queue = [...script];
  return async () => queue.shift();
};
async function main() {
  const dispatch = createJobServiceMcpDispatcher(impl);
  const inFlight = new JobServiceMcpInFlight();
  const http = createJobServiceMcpHttpDispatcher(impl, { allowedOrigins: [] }, async () => ({ ctx: {} }));
  const answers = {};
  const calls = {};
  for (const [name, text] of Object.entries(input.requests)) {
    reached.length = 0;
    const reply = await dispatch(text, () => ({}), async () => {}, inFlight);
    answers[name] = reply === undefined ? null : reply;
    calls[name] = [...reached];
  }
  reached.length = 0;
  const answered = {};
  for (const row of input.http) {
    const response = await respond(http, {
      method: 'POST',
      path: '/mcp',
      headers: row.headers,
      body: new TextEncoder().encode(row.body),
    });
    answered[row.name] = { status: response.status, body: new TextDecoder().decode(response.body) };
  }
  const scopes = input.queries.map(([method, name]) => jobServiceMcpScopes(method, name ?? undefined));
  const listens = [];
  for (const row of input.listens) {
    const written = [];
    const reply = await dispatch(row.request, () => ({}), async (message) => { written.push(message); }, new JobServiceMcpInFlight(), changesOf(row.script));
    listens.push({ reply: reply ?? null, written });
  }
  console.log(JSON.stringify({ answers, reached: calls, http: answered, scopes, listens }));
  process.exit(0);
}
main().catch((error) => { console.error(error); process.exit(1); });
";

const CLIENT_DRIVER: &str = r#"
const input = JSON.parse(INPUT);
const outcome = (result) => {
  if (result.ok) return { ok: result.value === undefined ? null : result.value };
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
  const client = createJobServiceMcpClient(transport);
  const calls = {
    started: () => client.startExport({ folder: "reports" }),
    working: () => client.exportStatus("7f3a"),
    completed: () => client.exportStatus("done"),
    waiting: () => client.exportStatus("needs"),
    failed: () => client.exportStatus("broken"),
    cancelled: () => client.exportStatus("stopped"),
    cancel: () => client.cancelExport("7f3a"),
    answer: () => client.answerExport({ taskId: "needs", key: "overwrite", action: "accept", content: "{}" }),
    declared: () => client.exportStatus("gone"),
    panicked: () => client.exportStatus("boom"),
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
  createJobServiceMcpClient,
  createJobServiceMcpDispatcher,
  type JobServiceImpl,
  type JobServiceMcpTransport,
  type McpTask,
  type McpTaskState,
} from "./bundle.js";

declare const impl: JobServiceImpl<number>;
declare const transport: JobServiceMcpTransport;

export const dispatcher = createJobServiceMcpDispatcher(impl);
export const handle: McpTask = { taskId: "7f3a", createdAt: "2026-11-01T20:00:00Z", lastUpdatedAt: "2026-11-01T20:00:00Z", ttlMs: null };
export const state: McpTaskState<{ files: number }> = { ...handle, status: "completed", result: { files: 3 } };

export async function usage() {
  const client = createJobServiceMcpClient(transport);
  const started = await client.startExport({ folder: "reports" });
  if (!started.ok) return 0;
  const status = await client.exportStatus(started.value.taskId);
  if (status.ok && status.value.status === "completed") {
    const files: number = status.value.result.files;
    return files;
  }
  return 1;
}
"#;

const REQUIRED_PACKAGES: &[&str] = &["zod"];

fn bundle() -> String {
    [
        "import { z, type ZodType, type $brand } from \"zod\";".to_owned(),
        JobError::ts_definition(),
        JobError::zod_schema(),
        ExportRequest::ts_definition(),
        ExportRequest::zod_schema(),
        ExportReport::ts_definition(),
        ExportReport::zod_schema(),
        JobServiceSchema::ts_definition(),
        JobServiceSchema::ts_mcp_content(),
        JobServiceSchema::ts_service(),
        JobServiceSchema::ts_mcp_service(),
        JobServiceSchema::ts_mcp_client(),
    ]
    .join("\n\n")
}

fn run(named: &str, body: &str, given: &Value) -> Option<Value> {
    let Some(modules) = node_modules(REQUIRED_PACKAGES) else {
        stand_down_modules(
            REQUIRED_PACKAGES,
            "the emitted TypeScript MCP server and client of the job service",
        );
        return None;
    };
    let source = format!(
        "{}\n\nconst INPUT = {};\n{IMPL}\n{NODE_RESPOND}\n{body}",
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
    let Some(report) = run("mcp-tasks-server", SERVER_DRIVER, &input()) else {
        return;
    };
    assert_matches(&report);
}

#[test]
fn the_typescript_client_reads_the_handle_and_the_state_the_rust_dispatcher_answers() {
    let Some(report) = run("mcp-tasks-client", CLIENT_DRIVER, &client_input()) else {
        return;
    };
    assert_client(&report);
}

#[test]
fn the_typescript_text_of_the_job_service_type_checks() {
    let Some(modules) = node_modules(REQUIRED_PACKAGES) else {
        stand_down_modules(
            REQUIRED_PACKAGES,
            "the type-check of the job service's text",
        );
        return;
    };
    let text = bundle();
    let Some((succeeded, said)) = compiled_with_modules(
        "mcp-tasks-types",
        COMPILER_VAR,
        &[("bundle.ts", &text), ("usage.ts", USAGE)],
        &modules,
    ) else {
        return;
    };
    assert!(succeeded, "the emitted text does not type-check:\n{said}");
}
