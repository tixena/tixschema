//! The emitted TypeScript `mcp` server run by node over resources, templates, a listing and a
//! completion: it answers the bytes the Rust dispatcher answers to the same requests, for services
//! that declare them and for one that declares none, alone and beside a second service on one
//! endpoint and over plain HTTP terms, and every answer is held to the official 2026-07-28 schema.

use super::mcp_library_rows::{
    HttpAnswer, HttpRow, ORIGIN, Row, assert_answers, assert_http, bare_answer, depot_answer,
    http_rows, library_answer, rows, workspace_answer,
};
use super::mcp_library_service::MirrorServiceSchema;
use super::mcp_library_service::{BookError, BookInfo, BookRequest};
use super::mcp_shelf_service::{ShelfLabel, assert_faults, fault_requests};
use super::runtime::{compiled_with_modules, node_modules, ran_with_modules, stand_down_modules};
use super::{BareServiceSchema, DepotServiceSchema, LibraryServiceSchema, ShelfServiceSchema};
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

const libraryImpl = {
  async getBook(ctx, req) {
    if (req.book_id === "panic") panicked();
    return req.book_id === "book-1" ? { ok: true, value: { pages: 7 } } : notFound;
  },
  async libraryGuide(ctx, req) {
    return { ok: true, value: "# The library" };
  },
  async bookCover(ctx, bookId) {
    if (bookId === "book-1") return { ok: true, value: [new Uint8Array([37, 80, 68, 70, 45]), "application/pdf"] };
    if (bookId === "book-2") return { ok: true, value: [new Uint8Array([0, 255, 128, 1, 2, 3, 4]), "image/png"] };
    if (bookId === "panic") panicked();
    return notFound;
  },
  async listBooks(ctx, cursor) {
    if (cursor === undefined) {
      return { ok: true, value: { resources: [{ uri: "library://covers/book-1", name: "cover-1.pdf", mimeType: "application/pdf" }], nextCursor: "page-2" } };
    }
    if (cursor === "page-2") {
      return { ok: true, value: { resources: [{ uri: "library://covers/book-2", name: "cover-2.pdf", mimeType: "application/pdf", size: 48213, title: "Second cover" }] } };
    }
    if (cursor === "panic") panicked();
    return notFound;
  },
  async suggestBooks(ctx, req) {
    if (req.typed === "many") return { ok: true, value: Array.from({ length: 150 }, (_, at) => `many-${at}`) };
    if (req.typed === "panic") panicked();
    if (req.typed === "none") return notFound;
    if (req.typed === "kind") return { ok: true, value: [req.chosen.kind ?? ""] };
    return { ok: true, value: ["book-1", "book-2"].filter((name) => name.startsWith(req.typed)) };
  },
};

const depotImpl = {
  async getCrate(ctx, crateId) {
    return crateId === "crate-1" ? { ok: true, value: "{\"total\":120}" } : notFound;
  },
  async getBin(ctx, req) {
    return { ok: true, value: `${req.aisle}/${req.binId}` };
  },
  async listCrates(ctx, cursor) {
    if (cursor !== undefined) return notFound;
    return { ok: true, value: { resources: [{ uri: "depot://crates/crate-1", name: "crate-1", mimeType: "application/json" }] } };
  },
};

const bareImpl = {
  async getPlain(ctx, req) {
    if (req.book_id === "panic") panicked();
    return req.book_id === "book-1" ? { ok: true, value: { pages: 7 } } : notFound;
  },
};

async function main() {
  const input = JSON.parse(INPUT);
  const library = createLibraryServiceMcpRoute(libraryImpl);
  const depot = createDepotServiceMcpRoute(depotImpl);
  const workspace = libraryServiceMcpEndpoint(libraryServiceMcpPart, depotServiceMcpPart);
  const host = { writer: async () => {}, inFlight: new LibraryServiceMcpInFlight() };
  const served = (text) => libraryServiceMcpServe(text, "Workspace", undefined, workspace, () => ({}), host,
    async (ctx, part, method, params) => (part === 0 ? library(ctx, method, params) : part === 1 ? depot(ctx, method, params) : undefined));
  const aloneLibrary = createLibraryServiceMcpDispatcher(libraryImpl);
  const aloneDepot = createDepotServiceMcpDispatcher(depotImpl);
  const aloneBare = createBareServiceMcpDispatcher(bareImpl);
  const over = createLibraryServiceMcpHttpDispatcher(libraryImpl, { allowedOrigins: [input.origin] }, async () => ({ ctx: {} }));
  const answers = (ask) => Promise.all(input.requests.map(async (text) => (await ask(text)) ?? null));
  const http = [];
  for (const row of input.http) {
    const response = await over({ method: "POST", path: "/mcp", headers: row.headers, body: new TextEncoder().encode(row.body) });
    http.push({ status: response.status, headers: response.headers.map(([name, value]) => [name, value]), body: new TextDecoder().decode(response.body) });
  }
  let shared = null;
  try {
    libraryServiceMcpEndpoint(...[libraryServiceMcpPart, mirrorServiceMcpPart]);
  } catch (thrown) {
    shared = String(thrown.message);
  }
  console.log(JSON.stringify({
    names: [libraryServiceMcpPart.names, depotServiceMcpPart.names, bareServiceMcpPart.names, mirrorServiceMcpPart.names],
    shared,
    workspace: await answers(served),
    library: await answers((text) => aloneLibrary(text, () => ({}), host.writer, host.inFlight)),
    depot: await answers((text) => aloneDepot(text, () => ({}), host.writer, host.inFlight)),
    bare: await answers((text) => aloneBare(text, () => ({}), host.writer, host.inFlight)),
    http,
  }));
  process.exit(0);
}
main().catch((error) => { console.error(error); process.exit(1); });
"##;

