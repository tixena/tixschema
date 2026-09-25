//! The emitted `http_rest` server run under Node: the design's seven requests, the reader
//! forms, a macro-generated message read off the query and the body, a bound `header_in`
//! echoed back and compared with the Rust twin, and the three body kinds compared against the
//! Rust `{service}_http_rest_dispatcher!()` twin for the same request.
//!
//! Beside `node` itself, this leg reaches for the `zod` package through `TIXSCHEMA_NODE_MODULES`
//! — the real schemas a bad payload has to fail against, not the stubs `run_node.rs` names for
//! the URL-shaped client leg. `just test-emitted` resolves it up front and refuses to stand down.

use super::content_http_rest_transport;
use super::echo_http_rest_transport;
use super::pulse_http_rest_transport;
use super::runtime::{node_modules, ran_with_modules, stand_down_modules};
use super::tests::{
    ArchiveClientServiceSchema, ArchiveError, ArchiveStatus, ContentBackEnd,
    ContentClientServiceSchema, ContentError, ConversationClientServiceSchema, ConversationId,
    EchoBackEnd, EchoClientServiceSchema, EchoRangeError, EchoRangeResponse,
    GateClientServiceSchema, GateError, GateStatus, LabelClientServiceSchema, LabelError,
    LabelStatus, PulseBackEnd, PulseClientServiceSchema, PulseError, PulseResponse,
    SealClientServiceSchema, SealError, SealStatus, SearchClientServiceSchema, SearchEcho,
    SearchError, ThumbnailBackEnd, ThumbnailClientServiceSchema, ThumbnailError,
    UploadDocumentBackEnd, UploadDocumentClientServiceSchema, UploadDocumentError,
    UploadDocumentResponse, VaultClientServiceSchema, VaultError, VaultStatus, WindowError,
    WindowPage,
};
use super::thumbnail_http_rest_transport;
use super::upload_document_http_rest_transport;
use core::future::Future;
use core::pin::pin;
use core::task::{Context as PollContext, Poll, Waker};
use std::io;

/// Names the runtime to run, for a machine that has one somewhere other than `PATH`.
const RUNTIME_VAR: &str = "TIXSCHEMA_NODE";

/// The one package this leg's driver imports at runtime: `zod`, for the schemas the emitted
/// dispatcher checks a payload against.
const REQUIRED_PACKAGES: &[&str] = &["zod"];

/// The design's own Node `http` adapter (section 3), and the implementation it serves: `missing`
/// answers `not-found`, `boom` throws, everything else answers the three-item page.
const SEVEN_REQUESTS_DRIVER: &str = r#"
const impl = {
  async purgeConversation() {},
  async window(ctx, req) {
    if (req.conversationId === "missing") return { ok: false, error: { errorCode: "not-found" } };
    if (req.conversationId === "boom") throw new Error("the handler came apart");
    return { ok: true, value: { items: [req.conversationId, `limit=${req.limit}`, "request 1"] } };
  },
};

const dispatch = createConversationClientServiceHttpDispatcher(impl);
const server = createServer(async (req, res) => {
  const chunks = [];
  for await (const chunk of req) chunks.push(chunk);
  const url = new URL(req.url ?? "/", "http://localhost");
  const answered = await dispatch(
    {},
    {
      body: new Uint8Array(Buffer.concat(chunks)),
      headers: Object.entries(req.headers).map(([name, value]) => [name, Array.isArray(value) ? value.join(",") : String(value ?? "")]),
      method: req.method ?? "GET",
      path: url.pathname,
      query: url.search.startsWith("?") ? url.search.slice(1) : url.search,
    },
  );
  res.writeHead(answered.status, Object.fromEntries(answered.headers));
  res.end(Buffer.from(answered.body));
});

async function main() {
  await new Promise((resolve) => server.listen(0, resolve));
  const port = server.address().port;

  async function request(method, path) {
    const response = await fetch(`http://127.0.0.1:${port}${path}`, { method });
    const contentType = response.headers.get("content-type");
    const text = await response.text();
    let body;
    try { body = JSON.parse(text); } catch { body = text; }
    return { status: response.status, contentType, body };
  }

  const results = [
    await request("GET", "/v1/conversations/652f1a3b4c5d6e7f8a9b0c1d/window?limit=10"),
    await request("GET", "/v1/conversations/missing/window"),
    await request("GET", "/v1/conversations/boom/window"),
    await request("DELETE", "/v1/conversations/652f1a3b4c5d6e7f8a9b0c1d"),
    await request("GET", "/v1/nothing/here"),
    await request("POST", "/v1/conversations/652f1a3b4c5d6e7f8a9b0c1d/window"),
    await request("GET", "/v1/conversations//window"),
  ];
  server.close();
  console.log(JSON.stringify(results));
  process.exit(0);
}
main().catch((error) => { console.error(error); process.exit(1); });
"#;

/// One call per case, straight against the exported dispatcher — no listening server. `ArchiveError`
/// carries a payload variant, which cannot be named in an `error_status` table, so
/// `ArchiveError$Variant` is read directly instead, off two hand-built wire values.
const READER_FORMS_DRIVER: &str = r#"
function req(method, path, body = new Uint8Array()) {
  return { method, path, query: "", headers: [], body };
}
function jsonBody(value) {
  return new TextEncoder().encode(JSON.stringify(value));
}
async function status(createDispatcher, impl, request) {
  const dispatch = createDispatcher(impl);
  const answered = await dispatch({}, request);
  return answered.status;
}

async function main() {
  const results = {};

  const gateImpl = {
    async checkGate(ctx, gateId) {
      if (gateId === "missing") return { ok: false, error: { kind: "not-found" } };
      if (gateId === "locked") return { ok: false, error: { kind: "sealed" } };
      return { ok: true, value: { open: true } };
    },
    async checkGateUnmapped() {
      return { ok: false, error: { kind: "not-found" } };
    },
  };
  results.gateMissing = await status(createGateClientServiceHttpDispatcher, gateImpl, req("GET", "/gates/missing"));
  results.gateLocked = await status(createGateClientServiceHttpDispatcher, gateImpl, req("GET", "/gates/locked"));
  results.gateUnmapped = await status(createGateClientServiceHttpDispatcher, gateImpl, req("POST", "/check-gate-unmapped", jsonBody("g1")));

  const vaultImpl = {
    async checkVault(ctx, vaultId) {
      if (vaultId === "missing") return { ok: false, error: { kind: "not-found" } };
      if (vaultId === "locked") return { ok: false, error: { kind: "bolted" } };
      return { ok: true, value: { open: true } };
    },
    async checkVaultUnmapped() {
      return { ok: false, error: { kind: "not-found" } };
    },
  };
  results.vaultMissing = await status(createVaultClientServiceHttpDispatcher, vaultImpl, req("GET", "/vaults/missing"));
  results.vaultLocked = await status(createVaultClientServiceHttpDispatcher, vaultImpl, req("GET", "/vaults/locked"));
  results.vaultUnmapped = await status(createVaultClientServiceHttpDispatcher, vaultImpl, req("POST", "/check-vault-unmapped", jsonBody("v1")));

  const sealImpl = {
    async checkSeal(ctx, sealId) {
      if (sealId === "missing") return { ok: false, error: "NotFound" };
      if (sealId === "locked") return { ok: false, error: "sealed-shut" };
      return { ok: true, value: { sealed: true } };
    },
    async checkSealUnmapped() {
      return { ok: false, error: "NotFound" };
    },
  };
  results.sealMissing = await status(createSealClientServiceHttpDispatcher, sealImpl, req("GET", "/seals/missing"));
  results.sealLocked = await status(createSealClientServiceHttpDispatcher, sealImpl, req("GET", "/seals/locked"));
  results.sealUnmapped = await status(createSealClientServiceHttpDispatcher, sealImpl, req("POST", "/check-seal-unmapped", jsonBody("s1")));

  results.archiveReaderNotFound = ArchiveError$Variant("NotFound");
  results.archiveReaderLocked = ArchiveError$Variant({ "vault-locked": { retry_after: 30 } });
  const archiveImpl = {
    async checkArchive() {
      return { ok: false, error: "NotFound" };
    },
  };
  results.archiveUnmapped = await status(createArchiveClientServiceHttpDispatcher, archiveImpl, req("POST", "/check-archive", jsonBody("a1")));

  console.log(JSON.stringify(results));
  process.exit(0);
}
main().catch((error) => { console.error(error); process.exit(1); });
"#;

