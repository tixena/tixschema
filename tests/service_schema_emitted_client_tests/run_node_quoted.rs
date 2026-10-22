//! Services whose message names, paths, part name and header names hold a `$`, a quote, a double
//! quote and a backslash, and a message name that holds a line break and the end of a comment,
//! called through the emitted TypeScript clients and served by the emitted TypeScript dispatchers
//! under node. Each request the `http_rest` client writes is read by the Rust dispatcher too.

use super::runtime::{node_modules, ran_with_modules, stand_down_modules};
use super::{impost_filing_http_rest_transport, impost_http_rest_transport};
use core::future::{Future, ready};
use core::pin::pin;
use core::task::{Context as PollContext, Poll, Waker};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::io;
use std::sync::Mutex;
use tixschema::{model_schema, service_schema};

/// The runtime the driver runs under, named for a machine that has one somewhere other than `PATH`.
const RUNTIME_VAR: &str = "TIXSCHEMA_NODE";

/// The package the emitted schemas import.
const REQUIRED_PACKAGES: &[&str] = &["zod"];

/// The message name `levy` declares.
const LEVY_WIRE: &str = "le$vy'it\"q\\x";

/// The message name `note` declares, which names its path too.
const NOTE_WIRE: &str = "no*/te\n\\";

/// The message name `file` declares.
const FILE_WIRE: &str = "fi\"l'ed\\";

/// The name of the part `file` carries its attachment in.
const FILE_PART: &str = "fi$le'x\"q\\";

/// The calls the driver makes, through the `http_rest` clients bound to the TypeScript
/// dispatchers and through the `ws_rpc` client bound to the attachment, each request the
/// `http_rest` clients write recorded with what the dispatcher answered it. `NAMES` and
/// `INBOUND_PATH` are written above this by the Rust side.
const DRIVER: &str = r#"
function fakeSocket(onSend) {
  const listeners = { message: new Set(), close: new Set() };
  return {
    send(text) { onSend(text); },
    close() { for (const listener of listeners.close) listener(); },
    addEventListener(type, listener) { listeners[type].add(listener); },
    removeEventListener(type, listener) { listeners[type].delete(listener); },
    deliver(text) { for (const listener of listeners.message) listener({ data: text }); },
  };
}

const noted = [];
let wake = () => {};
const impl = {
  async levy(ctx, req, trace) {
    if (req.label === "refuse") {
      return { ok: false, error: [{ errorCode: "refused" }, trace === undefined ? null : `at ${trace}`] };
    }
    return { ok: true, value: [{ label: `${req.label}|${req.note}` }, `age-${trace ?? "none"}`] };
  },
  async note(ctx, label) {
    noted.push(label);
    wake();
  },
};
const filing = {
  async file(ctx, req, attachment) {
    return { ok: true, value: { label: `${req.label}|${req.title}|${attachment}` } };
  },
};

function answeredOf(response) {
  return { status: response.status, headers: response.headers, bodyBytes: Array.from(response.body) };
}

const sent = [];
function carried(service, dispatch) {
  return {
    async send(request) {
      const response = await dispatch({}, {
        method: request.method,
        path: request.path,
        query: request.query,
        headers: request.headers,
        body: new TextEncoder().encode(request.body),
        ...(request.parts === undefined ? {} : { parts: request.parts }),
      });
      sent.push({ service, request, answered: answeredOf(response) });
      return { status: response.status, headers: response.headers, body: new TextDecoder().decode(response.body) };
    },
  };
}

