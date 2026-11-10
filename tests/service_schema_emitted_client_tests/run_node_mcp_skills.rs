//! The emitted TypeScript `mcp` server run by node over skills: a folder named on the service and
//! two skills an operation writes at run time. It answers the bytes the Rust dispatcher answers to
//! the same requests, alone and on an endpoint and over plain HTTP terms, and every answer is held
//! to the official 2026-07-28 schema.

use super::LessonServiceSchema;
use super::mcp_lesson_service::{assert_http, http_rows, lesson_answer, rows};
use super::mcp_library_rows::{HttpAnswer, assert_answers};
use super::mcp_library_service::{BookError, BookInfo, BookRequest};
use super::runtime::{compiled_with_modules, node_modules, ran_with_modules, stand_down_modules};
use crate::mcp_streams::NODE_RESPOND;
use serde_json::{Value, json};

/// Names the runtime to run, for a machine that has one somewhere other than `PATH`.
const RUNTIME_VAR: &str = "TIXSCHEMA_NODE";

/// Names the compiler the emitted text is type-checked with.
const COMPILER_VAR: &str = "TIXSCHEMA_TSC";

/// The one package the drivers import at runtime: `zod`, for the schemas the dispatcher judges by.
const REQUIRED_PACKAGES: &[&str] = &["zod"];

const DRIVER: &str = r##"
const panicked = () => { throw new Error("the handler came apart"); };
const notFound = { ok: false, error: { errorCode: "not-found" } };

const lessonImpl = {
  async releaseNotesSkill(ctx, path) {
    if (path === "SKILL.md") return { ok: true, value: "# Release notes\n\nRead the changes since 2026-11-01, then write one line per change.\n" };
    if (path === "references/changes.md") return { ok: true, value: "- renamed a document\n" };
    if (path === "panic") panicked();
    return notFound;
  },
  async figuresSkill(ctx, path) {
    if (path === "SKILL.md") return { ok: true, value: [new TextEncoder().encode("# Figures\n"), "text/markdown"] };
    if (path === "figures.bin") return { ok: true, value: [new Uint8Array([0, 255, 128, 1]), "application/octet-stream"] };
    return notFound;
  },
  async getLesson(ctx, req) {
    return req.book_id === "lesson-1" ? { ok: true, value: { pages: 3 } } : notFound;
  },
};

async function main() {
  const input = JSON.parse(INPUT);
  const route = createLessonServiceMcpRoute(lessonImpl);
  const endpoint = lessonServiceMcpEndpoint(lessonServiceMcpPart);
  const host = { writer: async () => {}, inFlight: new LessonServiceMcpInFlight() };
  const served = (text) => lessonServiceMcpServe(text, lessonServiceMcpServerName, lessonServiceMcpInstructions, endpoint, () => ({}), host,
    async (ctx, part, method, params) => (part === 0 ? route(ctx, method, params) : undefined));
  const alone = createLessonServiceMcpDispatcher(lessonImpl);
  const over = createLessonServiceMcpHttpDispatcher(lessonImpl, { allowedOrigins: [input.origin] }, async () => ({ ctx: {} }));
  const answers = (ask) => Promise.all(input.requests.map(async (text) => (await ask(text)) ?? null));
  const http = [];
  for (const row of input.http) {
    const response = await respond(over, { method: "POST", path: "/mcp", headers: row.headers, body: new TextEncoder().encode(row.body) });
    http.push({ status: response.status, headers: response.headers.map(([name, value]) => [name, value]), body: new TextDecoder().decode(response.body) });
  }
  console.log(JSON.stringify({
    names: lessonServiceMcpPart.names,
    endpoint: await answers(served),
    alone: await answers((text) => alone(text, () => ({}), host.writer, host.inFlight)),
    http,
  }));
  process.exit(0);
}
main().catch((error) => { console.error(error); process.exit(1); });
"##;

/// A caller that reads the table the way a consumer does.
const USAGE_CALLER: &str = r#"import { lessonServiceMcpOperations, lessonServiceMcpPart } from "./bundle.js";

export const kinds: ReadonlyArray<string> = lessonServiceMcpOperations.map((operation) => operation.kind);
export const names: readonly ["skill:workspace-figures", "tool:get-lesson", "skill:release-notes", "skill:code-review", "skill:pack-parcels"] = lessonServiceMcpPart.names;
"#;