const SEARCH_DRIVER: &str = r#"
async function main() {
  const impl = {
    async search(ctx, req) {
      return { ok: true, value: { limit: req.limit, verbose: req.verbose } };
    },
    async searchBody(ctx, req) {
      return { ok: true, value: { limit: req.limit, verbose: req.verbose } };
    },
  };
  const dispatch = createSearchClientServiceHttpDispatcher(impl);

  async function answered(request) {
    const response = await dispatch({}, request);
    const text = new TextDecoder().decode(response.body);
    let body;
    try { body = JSON.parse(text); } catch { body = text; }
    return { status: response.status, body };
  }

  const results = {
    query: await answered({ method: "GET", path: "/search", query: "limit=10&verbose=true", headers: [], body: new Uint8Array() }),
    body: await answered({ method: "POST", path: "/search", query: "", headers: [], body: new TextEncoder().encode(JSON.stringify({ limit: 10, verbose: true })) }),
    bad: await answered({ method: "POST", path: "/search", query: "", headers: [], body: new TextEncoder().encode(JSON.stringify({ limit: "abc", verbose: true })) }),
  };
  console.log(JSON.stringify(results));
  process.exit(0);
}
main().catch((error) => { console.error(error); process.exit(1); });
"#;

const BYTES_DRIVER: &str = r#"
async function main() {
  const impl = {
    async getThumbnail(ctx, documentId) {
      if (documentId === "missing") return { ok: false, error: [{ errorCode: "not-found" }, "archived"] };
      if (documentId === "gone") return { ok: false, error: [{ errorCode: "not-found" }, undefined] };
      if (documentId === "anon") return { ok: true, value: [new Uint8Array([0x89, 0x50, 0x4e, 0x47]), "image/png", undefined] };
      return { ok: true, value: [new Uint8Array([0x89, 0x50, 0x4e, 0x47]), "image/png", `doc-${documentId}`] };
    },
  };
  const dispatch = createThumbnailClientServiceHttpDispatcher(impl);

  async function answered(path) {
    const response = await dispatch({}, { method: "GET", path, query: "", headers: [], body: new Uint8Array() });
    return { status: response.status, headers: response.headers, bodyBytes: Array.from(response.body) };
  }

  const results = {
    ok: await answered("/thumbnails/present"),
    missing: await answered("/thumbnails/missing"),
    gone: await answered("/thumbnails/gone"),
    anon: await answered("/thumbnails/anon"),
  };
  console.log(JSON.stringify(results));
  process.exit(0);
}
main().catch((error) => { console.error(error); process.exit(1); });
"#;

/// `getFile`'s own signature carries no `byte_range`: it declares no `header_in` binding at all,
/// so the answer is always the full body — the one shape both dispatchers can agree on.
const STREAM_DRIVER: &str = r#"
async function main() {
  const content = new TextEncoder().encode("the quick brown fox jumps over the lazy dog");
  const impl = {
    async getFile(ctx, fileId) {
      if (fileId === "missing") return { ok: false, error: { errorCode: "not-found" } };
      return {
        ok: true,
        value: {
          contentRange: undefined,
          body: new ReadableStream({
            start(controller) {
              controller.enqueue(content);
              controller.close();
            },
          }),
        },
      };
    },
  };
  const dispatch = createContentClientServiceHttpDispatcher(impl);

  async function drain(body) {
    if (body instanceof Uint8Array) return body;
    const reader = body.getReader();
    const chunks = [];
    for (;;) {
      const { done, value } = await reader.read();
      if (done) break;
      chunks.push(value);
    }
    const total = chunks.reduce((sum, chunk) => sum + chunk.length, 0);
    const out = new Uint8Array(total);
    let offset = 0;
    for (const chunk of chunks) {
      out.set(chunk, offset);
      offset += chunk.length;
    }
    return out;
  }

  async function answered(path) {
    const response = await dispatch({}, { method: "GET", path, query: "", headers: [], body: new Uint8Array() });
    const bytes = await drain(response.body);
    return { status: response.status, headers: response.headers, bodyBytes: Array.from(bytes) };
  }

  const results = {
    full: await answered("/files/present"),
    missing: await answered("/files/missing"),
  };
  console.log(JSON.stringify(results));
  process.exit(0);
}
main().catch((error) => { console.error(error); process.exit(1); });
"#;

/// `attachment`'s own bytes reach the implementation (as `unknown`) but the driver below never
/// reads them, so the success value it answers with is built from `folder_id`, `title` and
/// whether `description` carried a part — nothing the Rust side alone can see.
const MULTIPART_DRIVER: &str = r#"
async function main() {
  const impl = {
    async uploadDocument(ctx, req) {
      if (req.title === "toolarge") return { ok: false, error: { errorCode: "too-large" } };
      return { ok: true, value: { document_id: `doc-${req.folderId}-${req.title}-${req.description != null}` } };
    },
  };
  const dispatch = createUploadDocumentClientServiceHttpDispatcher(impl);

  async function answered(parts) {
    const response = await dispatch(
      {},
      { method: "POST", path: "/folders/acme/documents", query: "", headers: [], body: new Uint8Array(), parts },
    );
    return { status: response.status, headers: response.headers, bodyBytes: Array.from(response.body) };
  }

  const results = {
    ok: await answered([["title", "quarterly-report"], ["description", "Q3 numbers"], ["file", "stand-in"]]),
    tooLarge: await answered([["title", "toolarge"], ["file", "stand-in"]]),
    missingFile: await answered([["title", "no-file"]]),
  };
  console.log(JSON.stringify(results));
  process.exit(0);
}
main().catch((error) => { console.error(error); process.exit(1); });
"#;

/// A required `header_in` binding, echoed back by the implementation: present, the value it was
/// bound reaches `impl.echoRange` as its own argument after the message; absent, the dispatcher
/// refuses before the implementation is ever called.
const ECHO_DRIVER: &str = r#"
async function main() {
  const impl = {
    async echoRange(ctx, req, byteRange) {
      return { ok: true, value: { received: byteRange } };
    },
  };
  const dispatch = createEchoClientServiceHttpDispatcher(impl);

  async function answered(headers) {
    const response = await dispatch(
      {},
      { method: "GET", path: "/echo/d1", query: "", headers, body: new Uint8Array() },
    );
    return { status: response.status, headers: response.headers, bodyBytes: Array.from(response.body) };
  }

  const results = {
    present: await answered([["range", "bytes=0-9"]]),
    absent: await answered([]),
  };
  console.log(JSON.stringify(results));
  process.exit(0);
}
main().catch((error) => { console.error(error); process.exit(1); });
"#;