async function main() {
  const dispatchImpost = createImpostClientServiceHttpDispatcher(impl);
  const dispatchFiling = createImpostFilingClientServiceHttpDispatcher(filing);
  const http = createImpostClientServiceHttpClient(carried("impost", dispatchImpost));
  const filingClient = createImpostFilingClientServiceHttpClient(carried("filing", dispatchFiling));
  const overHttp = {
    levied: await http.levy({ label: "a", note: "n" }, 7),
    bare: await http.levy({ label: "b", note: "n" }, undefined),
    refused: await http.levy({ label: "refuse", note: "n" }, 3),
    noted: (await http.note("c")) ?? null,
    outbound: await http.levy({ label: 5, note: "n" }, undefined),
    filed: await filingClient.file({ label: "f", title: "t" }, "stand-in"),
  };
  const inbound = answeredOf(
    await dispatchImpost({}, { method: "POST", path: INBOUND_PATH, query: "", headers: [], body: new TextEncoder().encode("{}") }),
  );

  let serverSocket;
  const clientSocket = fakeSocket((text) => queueMicrotask(() => serverSocket.deliver(text)));
  serverSocket = fakeSocket((text) => queueMicrotask(() => clientSocket.deliver(text)));
  const faults = [];
  attachImpostClientServiceWsDispatcher(serverSocket, {}, impl, (fault) => faults.push(fault));
  const ws = createImpostClientServiceClient(createImpostClientServiceWsTransport(clientSocket, { heartbeat: false }));
  const notedOverWs = new Promise((resolve) => { wake = resolve; });
  const overWs = {
    levied: await ws.levy({ label: "a", note: "n" }, 7),
    refused: await ws.levy({ label: "refuse", note: "n" }, 3),
    noted: (await ws.note("d")) ?? null,
  };
  await notedOverWs;

  const routes = [...impostClientServiceHttpRoutes, ...impostFilingClientServiceHttpRoutes].map(
    (route) => [route.method, route.path, route.operation],
  );
  console.log(JSON.stringify({ sent, overHttp, inbound, overWs, noted, faults, routes, names: NAMES }));
  process.exit(0);
}
main().catch((error) => { console.error(error); process.exit(1); });
"#;

/// A Rust twin's own constructor of a request: method, path, query, headers and body.
type BuildsARequest<Request> =
    fn(String, String, String, Vec<(String, String)>, Vec<u8>) -> Request;

/// What every operation answers with.
#[model_schema()]
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ImpostReceipt {
    pub label: String,
}

/// The one error each operation declares.
#[model_schema()]
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case", tag = "errorCode")]
pub enum ImpostError {
    Refused,
}

/// A path, a message name and a header in and out that hold a quote, over both transports, and a
/// one-way operation on the path its message name gives it.
#[service_schema(transports = ["http_rest", "ws_rpc"])]
pub trait ImpostClientService<Ctx> {
    #[service_schema_op(
        message = "le$vy'it\"q\\x",
        http(
            method = "POST",
            path = "/le$vy'it\"q\\x/{label}",
            header_in("x-it's" = trace),
            header_out("x-ag$e"),
            error_status(Refused = 409),
            error_header_out("x-rea$on"),
        )
    )]
    async fn levy(
        &self,
        ctx: &Ctx,
        label: String,
        note: String,
        trace: Option<u32>,
    ) -> Result<(ImpostReceipt, String), (ImpostError, Option<String>)>;

    #[service_schema_op(message = "no*/te\n\\", one_way)]
    async fn note(&self, ctx: &Ctx, label: String);
}

/// A multipart operation whose path, message name and part name hold a quote.
#[service_schema(transports = ["http_rest"])]
pub trait ImpostFilingClientService<Ctx> {
    #[service_schema_op(
        message = "fi\"l'ed\\",
        http(
            method = "POST",
            path = "/fi\"le/{label}",
            body = "multipart",
            part("fi$le'x\"q\\" = attachment),
        )
    )]
    async fn file(
        &self,
        ctx: &Ctx,
        label: String,
        title: String,
        attachment: Box<dyn impost_filing_client_service_schema::BodySource + Send>,
    ) -> Result<ImpostReceipt, ImpostError>;
}

/// Answers every fault `499`, under a header naming its kind.
struct RefusedAs499;

impl impost_http_rest_transport::FaultHandler for RefusedAs499 {
    fn on_fault(
        &self,
        fault: &impost_client_service_schema::ServiceFault,
    ) -> impost_http_rest_transport::OutgoingResponse {
        impost_http_rest_transport::OutgoingResponse::new(
            499,
            vec![("x-fault-kind".to_owned(), format!("{}", fault.kind()))],
            b"refused".to_vec(),
        )
    }
}