/// Answers the fault table's requests through the dispatcher of `ShelfService` alone and through its route.
const FAULT_DRIVER: &str = r#"
const shelfImpl = {
  async getSlot(ctx, req) {
    return { ok: true, value: `${req.aisle}/${req.slot}` };
  },
  async getCount(ctx, bin) {
    return { ok: true, value: String(bin) };
  },
  async suggestSlots(ctx, req) {
    return { ok: true, value: [req.typed] };
  },
};

async function main() {
  const requests = JSON.parse(INPUT);
  const host = { writer: async () => {}, inFlight: new ShelfServiceMcpInFlight() };
  const alone = createShelfServiceMcpDispatcher(shelfImpl);
  const route = createShelfServiceMcpRoute(shelfImpl);
  const endpoint = shelfServiceMcpEndpoint(shelfServiceMcpPart);
  const served = (text) => shelfServiceMcpServe(text, "Workspace", undefined, endpoint, () => ({}), host,
    async (ctx, part, method, params) => route(ctx, method, params));
  const answers = (ask) => Promise.all(requests.map(async (text) => (await ask(text)) ?? null));
  console.log(JSON.stringify({
    alone: await answers((text) => alone(text, () => ({}), host.writer, host.inFlight)),
    endpoint: await answers(served),
  }));
  process.exit(0);
}
main().catch((error) => { console.error(error); process.exit(1); });
"#;

/// An endpoint whose two parts both name `resource:guide://library`.
const SHARED_NAME_CALLER: &str = r#"import {
  libraryServiceMcpEndpoint,
  libraryServiceMcpPart,
  mirrorServiceMcpPart,
} from "./bundle.js";

export const shared = libraryServiceMcpEndpoint(libraryServiceMcpPart, mirrorServiceMcpPart);
"#;

/// A caller that reads the table and the page types the way a consumer does.
const USAGE_CALLER: &str = r#"import {
  createLibraryServiceMcpDispatcher,
  libraryServiceMcpOperations,
  libraryServiceMcpPart,
  type LibraryServiceImpl,
  type LibraryServiceMcpCall,
  type LibraryServiceMcpInFlight,
  type LibraryServiceMcpWriter,
  type LibraryServiceMcpResourcePage,
} from "./bundle.js";

declare const impl: LibraryServiceImpl<number>;

export const dispatcher: (
  text: string,
  ctx: (call: LibraryServiceMcpCall) => number,
  writer: LibraryServiceMcpWriter,
  inFlight: LibraryServiceMcpInFlight,
) => Promise<string | undefined> = createLibraryServiceMcpDispatcher(impl);
export const kinds: ReadonlyArray<string> = libraryServiceMcpOperations.map((operation) => operation.kind);
export const names: readonly ["resource template:library://covers/{book_id}", "tool:get-book", "resource:guide://library"] = libraryServiceMcpPart.names;
export const page: LibraryServiceMcpResourcePage = { resources: [{ uri: "library://covers/book-1", name: "cover-1.pdf" }], nextCursor: "page-2" };
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
        LibraryServiceSchema::ts_definition(),
        LibraryServiceSchema::ts_service(),
        LibraryServiceSchema::ts_mcp_service(),
        DepotServiceSchema::ts_definition(),
        DepotServiceSchema::ts_service(),
        DepotServiceSchema::ts_mcp_service(),
        BareServiceSchema::ts_definition(),
        BareServiceSchema::ts_service(),
        BareServiceSchema::ts_mcp_service(),
        MirrorServiceSchema::ts_definition(),
        MirrorServiceSchema::ts_service(),
        MirrorServiceSchema::ts_mcp_service(),
    ]
    .join("\n\n")
}

fn fault_bundle() -> String {
    [
        "import { z, type ZodType, type $brand } from \"zod\";".to_owned(),
        ShelfLabel::ts_definition(),
        ShelfLabel::zod_schema(),
        BookError::ts_definition(),
        BookError::zod_schema(),
        ShelfServiceSchema::ts_definition(),
        ShelfServiceSchema::ts_service(),
        ShelfServiceSchema::ts_mcp_service(),
    ]
    .join("\n\n")
}