/// A bodyless `GET` carrying no field beside the context at all.
const PULSE_DRIVER: &str = r#"
async function main() {
  const impl = {
    async pulse() {
      return { ok: true, value: { alive: true } };
    },
  };
  const dispatch = createPulseClientServiceHttpDispatcher(impl);
  const response = await dispatch(
    {},
    { method: "GET", path: "/pulse", query: "", headers: [], body: new Uint8Array() },
  );
  console.log(JSON.stringify({ status: response.status, headers: response.headers, bodyBytes: Array.from(response.body) }));
  process.exit(0);
}
main().catch((error) => { console.error(error); process.exit(1); });
"#;

/// A bodyless `GET` whose macro-generated message's two fields are both bound by the path -
/// nothing left for the emitted dispatcher to read off the query string.
const LABEL_DRIVER: &str = r#"
async function main() {
  const impl = {
    async getLabel(ctx, req) {
      if (req.labelId === "missing") return { ok: false, error: { errorCode: "not-found" } };
      return { ok: true, value: { label: `${req.orgId}/${req.labelId}` } };
    },
  };
  const dispatch = createLabelClientServiceHttpDispatcher(impl);

  async function answered(path) {
    const response = await dispatch({}, { method: "GET", path, query: "", headers: [], body: new Uint8Array() });
    const text = new TextDecoder().decode(response.body);
    return { status: response.status, body: JSON.parse(text) };
  }

  const results = {
    ok: await answered("/orgs/acme/labels/priority"),
    missing: await answered("/orgs/acme/labels/missing"),
  };
  console.log(JSON.stringify(results));
  process.exit(0);
}
main().catch((error) => { console.error(error); process.exit(1); });
"#;

/// A stub `Transport` answering by path alone, driving the emitted client's own declared-error
/// and `header_out` decode - the client-side twin of [`bytes_body_kind_agrees_with_rust`], which
/// drives the same three cases through the server.
const THUMBNAIL_CLIENT_DRIVER: &str = r#"
const transport = {
  async send(request) {
    if (request.path === "/thumbnails/missing") {
      return { status: 404, headers: [["x-thumbnail-reason", "archived"]], body: JSON.stringify({ errorCode: "not-found" }) };
    }
    if (request.path === "/thumbnails/gone") {
      return { status: 404, headers: [], body: JSON.stringify({ errorCode: "not-found" }) };
    }
    return { status: 200, headers: [["content-type", "image/png"]], body: "PNGDATA" };
  },
};
const client = createThumbnailClientServiceHttpClient(transport);
const results = {
  missing: await client.getThumbnail("missing"),
  gone: await client.getThumbnail("gone"),
  anon: await client.getThumbnail("anon"),
};
console.log(JSON.stringify(results));
"#;

/// A stub `Transport` recording whether `send` was ever called, driving the emitted client's own
/// `header_in` legality check on a value carrying a line feed.
const ECHO_CLIENT_DRIVER: &str = r#"
let sendCalled = false;
const transport = {
  async send(request) {
    sendCalled = true;
    return { status: 200, headers: [], body: JSON.stringify({ received: "unreachable" }) };
  },
};
const client = createEchoClientServiceHttpClient(transport);
const refused = await client.echoRange("doc-1", "bytes=0-10\nX-Injected: yes");
console.log(JSON.stringify({ refused, sendCalled }));
"#;

/// A legal value carrying a space and a quoted `ETag` together, driving the emitted client's own
/// `header_in` legality check the other way: it must reach the transport unchanged.
const ECHO_CLIENT_LEGAL_DRIVER: &str = r#"
let sentHeaders = null;
const transport = {
  async send(request) {
    sentHeaders = request.headers;
    return { status: 200, headers: [], body: JSON.stringify({ received: "unreachable" }) };
  },
};
const client = createEchoClientServiceHttpClient(transport);
const answered = await client.echoRange("doc-1", "a \"etag\" b");
console.log(JSON.stringify({ answered, sentHeaders }));
"#;

// -------------------------------------------------------------------------------------------
// Group 1: the design's own seven requests, against the design's own Node `http` adapter.
// -------------------------------------------------------------------------------------------

/// One answer in plain terms, normalized for comparison: headers sorted, so an incidental
/// ordering difference between the two dispatchers is not mistaken for a disagreement.
#[derive(Debug, PartialEq, Eq)]
struct Answered {
    body: Vec<u8>,
    headers: Vec<(String, String)>,
    status: u16,
}

struct RecordingFaultHandler;

impl thumbnail_http_rest_transport::FaultHandler for RecordingFaultHandler {
    fn on_fault(
        &self,
        fault: &super::tests::thumbnail_client_service_schema::ServiceFault,
    ) -> thumbnail_http_rest_transport::OutgoingResponse {
        thumbnail_http_rest_transport::OutgoingResponse::new(
            499,
            vec![("x-fault-kind".to_owned(), format!("{}", fault.kind()))],
            b"handled".to_vec(),
        )
    }
}

impl content_http_rest_transport::FaultHandler for RecordingFaultHandler {
    fn on_fault(
        &self,
        fault: &super::tests::content_client_service_schema::ServiceFault,
    ) -> content_http_rest_transport::OutgoingResponse {
        content_http_rest_transport::OutgoingResponse::new(
            499,
            vec![("x-fault-kind".to_owned(), format!("{}", fault.kind()))],
            content_http_rest_transport::OutgoingBody::Bytes(b"handled".to_vec()),
        )
    }
}

impl upload_document_http_rest_transport::FaultHandler for RecordingFaultHandler {
    fn on_fault(
        &self,
        fault: &super::tests::upload_document_client_service_schema::ServiceFault,
    ) -> upload_document_http_rest_transport::OutgoingResponse {
        upload_document_http_rest_transport::OutgoingResponse::new(
            499,
            vec![("x-fault-kind".to_owned(), format!("{}", fault.kind()))],
            b"handled".to_vec(),
        )
    }
}

impl echo_http_rest_transport::FaultHandler for RecordingFaultHandler {
    fn on_fault(
        &self,
        fault: &super::tests::echo_client_service_schema::ServiceFault,
    ) -> echo_http_rest_transport::OutgoingResponse {
        echo_http_rest_transport::OutgoingResponse::new(
            499,
            vec![("x-fault-kind".to_owned(), format!("{}", fault.kind()))],
            b"handled".to_vec(),
        )
    }
}

impl pulse_http_rest_transport::FaultHandler for RecordingFaultHandler {
    fn on_fault(
        &self,
        fault: &super::tests::pulse_client_service_schema::ServiceFault,
    ) -> pulse_http_rest_transport::OutgoingResponse {
        pulse_http_rest_transport::OutgoingResponse::new(
            499,
            vec![("x-fault-kind".to_owned(), format!("{}", fault.kind()))],
            b"handled".to_vec(),
        )
    }
}

/// Runs `source` under Node, or stands down (naming `TIXSCHEMA_NODE_MODULES`) when `zod` is not
/// reachable through it.
fn run(named: &str, entry: &str, source: &str) -> Option<serde_json::Value> {
    let modules = node_modules(REQUIRED_PACKAGES)?;
    let wrote = ran_with_modules(named, RUNTIME_VAR, "node", entry, source, &modules)?;
    Some(serde_json::from_str(wrote.trim()).unwrap())
}

/// [`run`], standing down loudly rather than silently when no answer came back.
fn run_or_stand_down(named: &str, entry: &str, source: &str) -> Option<serde_json::Value> {
    let answered = run(named, entry, source);
    if answered.is_none() {
        stand_down_modules(REQUIRED_PACKAGES, "the emitted TypeScript REST server");
    }
    answered
}