impl impost_filing_http_rest_transport::FaultHandler for RefusedAs499 {
    fn on_fault(
        &self,
        fault: &impost_filing_client_service_schema::ServiceFault,
    ) -> impost_filing_http_rest_transport::OutgoingResponse {
        impost_filing_http_rest_transport::OutgoingResponse::new(
            499,
            vec![("x-fault-kind".to_owned(), format!("{}", fault.kind()))],
            b"refused".to_vec(),
        )
    }
}

/// Answers as the driver's implementation does, and records every label `note` is handed.
#[derive(Default)]
struct ImpostBackEnd {
    noted: Mutex<Vec<String>>,
}

impl ImpostClientService<()> for ImpostBackEnd {
    async fn levy(
        &self,
        _ctx: &(),
        label: String,
        note: String,
        trace: Option<u32>,
    ) -> Result<(ImpostReceipt, String), (ImpostError, Option<String>)> {
        ready(()).await;
        if label == "refuse" {
            return Err((ImpostError::Refused, trace.map(|at| format!("at {at}"))));
        }
        let age = trace.map_or_else(|| "none".to_owned(), |at| at.to_string());
        Ok((
            ImpostReceipt {
                label: format!("{label}|{note}"),
            },
            format!("age-{age}"),
        ))
    }

    async fn note(&self, _ctx: &(), label: String) {
        ready(()).await;
        self.noted.lock().unwrap().push(label);
    }
}

impl ImpostFilingClientService<()> for ImpostBackEnd {
    async fn file(
        &self,
        _ctx: &(),
        label: String,
        title: String,
        mut attachment: Box<dyn impost_filing_client_service_schema::BodySource + Send>,
    ) -> Result<ImpostReceipt, ImpostError> {
        ready(()).await;
        let mut drained = Vec::new();
        let mut buf = [0_u8; 64];
        loop {
            let read = attachment.pull(&mut buf).unwrap();
            if read == 0 {
                break;
            }
            drained.extend_from_slice(&buf[..read]);
        }
        let content = String::from_utf8(drained).unwrap();
        Ok(ImpostReceipt {
            label: format!("{label}|{title}|{content}"),
        })
    }
}

/// One HTTP answer, as either dispatcher gave it.
#[derive(Debug, Eq, PartialEq)]
struct Answered {
    body: Vec<u8>,
    headers: Vec<(String, String)>,
    status: u16,
}

/// The dispatcher never suspends, so one poll answers it.
fn poll_once<Answering>(answering: Answering) -> Option<Answering::Output>
where
    Answering: Future,
{
    let mut pinned = pin!(answering);
    match pinned
        .as_mut()
        .poll(&mut PollContext::from_waker(Waker::noop()))
    {
        Poll::Ready(answer) => Some(answer),
        Poll::Pending => None,
    }
}

/// The text `value` holds.
fn text(value: &Value) -> String {
    value.as_str().unwrap().to_owned()
}

/// The `[name, value]` pairs `value` holds.
fn pairs(value: &Value) -> Vec<(String, String)> {
    let mut held: Vec<(String, String)> = value
        .as_array()
        .unwrap()
        .iter()
        .map(|pair| (text(&pair[0]), text(&pair[1])))
        .collect();
    held.sort();
    held
}

/// What the TypeScript dispatcher answered, as the driver printed it.
fn node_answered(value: &Value) -> Answered {
    Answered {
        body: value["bodyBytes"]
            .as_array()
            .unwrap()
            .iter()
            .map(|byte| u8::try_from(byte.as_u64().unwrap()).unwrap())
            .collect(),
        headers: pairs(&value["headers"]),
        status: u16::try_from(value["status"].as_u64().unwrap()).unwrap(),
    }
}