fn bundle() -> String {
    [
        "import { z, type ZodType, type $brand } from \"zod\";".to_owned(),
        BookRequest::ts_definition(),
        BookRequest::zod_schema(),
        BookInfo::ts_definition(),
        BookInfo::zod_schema(),
        BookError::ts_definition(),
        BookError::zod_schema(),
        LessonServiceSchema::ts_definition(),
        LessonServiceSchema::ts_service(),
        LessonServiceSchema::ts_mcp_service(),
    ]
    .join("\n\n")
}

/// The report of the one node run; `None` where no node toolchain was reachable.
fn driven() -> Option<Value> {
    let Some(modules) = node_modules(REQUIRED_PACKAGES) else {
        stand_down_modules(
            REQUIRED_PACKAGES,
            "the emitted TypeScript MCP server over skills",
        );
        return None;
    };
    let requests: Vec<String> = rows().into_iter().map(|row| row.text).collect();
    let http: Vec<Value> = http_rows()
        .into_iter()
        .map(|row| {
            json!({
                "headers": row.headers.iter().map(|(name, value)| json!([name, value])).collect::<Vec<_>>(),
                "body": row.body,
            })
        })
        .collect();
    let input =
        json!({ "origin": super::mcp_library_rows::ORIGIN, "requests": requests, "http": http });
    let literal = Value::String(input.to_string());
    let source = format!(
        "{}\n\n{}\nconst INPUT = {literal};\n{DRIVER}",
        bundle(),
        NODE_RESPOND
    );
    let wrote = ran_with_modules(
        "mcp-skills",
        RUNTIME_VAR,
        "node",
        "mcp-skills.mts",
        &source,
        &modules,
    )?;
    Some(serde_json::from_str(wrote.trim()).unwrap())
}

fn texts(report: &Value) -> Vec<Option<String>> {
    report
        .as_array()
        .unwrap()
        .iter()
        .map(|answered| answered.as_str().map(str::to_owned))
        .collect()
}

#[test]
fn the_typescript_server_answers_the_bytes_the_rust_dispatcher_answers() {
    let Some(report) = driven() else {
        return;
    };
    let rows = rows();
    assert_answers("alone", lesson_answer, &texts(&report["alone"]), &rows);
    assert_answers(
        "endpoint",
        lesson_answer,
        &texts(&report["endpoint"]),
        &rows,
    );
}

#[test]
fn an_endpoint_names_every_skill_as_the_rust_registry_does() {
    let Some(report) = driven() else {
        return;
    };
    assert_eq!(
        report["names"],
        json!(
            LessonServiceSchema::MCP_NAMES
                .iter()
                .map(|(kind, name)| format!("{kind}:{name}"))
                .collect::<Vec<_>>()
        )
    );
}

#[test]
fn the_typescript_server_answers_the_skills_over_plain_http_terms_as_rust_does() {
    let Some(report) = driven() else {
        return;
    };
    let answered: Vec<HttpAnswer> = report["http"]
        .as_array()
        .unwrap()
        .iter()
        .map(|held| {
            (
                u16::try_from(held["status"].as_u64().unwrap()).unwrap(),
                held["headers"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|pair| {
                        (
                            pair[0].as_str().unwrap().to_owned(),
                            pair[1].as_str().unwrap().to_owned(),
                        )
                    })
                    .collect(),
                held["body"].as_str().unwrap().to_owned(),
            )
        })
        .collect();
    assert_http("typescript", &answered);
}

#[test]
fn the_skills_text_type_checks_under_strict() {
    let Some(modules) = node_modules(REQUIRED_PACKAGES) else {
        stand_down_modules(
            REQUIRED_PACKAGES,
            "the type-check of the emitted MCP text over skills",
        );
        return;
    };
    let text = bundle();
    let Some((succeeded, said)) = compiled_with_modules(
        "mcp-skill-types",
        COMPILER_VAR,
        &[("bundle.ts", &text), ("usage.ts", USAGE_CALLER)],
        &modules,
    ) else {
        return;
    };
    assert!(succeeded, "the emitted text does not type-check:\n{said}");
}