fn conversation_emitted() -> String {
    [
        "import { z } from \"zod\";".to_owned(),
        "import { createServer } from \"node:http\";".to_owned(),
        WindowPage::ts_definition(),
        WindowPage::zod_schema(),
        WindowError::ts_definition(),
        WindowError::zod_schema(),
        ConversationId::ts_definition(),
        ConversationId::zod_schema(),
        ConversationClientServiceSchema::ts_definition(),
        ConversationClientServiceSchema::ts_service(),
        ConversationClientServiceSchema::ts_http_service(),
    ]
    .join("\n\n")
}

#[test]
fn the_seven_requests_answer_as_the_design_recorded() {
    let module = format!("{}\n\n{SEVEN_REQUESTS_DRIVER}", conversation_emitted());
    let Some(answered) = run_or_stand_down("http-service-seven", "seven.mts", &module) else {
        return;
    };
    let results = answered.as_array().unwrap();
    assert_eq!(results.len(), 7, "got: {results:#?}");

    assert_eq!(results[0]["status"], 200_i64, "got: {results:#?}");
    assert_eq!(
        results[0]["contentType"], "application/json",
        "got: {results:#?}"
    );
    assert_eq!(
        results[0]["body"],
        serde_json::json!({"items": ["652f1a3b4c5d6e7f8a9b0c1d", "limit=10", "request 1"]}),
        "got: {results:#?}"
    );

    assert_eq!(results[1]["status"], 404_i64, "got: {results:#?}");
    assert_eq!(
        results[1]["body"],
        serde_json::json!({"errorCode": "not-found"}),
        "got: {results:#?}"
    );

    assert_eq!(results[2]["status"], 500_i64, "got: {results:#?}");
    assert_eq!(
        results[2]["body"],
        serde_json::json!({
            "detail": "the handler came apart", "kind": "handler-panic", "operation": "window",
        }),
        "got: {results:#?}"
    );

    assert_eq!(results[3]["status"], 204_i64, "got: {results:#?}");
    assert_eq!(
        results[3]["contentType"],
        serde_json::Value::Null,
        "got: {results:#?}"
    );
    assert_eq!(results[3]["body"], "", "got: {results:#?}");

    for (index, method, path) in [
        (4_usize, "GET", "/v1/nothing/here"),
        (
            5,
            "POST",
            "/v1/conversations/652f1a3b4c5d6e7f8a9b0c1d/window",
        ),
        (6, "GET", "/v1/conversations//window"),
    ] {
        assert_eq!(results[index]["status"], 404_i64, "got: {results:#?}");
        assert_eq!(
            results[index]["body"],
            serde_json::json!({
                "detail": "the service answers to no route by that method and path",
                "kind": "unknown-operation",
                "operation": format!("{method} {path}"),
            }),
            "got: {results:#?}"
        );
    }
}

// -------------------------------------------------------------------------------------------
// Group 2: the four reader forms, three status cases each.
// -------------------------------------------------------------------------------------------

fn reader_forms_emitted() -> String {
    [
        "import { z } from \"zod\";".to_owned(),
        GateStatus::ts_definition(),
        GateStatus::zod_schema(),
        GateError::ts_definition(),
        GateError::zod_schema(),
        GateClientServiceSchema::ts_definition(),
        GateClientServiceSchema::ts_service(),
        GateClientServiceSchema::ts_http_service(),
        VaultStatus::ts_definition(),
        VaultStatus::zod_schema(),
        VaultError::ts_definition(),
        VaultError::zod_schema(),
        VaultClientServiceSchema::ts_definition(),
        VaultClientServiceSchema::ts_service(),
        VaultClientServiceSchema::ts_http_service(),
        SealStatus::ts_definition(),
        SealStatus::zod_schema(),
        SealError::ts_definition(),
        SealError::zod_schema(),
        SealClientServiceSchema::ts_definition(),
        SealClientServiceSchema::ts_service(),
        SealClientServiceSchema::ts_http_service(),
        ArchiveStatus::ts_definition(),
        ArchiveStatus::zod_schema(),
        ArchiveError::ts_definition(),
        ArchiveError::zod_schema(),
        ArchiveClientServiceSchema::ts_definition(),
        ArchiveClientServiceSchema::ts_service(),
        ArchiveClientServiceSchema::ts_http_service(),
    ]
    .join("\n\n")
}

#[test]
fn each_reader_form_picks_the_mapped_status() {
    let module = format!("{}\n\n{READER_FORMS_DRIVER}", reader_forms_emitted());
    let Some(results) = run_or_stand_down("http-service-reader-forms", "reader-forms.mts", &module)
    else {
        return;
    };

    for (key, expected) in [
        ("gateMissing", 404_i64),
        ("gateLocked", 423_i64),
        ("gateUnmapped", 422_i64),
        ("vaultMissing", 404_i64),
        ("vaultLocked", 423_i64),
        ("vaultUnmapped", 422_i64),
        ("sealMissing", 404_i64),
        ("sealLocked", 423_i64),
        ("sealUnmapped", 422_i64),
        ("archiveUnmapped", 422_i64),
    ] {
        assert_eq!(results[key], expected, "{key}. got: {results:#?}");
    }
    assert_eq!(
        results["archiveReaderNotFound"], "NotFound",
        "the bare-string wire form. got: {results:#?}"
    );
    assert_eq!(
        results["archiveReaderLocked"], "Locked",
        "the sole-key wire form. got: {results:#?}"
    );
}

// -------------------------------------------------------------------------------------------
// Group 3: a macro-generated message read off the query on a bodyless `GET` and off the body on
// a `POST`, plus the 400 `failed-validation` a bad payload earns against the real schema.
// -------------------------------------------------------------------------------------------

fn search_emitted() -> String {
    [
        "import { z } from \"zod\";".to_owned(),
        SearchEcho::ts_definition(),
        SearchEcho::zod_schema(),
        SearchError::ts_definition(),
        SearchError::zod_schema(),
        SearchClientServiceSchema::ts_definition(),
        SearchClientServiceSchema::ts_service(),
        SearchClientServiceSchema::ts_http_service(),
    ]
    .join("\n\n")
}

#[test]
fn a_generated_message_is_read_off_the_query_and_the_body() {
    let module = format!("{}\n\n{SEARCH_DRIVER}", search_emitted());
    let Some(results) = run_or_stand_down("http-service-search", "search.mts", &module) else {
        return;
    };

    assert_eq!(results["query"]["status"], 200_i64, "got: {results:#?}");
    assert_eq!(
        results["query"]["body"],
        serde_json::json!({"limit": 10_i64, "verbose": true}),
        "coerced off the query string. got: {results:#?}"
    );

    assert_eq!(results["body"]["status"], 200_i64, "got: {results:#?}");
    assert_eq!(
        results["body"]["body"],
        serde_json::json!({"limit": 10_i64, "verbose": true}),
        "read off the parsed JSON body. got: {results:#?}"
    );

    assert_eq!(results["bad"]["status"], 400_i64, "got: {results:#?}");
    assert_eq!(
        results["bad"]["body"]["kind"], "failed-validation",
        "got: {results:#?}"
    );
    assert_eq!(
        results["bad"]["body"]["field"], "limit",
        "got: {results:#?}"
    );
}

// -------------------------------------------------------------------------------------------
// Group 4: one `bytes`, one `stream` and one `multipart` operation, each answer compared
// byte-for-byte with the Rust `{service}_http_rest_dispatcher!()` twin.
// -------------------------------------------------------------------------------------------

fn sorted(mut headers: Vec<(String, String)>) -> Vec<(String, String)> {
    headers.sort();
    headers
}