/// What the Rust dispatcher answers the request the TypeScript client wrote to `service`.
fn rust_answered(backend: &ImpostBackEnd, service: &str, request: &Value) -> Answered {
    let method = text(&request["method"]);
    let path = text(&request["path"]);
    let query = text(&request["query"]);
    let headers = pairs(&request["headers"]);
    let body = text(&request["body"]).into_bytes();
    if service == "filing" {
        let parts = request["parts"]
            .as_array()
            .unwrap()
            .iter()
            .map(|part| {
                let (name, content) = (text(&part[0]), text(&part[1]));
                let carried = if name == FILE_PART {
                    impost_filing_http_rest_transport::IncomingPart::File(Box::new(
                        io::Cursor::new(content.into_bytes()),
                    ))
                } else {
                    impost_filing_http_rest_transport::IncomingPart::Text(content)
                };
                (name, carried)
            })
            .collect();
        let incoming = impost_filing_http_rest_transport::IncomingRequest::new(
            method, path, query, headers, body,
        );
        let response = poll_once(impost_filing_http_rest_transport::dispatch(
            backend,
            &(),
            &incoming,
            parts,
            &impost_filing_http_rest_transport::DefaultFaultHandler,
        ))
        .unwrap();
        let mut answered_headers = response.headers().to_vec();
        answered_headers.sort();
        return Answered {
            body: response.body().to_vec(),
            headers: answered_headers,
            status: response.status(),
        };
    }
    let incoming =
        impost_http_rest_transport::IncomingRequest::new(method, path, query, headers, body);
    let response = poll_once(impost_http_rest_transport::dispatch(
        backend,
        &(),
        &incoming,
        &impost_http_rest_transport::DefaultFaultHandler,
    ))
    .unwrap();
    let mut answered_headers = response.headers().to_vec();
    answered_headers.sort();
    Answered {
        body: response.body().to_vec(),
        headers: answered_headers,
        status: response.status(),
    }
}

/// A request no route matches, built by a Rust twin's own constructor.
fn unmatched<Request>(build: BuildsARequest<Request>) -> Request {
    build(
        "GET".to_owned(),
        "/nowhere".to_owned(),
        "unused=1".to_owned(),
        vec![("x-it's".to_owned(), "abc".to_owned())],
        b"ignored".to_vec(),
    )
}

/// The driver run under node, or `None` where node or `zod` is not reachable.
fn driven() -> Option<Value> {
    let Some(modules) = node_modules(REQUIRED_PACKAGES) else {
        stand_down_modules(
            REQUIRED_PACKAGES,
            "the emitted TypeScript services whose names hold a quote",
        );
        return None;
    };
    let names = json!([LEVY_WIRE, NOTE_WIRE, FILE_WIRE, FILE_PART]);
    let module = [
        "import { z } from \"zod\";".to_owned(),
        ImpostReceipt::ts_definition(),
        ImpostReceipt::zod_schema(),
        ImpostError::ts_definition(),
        ImpostError::zod_schema(),
        ImpostClientServiceSchema::ts_definition(),
        ImpostClientServiceSchema::ts_client(),
        ImpostClientServiceSchema::ts_service(),
        ImpostClientServiceSchema::ts_http_client(),
        ImpostClientServiceSchema::ts_http_service(),
        ImpostClientServiceSchema::ts_ws_client(),
        ImpostClientServiceSchema::ts_ws_service(),
        ImpostFilingClientServiceSchema::ts_definition(),
        ImpostFilingClientServiceSchema::ts_service(),
        ImpostFilingClientServiceSchema::ts_http_client(),
        ImpostFilingClientServiceSchema::ts_http_service(),
        format!(
            "const NAMES = {names};\nconst INBOUND_PATH = {};",
            json!(format!("/{LEVY_WIRE}/a"))
        ),
        DRIVER.to_owned(),
    ]
    .join("\n\n");
    let printed = ran_with_modules(
        "quoted",
        RUNTIME_VAR,
        "node",
        "quoted.mts",
        &module,
        &modules,
    )?;
    Some(serde_json::from_str(printed.trim()).unwrap())
}