/// The report of the fault run; `None` where no node toolchain was reachable.
fn faulted() -> Option<Value> {
    let Some(modules) = node_modules(REQUIRED_PACKAGES) else {
        stand_down_modules(
            REQUIRED_PACKAGES,
            "the emitted TypeScript MCP server over resource faults",
        );
        return None;
    };
    let literal = Value::String(json!(fault_requests()).to_string());
    let source = format!(
        "{}\n\nconst INPUT = {literal};\n{FAULT_DRIVER}",
        fault_bundle()
    );
    let wrote = ran_with_modules(
        "mcp-resource-faults",
        RUNTIME_VAR,
        "node",
        "mcp-resource-faults.mts",
        &source,
        &modules,
    )?;
    Some(serde_json::from_str(wrote.trim()).unwrap())
}

fn input(requests: &[Row], http: &[HttpRow]) -> Value {
    json!({
        "origin": ORIGIN,
        "requests": requests.iter().map(|row| row.text.clone()).collect::<Vec<_>>(),
        "http": http.iter().map(|row| json!({
            "headers": row.headers.iter().map(|(name, value)| json!([name, value])).collect::<Vec<_>>(),
            "body": row.body,
        })).collect::<Vec<_>>(),
    })
}

/// The report of the one node run; `None` where no node toolchain was reachable.
fn driven() -> Option<Value> {
    let Some(modules) = node_modules(REQUIRED_PACKAGES) else {
        stand_down_modules(
            REQUIRED_PACKAGES,
            "the emitted TypeScript MCP server over resources",
        );
        return None;
    };
    let literal = Value::String(input(&rows(), &http_rows()).to_string());
    let source = format!("{}\n\nconst INPUT = {literal};\n{DRIVER}", bundle());
    let wrote = ran_with_modules(
        "mcp-resources",
        RUNTIME_VAR,
        "node",
        "mcp-resources.mts",
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
    assert_answers(
        "endpoint",
        workspace_answer,
        &texts(&report["workspace"]),
        &rows,
    );
    assert_answers("library", library_answer, &texts(&report["library"]), &rows);
    assert_answers("depot", depot_answer, &texts(&report["depot"]), &rows);
    assert_answers("bare", bare_answer, &texts(&report["bare"]), &rows);
}

#[test]
fn an_endpoint_names_a_resource_and_a_template_by_their_kind_and_refuses_one_two_services_share() {
    let Some(report) = driven() else {
        return;
    };
    assert_eq!(
        report["names"],
        json!([
            [
                "resource template:library://covers/{book_id}",
                "tool:get-book",
                "resource:guide://library"
            ],
            [
                "resource template:depot://aisles/{aisle}/bins/{bin_id}",
                "resource template:depot://crates/{crate_id}"
            ],
            ["tool:get-plain"],
            [
                "resource template:library://covers/{book_id}",
                "resource:guide://library"
            ],
        ])
    );
    assert_eq!(
        report["names"][0],
        json!(
            LibraryServiceSchema::MCP_NAMES
                .iter()
                .map(|(kind, name)| format!("{kind}:{name}"))
                .collect::<Vec<_>>()
        )
    );
    assert_eq!(
        report["shared"],
        "two services share the name `resource template:library://covers/{book_id}`"
    );
}

#[test]
fn the_typescript_server_answers_resource_requests_over_plain_http_terms_as_rust_does() {
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
    assert_http("library", &http_rows(), &answered);
}

#[test]
fn the_resource_text_type_checks_under_strict() {
    let Some(modules) = node_modules(REQUIRED_PACKAGES) else {
        stand_down_modules(
            REQUIRED_PACKAGES,
            "the type-check of the emitted MCP text over resources",
        );
        return;
    };
    let text = bundle();
    let Some((succeeded, said)) = compiled_with_modules(
        "mcp-resource-types",
        COMPILER_VAR,
        &[("bundle.ts", &text), ("usage.ts", USAGE_CALLER)],
        &modules,
    ) else {
        return;
    };
    assert!(succeeded, "the emitted text does not type-check:\n{said}");
    let Some((refused, complaint)) = compiled_with_modules(
        "mcp-resource-shared",
        COMPILER_VAR,
        &[("bundle.ts", &text), ("shared.ts", SHARED_NAME_CALLER)],
        &modules,
    ) else {
        return;
    };
    assert!(
        !refused,
        "an endpoint of two services sharing a resource type-checks:\n{complaint}"
    );
    assert!(
        complaint.contains("twoServicesShare") && complaint.contains("resource:guide://library"),
        "tsc does not name the shared name:\n{complaint}"
    );
}

#[test]
fn a_resource_read_and_a_completion_the_zod_reads_refuse_answer_the_fault_the_rust_dispatcher_answers()
 {
    let Some(report) = faulted() else {
        return;
    };
    assert_faults("alone", &texts(&report["alone"]));
    assert_faults("endpoint", &texts(&report["endpoint"]));
}