fn node_answered(value: &serde_json::Value) -> Answered {
    Answered {
        body: value["bodyBytes"]
            .as_array()
            .unwrap()
            .iter()
            .map(|byte| u8::try_from(byte.as_u64().unwrap()).unwrap())
            .collect(),
        headers: sorted(
            value["headers"]
                .as_array()
                .unwrap()
                .iter()
                .map(|entry| {
                    let fields = entry.as_array().unwrap();
                    (
                        fields[0].as_str().unwrap().to_owned(),
                        fields[1].as_str().unwrap().to_owned(),
                    )
                })
                .collect(),
        ),
        status: u16::try_from(value["status"].as_u64().unwrap()).unwrap(),
    }
}

/// The transports never suspend, so one poll answers them — `None` says that assumption stopped
/// holding rather than that the runtime is missing.
fn poll_once<Answering>(answering: Answering) -> Option<Answering::Output>
where
    Answering: Future,
{
    let mut pinned = pin!(answering);
    let mut polling = PollContext::from_waker(Waker::noop());
    match pinned.as_mut().poll(&mut polling) {
        Poll::Ready(answer) => Some(answer),
        Poll::Pending => None,
    }
}

fn thumbnail_emitted() -> String {
    [
        "import { z } from \"zod\";".to_owned(),
        ThumbnailError::ts_definition(),
        ThumbnailError::zod_schema(),
        ThumbnailClientServiceSchema::ts_definition(),
        ThumbnailClientServiceSchema::ts_service(),
        ThumbnailClientServiceSchema::ts_http_service(),
    ]
    .join("\n\n")
}

#[test]
fn bytes_stream_and_multipart_answer_what_the_rust_dispatcher_answers() {
    bytes_body_kind_agrees_with_rust();
    stream_body_kind_agrees_with_rust();
    multipart_body_kind_agrees_with_rust();
}

fn thumbnail_rust_answered(document_id: &str) -> Answered {
    let request = thumbnail_http_rest_transport::IncomingRequest::new(
        "GET".to_owned(),
        format!("/thumbnails/{document_id}"),
        String::new(),
        Vec::new(),
        Vec::new(),
    );
    let response = poll_once(thumbnail_http_rest_transport::dispatch(
        &ThumbnailBackEnd,
        &(),
        &request,
        &thumbnail_http_rest_transport::DefaultFaultHandler,
    ))
    .unwrap();
    Answered {
        body: response.body().to_vec(),
        headers: sorted(response.headers().to_vec()),
        status: response.status(),
    }
}

fn bytes_body_kind_agrees_with_rust() {
    let module = format!("{}\n\n{BYTES_DRIVER}", thumbnail_emitted());
    let Some(results) = run_or_stand_down("http-service-bytes", "bytes.mts", &module) else {
        return;
    };
    let node_ok = node_answered(&results["ok"]);
    let node_missing = node_answered(&results["missing"]);
    let node_gone = node_answered(&results["gone"]);
    let node_anon = node_answered(&results["anon"]);
    let rust_ok = thumbnail_rust_answered("present");
    let rust_missing = thumbnail_rust_answered("missing");
    let rust_gone = thumbnail_rust_answered("gone");
    let rust_anon = thumbnail_rust_answered("anon");
    assert_eq!(node_ok, rust_ok, "node: {node_ok:#?}, rust: {rust_ok:#?}");
    assert_eq!(
        node_missing, rust_missing,
        "node: {node_missing:#?}, rust: {rust_missing:#?}"
    );
    assert_eq!(
        node_gone, rust_gone,
        "node: {node_gone:#?}, rust: {rust_gone:#?}"
    );
    assert_eq!(
        node_anon, rust_anon,
        "node: {node_anon:#?}, rust: {rust_anon:#?}"
    );
    assert_eq!(rust_ok.status, 200, "got: {rust_ok:#?}");
    assert_eq!(rust_missing.status, 404, "got: {rust_missing:#?}");
    assert!(
        rust_missing
            .headers
            .contains(&("x-thumbnail-reason".to_owned(), "archived".to_owned())),
        "the declared error's own `error_header_out` entry must reach the response. \
         got: {rust_missing:#?}"
    );
    assert!(
        rust_gone
            .headers
            .iter()
            .all(|(name, _)| name != "x-thumbnail-reason"),
        "a `None` `error_header_out` element must omit the header, on both the Rust and the \
         Node-run TypeScript server. got: {rust_gone:#?}"
    );
    assert!(
        rust_anon
            .headers
            .iter()
            .all(|(name, _)| name != "x-document-id"),
        "a `None` `header_out` element must omit the header, on both the Rust and the \
         Node-run TypeScript server. got: {rust_anon:#?}"
    );
}

fn thumbnail_client_emitted() -> String {
    [
        "import { z } from \"zod\";".to_owned(),
        ThumbnailError::ts_definition(),
        ThumbnailError::zod_schema(),
        ThumbnailClientServiceSchema::ts_definition(),
        ThumbnailClientServiceSchema::ts_http_client(),
    ]
    .join("\n\n")
}

#[test]
fn the_client_decodes_the_declared_errors_own_header_and_omits_a_none_header_out_element() {
    let module = format!(
        "{}\n\n{THUMBNAIL_CLIENT_DRIVER}",
        thumbnail_client_emitted()
    );
    let Some(results) = run_or_stand_down("http-client-thumbnail", "thumbnail-client.mts", &module)
    else {
        return;
    };
    assert_eq!(
        results["missing"],
        serde_json::json!({"ok": false, "error": [{"errorCode": "not-found"}, "archived"]}),
        "the declared error's own head and its `error_header_out` element must both decode. \
         got: {results:#?}"
    );
    assert_eq!(
        results["gone"],
        serde_json::json!({"ok": false, "error": [{"errorCode": "not-found"}, null]}),
        "an absent `error_header_out` header must decode as the tuple's own `undefined`, not a \
         string. got: {results:#?}"
    );
    assert_eq!(
        results["anon"],
        serde_json::json!({"ok": true, "value": ["PNGDATA", "image/png", null]}),
        "an absent `header_out` header must decode as the tuple's own `undefined`, not a \
         string. got: {results:#?}"
    );
}

fn content_emitted() -> String {
    [
        "import { z } from \"zod\";".to_owned(),
        ContentError::ts_definition(),
        ContentError::zod_schema(),
        ContentClientServiceSchema::ts_definition(),
        ContentClientServiceSchema::ts_service(),
        ContentClientServiceSchema::ts_http_service(),
    ]
    .join("\n\n")
}

fn drain_outgoing(body: content_http_rest_transport::OutgoingBody) -> Vec<u8> {
    match body {
        content_http_rest_transport::OutgoingBody::Bytes(bytes) => bytes,
        content_http_rest_transport::OutgoingBody::Stream(mut source) => {
            let mut drained = Vec::new();
            let mut buf = [0_u8; 64];
            loop {
                let read = source.pull(&mut buf).unwrap();
                if read == 0 {
                    break;
                }
                drained.extend_from_slice(&buf[..read]);
            }
            drained
        }
    }
}

fn content_rust_answered(file_id: &str) -> Answered {
    let request = content_http_rest_transport::IncomingRequest::new(
        "GET".to_owned(),
        format!("/files/{file_id}"),
        String::new(),
        Vec::new(),
        Vec::new(),
    );
    let response = poll_once(content_http_rest_transport::dispatch(
        &ContentBackEnd,
        &(),
        &request,
        &content_http_rest_transport::DefaultFaultHandler,
    ))
    .unwrap();
    let status = response.status();
    let headers = sorted(response.headers().to_vec());
    Answered {
        body: drain_outgoing(response.into_body()),
        headers,
        status,
    }
}