/// Every request the `http_rest` clients wrote reaches the path and carries the header and the part
/// its operation declares, and the Rust dispatcher answers each one, and a request the TypeScript
/// dispatcher refuses, as the TypeScript dispatcher does.
fn assert_dispatched_alike(ran: &Value) {
    let sent = ran["sent"].as_array().unwrap();
    let paths: Vec<String> = sent
        .iter()
        .map(|carried| text(&carried["request"]["path"]))
        .collect();
    assert_eq!(
        paths,
        [
            "/le$vy'it\"q\\x/a",
            "/le$vy'it\"q\\x/b",
            "/le$vy'it\"q\\x/refuse",
            "/no*/te\n\\",
            "/fi\"le/f",
        ],
        "the clients wrote another path"
    );
    assert_eq!(
        pairs(&sent[0]["request"]["headers"]),
        [("x-it's".to_owned(), "7".to_owned())],
        "the client wrote another header"
    );
    assert_eq!(
        sent[4]["request"]["parts"],
        json!([["title", "t"], [FILE_PART, "stand-in"]]),
        "the client wrote other parts"
    );
    let backend = ImpostBackEnd::default();
    for carried in sent {
        let service = text(&carried["service"]);
        let node = node_answered(&carried["answered"]);
        let rust = rust_answered(&backend, &service, &carried["request"]);
        assert_eq!(
            node, rust,
            "the dispatchers answer {} apart",
            carried["request"]
        );
    }
    assert_eq!(
        *backend.noted.lock().unwrap(),
        ["c"],
        "the Rust dispatcher did not read the one-way call"
    );

    let inbound = node_answered(&ran["inbound"]);
    let incoming = json!({
        "method": "POST",
        "path": format!("/{LEVY_WIRE}/a"),
        "query": "",
        "headers": [],
        "body": "{}",
    });
    let rust_inbound = rust_answered(&backend, "impost", &incoming);
    assert_eq!(inbound.status, rust_inbound.status, "{inbound:?}");
    let node_fault: Value = serde_json::from_slice(&inbound.body).unwrap();
    let rust_fault: Value = serde_json::from_slice(&rust_inbound.body).unwrap();
    assert_eq!(node_fault["operation"], LEVY_WIRE, "{node_fault}");
    assert_eq!(rust_fault["operation"], LEVY_WIRE, "{rust_fault}");
    assert_eq!(node_fault["kind"], rust_fault["kind"], "{node_fault}");
}

/// Over both transports a call answers what the implementation answered, a one-way call reaches
/// it, and the fault of a message the client refuses names the message name as declared.
fn assert_answered(ran: &Value) {
    assert_eq!(
        ran["noted"],
        json!(["c", "d"]),
        "the TypeScript dispatchers did not read both one-way calls"
    );
    let levied = json!({ "ok": true, "value": [{ "label": "a|n" }, "age-7"] });
    let refused = json!({ "ok": false, "error": [{ "errorCode": "refused" }, "at 3"] });
    let over_http = &ran["overHttp"];
    assert_eq!(over_http["levied"], levied, "{over_http}");
    assert_eq!(
        over_http["bare"],
        json!({ "ok": true, "value": [{ "label": "b|n" }, "age-none"] }),
        "{over_http}"
    );
    assert_eq!(over_http["refused"], refused, "{over_http}");
    assert_eq!(over_http["noted"], Value::Null, "{over_http}");
    assert_eq!(
        over_http["filed"],
        json!({ "ok": true, "value": { "label": "f|t|stand-in" } }),
        "{over_http}"
    );
    let outbound = &over_http["outbound"]["error"]["fault"];
    assert_eq!(outbound["operation"], LEVY_WIRE, "{outbound}");
    assert_eq!(outbound["kind"], "failed-validation", "{outbound}");

    let over_ws = &ran["overWs"];
    assert_eq!(over_ws["levied"], levied, "{over_ws}");
    assert_eq!(over_ws["refused"], refused, "{over_ws}");
    assert_eq!(over_ws["noted"], Value::Null, "{over_ws}");
    assert_eq!(ran["faults"], json!([]), "the attachment answered a fault");
}