fn stream_body_kind_agrees_with_rust() {
    let module = format!("{}\n\n{STREAM_DRIVER}", content_emitted());
    let Some(results) = run_or_stand_down("http-service-stream", "stream.mts", &module) else {
        return;
    };
    let node_full = node_answered(&results["full"]);
    let node_missing = node_answered(&results["missing"]);
    let rust_full = content_rust_answered("present");
    let rust_missing = content_rust_answered("missing");
    assert_eq!(
        node_full, rust_full,
        "node: {node_full:#?}, rust: {rust_full:#?}"
    );
    assert_eq!(
        node_missing, rust_missing,
        "node: {node_missing:#?}, rust: {rust_missing:#?}"
    );
    assert_eq!(rust_full.status, 200, "got: {rust_full:#?}");
    assert_eq!(rust_missing.status, 404, "got: {rust_missing:#?}");
}

fn upload_document_emitted() -> String {
    [
        "import { z } from \"zod\";".to_owned(),
        UploadDocumentResponse::ts_definition(),
        UploadDocumentResponse::zod_schema(),
        UploadDocumentError::ts_definition(),
        UploadDocumentError::zod_schema(),
        UploadDocumentClientServiceSchema::ts_definition(),
        UploadDocumentClientServiceSchema::ts_service(),
        UploadDocumentClientServiceSchema::ts_http_service(),
    ]
    .join("\n\n")
}

fn upload_document_rust_answered(
    parts: Vec<(String, upload_document_http_rest_transport::IncomingPart)>,
) -> Answered {
    let request = upload_document_http_rest_transport::IncomingRequest::new(
        "POST".to_owned(),
        "/folders/acme/documents".to_owned(),
        String::new(),
        Vec::new(),
        Vec::new(),
    );
    let response = poll_once(upload_document_http_rest_transport::dispatch(
        &UploadDocumentBackEnd,
        &(),
        &request,
        parts,
        &upload_document_http_rest_transport::DefaultFaultHandler,
    ))
    .unwrap();
    Answered {
        body: response.body().to_vec(),
        headers: sorted(response.headers().to_vec()),
        status: response.status(),
    }
}

fn stand_in_file_part() -> (String, upload_document_http_rest_transport::IncomingPart) {
    (
        "file".to_owned(),
        upload_document_http_rest_transport::IncomingPart::File(Box::new(io::Cursor::new(
            b"stand-in".to_vec(),
        ))),
    )
}

fn multipart_body_kind_agrees_with_rust() {
    let module = format!("{}\n\n{MULTIPART_DRIVER}", upload_document_emitted());
    let Some(results) = run_or_stand_down("http-service-multipart", "multipart.mts", &module)
    else {
        return;
    };

    let node_ok = node_answered(&results["ok"]);
    let rust_ok = upload_document_rust_answered(vec![
        (
            "title".to_owned(),
            upload_document_http_rest_transport::IncomingPart::Text("quarterly-report".to_owned()),
        ),
        (
            "description".to_owned(),
            upload_document_http_rest_transport::IncomingPart::Text("Q3 numbers".to_owned()),
        ),
        stand_in_file_part(),
    ]);
    assert_eq!(node_ok, rust_ok, "node: {node_ok:#?}, rust: {rust_ok:#?}");
    assert_eq!(rust_ok.status, 200, "got: {rust_ok:#?}");

    let node_too_large = node_answered(&results["tooLarge"]);
    let rust_too_large = upload_document_rust_answered(vec![
        (
            "title".to_owned(),
            upload_document_http_rest_transport::IncomingPart::Text("toolarge".to_owned()),
        ),
        stand_in_file_part(),
    ]);
    assert_eq!(
        node_too_large, rust_too_large,
        "node: {node_too_large:#?}, rust: {rust_too_large:#?}"
    );
    assert_eq!(rust_too_large.status, 413, "got: {rust_too_large:#?}");

    // The missing-part fault's `detail` text is no longer byte-equal with Rust's, so `status`,
    // `kind` and `field` are compared here instead of the raw body.
    let node_missing_file = node_answered(&results["missingFile"]);
    let rust_missing_file = upload_document_rust_answered(vec![(
        "title".to_owned(),
        upload_document_http_rest_transport::IncomingPart::Text("no-file".to_owned()),
    )]);
    assert_eq!(
        node_missing_file.status, rust_missing_file.status,
        "got: {node_missing_file:#?}"
    );
    assert_eq!(rust_missing_file.status, 400, "got: {rust_missing_file:#?}");
    let node_missing_file_body: serde_json::Value =
        serde_json::from_slice(&node_missing_file.body).unwrap();
    let rust_missing_file_body: serde_json::Value =
        serde_json::from_slice(&rust_missing_file.body).unwrap();
    assert_eq!(
        node_missing_file_body["field"], rust_missing_file_body["field"],
        "got: {node_missing_file_body:#?}"
    );
    assert_eq!(
        node_missing_file_body["kind"], rust_missing_file_body["kind"],
        "got: {node_missing_file_body:#?}"
    );
    assert_eq!(
        node_missing_file_body["field"], "file",
        "got: {node_missing_file_body:#?}"
    );
}

// -------------------------------------------------------------------------------------------
// Group 5: a required `header_in` binding, echoed back by the implementation — present or
// absent, compared byte-for-byte with the Rust `{service}_http_rest_dispatcher!()` twin.
// -------------------------------------------------------------------------------------------

fn echo_emitted() -> String {
    [
        "import { z } from \"zod\";".to_owned(),
        EchoRangeResponse::ts_definition(),
        EchoRangeResponse::zod_schema(),
        EchoRangeError::ts_definition(),
        EchoRangeError::zod_schema(),
        EchoClientServiceSchema::ts_definition(),
        EchoClientServiceSchema::ts_service(),
        EchoClientServiceSchema::ts_http_service(),
    ]
    .join("\n\n")
}

fn echo_rust_answered(headers: Vec<(String, String)>) -> Answered {
    let request = echo_http_rest_transport::IncomingRequest::new(
        "GET".to_owned(),
        "/echo/d1".to_owned(),
        String::new(),
        headers,
        Vec::new(),
    );
    let response = poll_once(echo_http_rest_transport::dispatch(
        &EchoBackEnd,
        &(),
        &request,
        &echo_http_rest_transport::DefaultFaultHandler,
    ))
    .unwrap();
    Answered {
        body: response.body().to_vec(),
        headers: sorted(response.headers().to_vec()),
        status: response.status(),
    }
}

#[test]
fn a_bound_header_reaches_the_implementation_and_agrees_with_the_rust_dispatcher() {
    let module = format!("{}\n\n{ECHO_DRIVER}", echo_emitted());
    let Some(results) = run_or_stand_down("http-service-echo", "echo.mts", &module) else {
        return;
    };

    let node_present = node_answered(&results["present"]);
    let rust_present = echo_rust_answered(vec![("range".to_owned(), "bytes=0-9".to_owned())]);
    assert_eq!(
        node_present, rust_present,
        "node: {node_present:#?}, rust: {rust_present:#?}"
    );
    assert_eq!(rust_present.status, 200, "got: {rust_present:#?}");
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&rust_present.body).unwrap(),
        serde_json::json!({"received": "bytes=0-9"}),
        "got: {rust_present:#?}"
    );

    // `detail` is not compared byte-for-byte: it differs from the Rust dispatcher's own text, so
    // `status`, `kind` and `field` are compared here instead of the raw body.
    let node_absent = node_answered(&results["absent"]);
    let rust_absent = echo_rust_answered(Vec::new());
    assert_eq!(
        node_absent.status, rust_absent.status,
        "got: {node_absent:#?}"
    );
    assert_eq!(rust_absent.status, 400, "got: {rust_absent:#?}");
    let node_absent_body: serde_json::Value = serde_json::from_slice(&node_absent.body).unwrap();
    let rust_absent_body: serde_json::Value = serde_json::from_slice(&rust_absent.body).unwrap();
    assert_eq!(
        node_absent_body["field"], rust_absent_body["field"],
        "got: {node_absent_body:#?}"
    );
    assert_eq!(
        node_absent_body["kind"], rust_absent_body["kind"],
        "got: {node_absent_body:#?}"
    );
    assert_eq!(
        node_absent_body["field"], "range",
        "got: {node_absent_body:#?}"
    );
}

// -------------------------------------------------------------------------------------------
// Group 6: a bodyless operation with no field beside the context, compared whole with the Rust
// `{service}_http_rest_dispatcher!()` twin for the same request.
// -------------------------------------------------------------------------------------------

fn pulse_emitted() -> String {
    [
        "import { z } from \"zod\";".to_owned(),
        PulseResponse::ts_definition(),
        PulseResponse::zod_schema(),
        PulseError::ts_definition(),
        PulseError::zod_schema(),
        PulseClientServiceSchema::ts_definition(),
        PulseClientServiceSchema::ts_service(),
        PulseClientServiceSchema::ts_http_service(),
    ]
    .join("\n\n")
}

fn pulse_rust_answered() -> Answered {
    let request = pulse_http_rest_transport::IncomingRequest::new(
        "GET".to_owned(),
        "/pulse".to_owned(),
        String::new(),
        Vec::new(),
        Vec::new(),
    );
    let response = poll_once(pulse_http_rest_transport::dispatch(
        &PulseBackEnd,
        &(),
        &request,
        &pulse_http_rest_transport::DefaultFaultHandler,
    ))
    .unwrap();
    Answered {
        body: response.body().to_vec(),
        headers: sorted(response.headers().to_vec()),
        status: response.status(),
    }
}

#[test]
fn a_bodyless_operation_with_no_field_assembles_the_same_message_as_the_rust_dispatcher() {
    let module = format!("{}\n\n{PULSE_DRIVER}", pulse_emitted());
    let Some(result) = run_or_stand_down("http-service-pulse", "pulse.mts", &module) else {
        return;
    };

    let node = node_answered(&result);
    let rust = pulse_rust_answered();
    assert_eq!(node, rust, "node: {node:#?}, rust: {rust:#?}");
    assert_eq!(rust.status, 200, "got: {rust:#?}");
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&rust.body).unwrap(),
        serde_json::json!({"alive": true}),
        "got: {rust:#?}"
    );
}

// -------------------------------------------------------------------------------------------
// The Rust twins' own accessors, exercised once each — the same claims
// `tests/service_schema_dispatch_tests/` makes for every other dispatcher macro placement.
// -------------------------------------------------------------------------------------------

/// One `IncomingRequest` reads back everything it was built with. Takes the accessors' own
/// answers rather than the request itself: the three placement modules' `IncomingRequest` share
/// this shape but not a trait.
fn assert_incoming_request_reads_back(
    body: &[u8],
    query: &str,
    headers: &[(String, String)],
    header: Option<&str>,
) {
    assert_eq!(body, b"ignored");
    assert_eq!(query, "unused=1");
    assert_eq!(headers, &[("x-trace".to_owned(), "abc".to_owned())]);
    assert_eq!(header, Some("abc"));
}

#[test]
fn the_thumbnail_route_table_and_incoming_request_read_back_what_they_were_built_with() {
    let routes = thumbnail_http_rest_transport::ROUTES;
    assert_eq!(
        routes.len(),
        1,
        "got: {:?}",
        routes
            .iter()
            .map(thumbnail_http_rest_transport::Route::operation)
            .collect::<Vec<_>>()
    );
    assert_eq!(routes[0].method(), "GET");
    assert_eq!(routes[0].path(), "/thumbnails/{document_id}");
    assert_eq!(routes[0].operation(), "get-thumbnail");
    assert_eq!(routes[0].ok_status(), 200);
    assert_eq!(routes[0].error_statuses(), &[404]);

    let request = thumbnail_http_rest_transport::IncomingRequest::new(
        "GET".to_owned(),
        "/thumbnails/present".to_owned(),
        "unused=1".to_owned(),
        vec![("x-trace".to_owned(), "abc".to_owned())],
        b"ignored".to_vec(),
    );
    assert_incoming_request_reads_back(
        request.body(),
        request.query(),
        request.headers(),
        request.header("x-trace"),
    );

    let unmatched = thumbnail_http_rest_transport::IncomingRequest::new(
        "GET".to_owned(),
        "/nowhere".to_owned(),
        String::new(),
        Vec::new(),
        Vec::new(),
    );
    let response = poll_once(thumbnail_http_rest_transport::dispatch(
        &ThumbnailBackEnd,
        &(),
        &unmatched,
        &RecordingFaultHandler,
    ))
    .unwrap();
    assert_eq!(response.status(), 499);
    assert_eq!(
        response.headers(),
        &[("x-fault-kind".to_owned(), "unknown operation".to_owned())]
    );
    assert_eq!(response.body(), b"handled");
}

#[test]
fn the_content_route_table_and_incoming_request_read_back_what_they_were_built_with() {
    let routes = content_http_rest_transport::ROUTES;
    assert_eq!(
        routes.len(),
        1,
        "got: {:?}",
        routes
            .iter()
            .map(content_http_rest_transport::Route::operation)
            .collect::<Vec<_>>()
    );
    assert_eq!(routes[0].method(), "GET");
    assert_eq!(routes[0].path(), "/files/{file_id}");
    assert_eq!(routes[0].operation(), "get-file");
    assert_eq!(routes[0].ok_status(), 200);
    assert_eq!(routes[0].error_statuses(), &[404]);

    let request = content_http_rest_transport::IncomingRequest::new(
        "GET".to_owned(),
        "/files/present".to_owned(),
        "unused=1".to_owned(),
        vec![("x-trace".to_owned(), "abc".to_owned())],
        b"ignored".to_vec(),
    );
    assert_incoming_request_reads_back(
        request.body(),
        request.query(),
        request.headers(),
        request.header("x-trace"),
    );

    let unmatched = content_http_rest_transport::IncomingRequest::new(
        "GET".to_owned(),
        "/nowhere".to_owned(),
        String::new(),
        Vec::new(),
        Vec::new(),
    );
    let response = poll_once(content_http_rest_transport::dispatch(
        &ContentBackEnd,
        &(),
        &unmatched,
        &RecordingFaultHandler,
    ))
    .unwrap();
    assert_eq!(response.status(), 499);
    assert_eq!(drain_outgoing(response.into_body()), b"handled");
}