/// Each name reaches node as declared, the requests the clients write are answered alike by both
/// dispatchers, every call answers what the implementation answered over both transports, and the
/// route tables name each path and message name as the Rust ones do.
#[test]
fn a_service_whose_names_hold_a_quote_is_carried_as_declared() {
    let Some(ran) = driven() else {
        return;
    };
    assert_eq!(
        ran["names"],
        json!([LEVY_WIRE, NOTE_WIRE, FILE_WIRE, FILE_PART]),
        "node does not read the names the Rust side wrote"
    );
    assert_dispatched_alike(&ran);
    assert_answered(&ran);
    let rust_routes: Vec<Value> = impost_http_rest_transport::ROUTES
        .iter()
        .map(|route| json!([route.method(), route.path(), route.operation()]))
        .chain(
            impost_filing_http_rest_transport::ROUTES
                .iter()
                .map(|route| json!([route.method(), route.path(), route.operation()])),
        )
        .collect();
    assert_eq!(
        ran["routes"],
        Value::Array(rust_routes),
        "the route tables name another path or message name"
    );
}

/// The README's sentence on a name that holds a quote shows the declaration the run makes.
#[test]
fn the_readme_shows_the_declaration_the_run_makes() {
    let readme = include_str!("../../README.md");
    for shown in [
        r#"message = "le$vy'it\"q\\x""#,
        r#"path = "/le$vy'it\"q\\x/{label}""#,
        r#"part("fi$le'x\"q\\" = attachment)"#,
    ] {
        assert!(readme.contains(shown), "not in the README: {shown}");
    }
}

/// Each Rust twin's route table names the statuses each operation declares, a request reads back
/// what it was built with, and a request no route matches reaches the fault handler it is given.
#[test]
fn the_rust_twins_read_back_what_they_were_built_with() {
    let statuses: Vec<(u16, Vec<u16>)> = impost_http_rest_transport::ROUTES
        .iter()
        .map(|route| (route.ok_status(), route.error_statuses().to_vec()))
        .chain(
            impost_filing_http_rest_transport::ROUTES
                .iter()
                .map(|route| (route.ok_status(), route.error_statuses().to_vec())),
        )
        .collect();
    assert_eq!(
        statuses,
        [(200, vec![409]), (204, Vec::new()), (200, vec![422])],
        "a route table names other statuses"
    );

    let impost = unmatched(impost_http_rest_transport::IncomingRequest::new);
    let filing = unmatched(impost_filing_http_rest_transport::IncomingRequest::new);
    for (read_back, kept) in [
        (
            (
                impost.body(),
                impost.query(),
                impost.headers(),
                impost.header("X-It's"),
            ),
            "the request to `ImpostClientService`",
        ),
        (
            (
                filing.body(),
                filing.query(),
                filing.headers(),
                filing.header("X-It's"),
            ),
            "the request to `ImpostFilingClientService`",
        ),
    ] {
        assert_eq!(
            read_back,
            (
                &b"ignored"[..],
                "unused=1",
                &[("x-it's".to_owned(), "abc".to_owned())][..],
                Some("abc"),
            ),
            "{kept} reads back another value"
        );
    }

    let backend = ImpostBackEnd::default();
    let refused = poll_once(impost_http_rest_transport::dispatch(
        &backend,
        &(),
        &impost,
        &RefusedAs499,
    ))
    .unwrap();
    assert_eq!(
        (refused.status(), refused.body()),
        (499, &b"refused"[..]),
        "an unmatched request to `ImpostClientService` is answered another way"
    );
    let refused_filing = poll_once(impost_filing_http_rest_transport::dispatch(
        &backend,
        &(),
        &filing,
        Vec::new(),
        &RefusedAs499,
    ))
    .unwrap();
    assert_eq!(
        (refused_filing.status(), refused_filing.body()),
        (499, &b"refused"[..]),
        "an unmatched request to `ImpostFilingClientService` is answered another way"
    );
}