#[test]
fn the_upload_document_route_table_and_incoming_request_read_back_what_they_were_built_with() {
    let routes = upload_document_http_rest_transport::ROUTES;
    assert_eq!(
        routes.len(),
        1,
        "got: {:?}",
        routes
            .iter()
            .map(upload_document_http_rest_transport::Route::operation)
            .collect::<Vec<_>>()
    );
    assert_eq!(routes[0].method(), "POST");
    assert_eq!(routes[0].path(), "/folders/{folder_id}/documents");
    assert_eq!(routes[0].operation(), "upload-document");
    assert_eq!(routes[0].ok_status(), 200);
    assert_eq!(routes[0].error_statuses(), &[413]);

    let request = upload_document_http_rest_transport::IncomingRequest::new(
        "POST".to_owned(),
        "/folders/acme/documents".to_owned(),
        "unused=1".to_owned(),
        vec![("x-trace".to_owned(), "abc".to_owned())],
        b"ignored".to_vec(),
    );
    assert_incoming_request_reads_back(
        request.body(),
        request.query(),
        request.headers(),
        request.header("x-trace"),
    );

    let unmatched = upload_document_http_rest_transport::IncomingRequest::new(
        "GET".to_owned(),
        "/nowhere".to_owned(),
        String::new(),
        Vec::new(),
        Vec::new(),
    );
    let response = poll_once(upload_document_http_rest_transport::dispatch(
        &UploadDocumentBackEnd,
        &(),
        &unmatched,
        Vec::new(),
        &RecordingFaultHandler,
    ))
    .unwrap();
    assert_eq!(response.status(), 499);
    assert_eq!(response.body(), b"handled");
}

#[test]
fn the_echo_route_table_and_incoming_request_read_back_what_they_were_built_with() {
    let routes = echo_http_rest_transport::ROUTES;
    assert_eq!(
        routes.len(),
        1,
        "got: {:?}",
        routes
            .iter()
            .map(echo_http_rest_transport::Route::operation)
            .collect::<Vec<_>>()
    );
    assert_eq!(routes[0].method(), "GET");
    assert_eq!(routes[0].path(), "/echo/{document_id}");
    assert_eq!(routes[0].operation(), "echo-range");
    assert_eq!(routes[0].ok_status(), 200);
    assert_eq!(routes[0].error_statuses(), &[404]);

    let request = echo_http_rest_transport::IncomingRequest::new(
        "GET".to_owned(),
        "/echo/d1".to_owned(),
        "unused=1".to_owned(),
        vec![("x-trace".to_owned(), "abc".to_owned())],
        b"ignored".to_vec(),
    );
    assert_incoming_request_reads_back(
        request.body(),
        request.query(),
        request.headers(),
        request.header("x-trace"),
    );

    let unmatched = echo_http_rest_transport::IncomingRequest::new(
        "GET".to_owned(),
        "/nowhere".to_owned(),
        String::new(),
        Vec::new(),
        Vec::new(),
    );
    let response = poll_once(echo_http_rest_transport::dispatch(
        &EchoBackEnd,
        &(),
        &unmatched,
        &RecordingFaultHandler,
    ))
    .unwrap();
    assert_eq!(response.status(), 499);
    assert_eq!(response.body(), b"handled");
}

#[test]
fn the_pulse_route_table_and_incoming_request_read_back_what_they_were_built_with() {
    let routes = pulse_http_rest_transport::ROUTES;
    assert_eq!(
        routes.len(),
        1,
        "got: {:?}",
        routes
            .iter()
            .map(pulse_http_rest_transport::Route::operation)
            .collect::<Vec<_>>()
    );
    assert_eq!(routes[0].method(), "GET");
    assert_eq!(routes[0].path(), "/pulse");
    assert_eq!(routes[0].operation(), "pulse");
    assert_eq!(routes[0].ok_status(), 200);
    assert_eq!(routes[0].error_statuses(), &[422]);

    let request = pulse_http_rest_transport::IncomingRequest::new(
        "GET".to_owned(),
        "/pulse".to_owned(),
        "unused=1".to_owned(),
        vec![("x-trace".to_owned(), "abc".to_owned())],
        b"ignored".to_vec(),
    );
    assert_incoming_request_reads_back(
        request.body(),
        request.query(),
        request.headers(),
        request.header("x-trace"),
    );

    let unmatched = pulse_http_rest_transport::IncomingRequest::new(
        "GET".to_owned(),
        "/nowhere".to_owned(),
        String::new(),
        Vec::new(),
        Vec::new(),
    );
    let response = poll_once(pulse_http_rest_transport::dispatch(
        &PulseBackEnd,
        &(),
        &unmatched,
        &RecordingFaultHandler,
    ))
    .unwrap();
    assert_eq!(response.status(), 499);
    assert_eq!(response.body(), b"handled");
}

// -------------------------------------------------------------------------------------------
// Group 7: a fully path-bound macro-generated message - two placeholders, no query field - and
// the declared error it still answers.
// -------------------------------------------------------------------------------------------

fn label_emitted() -> String {
    [
        "import { z } from \"zod\";".to_owned(),
        LabelStatus::ts_definition(),
        LabelStatus::zod_schema(),
        LabelError::ts_definition(),
        LabelError::zod_schema(),
        LabelClientServiceSchema::ts_definition(),
        LabelClientServiceSchema::ts_service(),
        LabelClientServiceSchema::ts_http_service(),
    ]
    .join("\n\n")
}

#[test]
fn a_fully_path_bound_generated_message_dispatches_with_no_query_reader() {
    let module = format!("{}\n\n{LABEL_DRIVER}", label_emitted());
    let Some(results) = run_or_stand_down("http-service-label", "label.mts", &module) else {
        return;
    };

    assert_eq!(results["ok"]["status"], 200_i64, "got: {results:#?}");
    assert_eq!(
        results["ok"]["body"],
        serde_json::json!({"label": "acme/priority"}),
        "both fields decoded off the path placeholders. got: {results:#?}"
    );

    assert_eq!(results["missing"]["status"], 404_i64, "got: {results:#?}");
    assert_eq!(
        results["missing"]["body"],
        serde_json::json!({"errorCode": "not-found"}),
        "got: {results:#?}"
    );
}

fn echo_client_emitted() -> String {
    [
        "import { z } from \"zod\";".to_owned(),
        EchoRangeResponse::ts_definition(),
        EchoRangeResponse::zod_schema(),
        EchoRangeError::ts_definition(),
        EchoRangeError::zod_schema(),
        EchoClientServiceSchema::ts_definition(),
        EchoClientServiceSchema::ts_http_client(),
    ]
    .join("\n\n")
}

#[test]
fn a_header_in_value_with_a_line_feed_is_refused_before_the_transport_is_ever_reached() {
    let module = format!("{}\n\n{ECHO_CLIENT_DRIVER}", echo_client_emitted());
    let Some(results) = run_or_stand_down("http-client-echo", "echo-client.mts", &module) else {
        return;
    };
    assert_eq!(
        results["sendCalled"], false,
        "an illegal `header_in` value must refuse before the transport is ever reached. \
         got: {results:#?}"
    );
    assert_eq!(results["refused"]["ok"], false, "got: {results:#?}");
    assert_eq!(
        results["refused"]["error"]["fault"]["kind"], "failed-validation",
        "got: {results:#?}"
    );
}

/// A legal `header_in` value carrying a space and a quoted `ETag` reaches the transport unchanged
/// rather than being refused as if it were illegal.
#[test]
fn a_legal_header_in_value_with_a_space_and_a_quoted_etag_reaches_the_transport_unchanged() {
    let module = format!("{}\n\n{ECHO_CLIENT_LEGAL_DRIVER}", echo_client_emitted());
    let Some(results) =
        run_or_stand_down("http-client-echo-legal", "echo-client-legal.mts", &module)
    else {
        return;
    };
    assert_eq!(results["answered"]["ok"], true, "got: {results:#?}");
    let headers = results["sentHeaders"].as_array().unwrap();
    assert!(
        headers.iter().any(|entry| entry[0] == "range"),
        "no `range` header was sent. got: {results:#?}"
    );
    let range_header = headers.iter().find(|entry| entry[0] == "range").unwrap();
    assert_eq!(
        range_header[1], "a \"etag\" b",
        "a legal value must reach the transport unchanged. got: {results:#?}"
    );
}
