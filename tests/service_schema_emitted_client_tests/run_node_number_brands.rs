//! A brand over a number, a brand over a `String` of digits, an alias and a struct of one slot,
//! declared above their service, called through the emitted TypeScript `http_rest` client and
//! dispatcher under node: the client writes each number brand as the text of the number in a path,
//! a query, a header and a text part, the dispatcher reads it back as the number, and a brand over
//! text stays text.

use super::runtime::{node_modules, ran_with_modules, stand_down_modules};
use super::{wharf_filing_http_rest_transport, wharf_http_rest_transport};
use core::future::{Future, ready};
use core::pin::pin;
use core::task::{Context as PollContext, Poll, Waker};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::io;

#[cfg(feature = "dart")]
use super::run_dart_validate::dart;
#[cfg(feature = "dart")]
use super::runtime::ran;
#[cfg(feature = "kotlin")]
use super::runtime::ran_kotlin;
use tixschema::{model_schema, service_schema};

const RUNTIME_VAR: &str = "TIXSCHEMA_NODE";

const REQUIRED_PACKAGES: &[&str] = &["zod"];

const DRIVER: &str = r#"
const kinds = [];
const impl = {
  async find(ctx, req) {
    kinds.push(typeof req.berth, typeof req.ticket, typeof req.since, typeof req.berths[0]);
    return { ok: true, value: [req.berth, req.ticket, req.label, req.since, req.berths.join(",")].join("|") };
  },
  async moor(ctx, req, berth, ticket, label) {
    kinds.push(typeof berth, typeof ticket);
    return { ok: true, value: [[req, berth, ticket, label].join("|"), ticket + 1n] };
  },
};
const filing = {
  async file(ctx, req, attachment) {
    kinds.push(typeof req.berth, typeof req.ticket);
    return { ok: true, value: [req.berth, req.ticket, req.label, attachment].join("|") };
  },
};

const sent = [];
function carried(dispatch) {
  return {
    async send(request) {
      const response = await dispatch({}, {
        method: request.method,
        path: request.path,
        query: request.query,
        headers: request.headers,
        body: request.body,
        ...(request.parts === undefined ? {} : { parts: request.parts }),
      });
      sent.push({
        method: request.method,
        path: request.path,
        query: request.query,
        headers: request.headers,
        body: new TextDecoder().decode(request.body),
        parts: request.parts ?? null,
        status: response.status,
        answeredHeaders: response.headers,
        answeredBody: new TextDecoder().decode(response.body),
      });
      return { status: response.status, headers: response.headers, body: response.body };
    },
  };
}

async function main() {
  const http = createWharfClientServiceHttpClient(carried(createWharfClientServiceHttpDispatcher(impl)));
  const filingClient = createWharfFilingClientServiceHttpClient(
    carried(createWharfFilingClientServiceHttpDispatcher(filing)),
  );
  const ticket = 9007199254740993n;
  const answered = {
    found: await http.find({
      berth: 7, ticket, label: "007", since: 9, berths: [1, 2],
    }),
    moored: await http.moor("n", 7, ticket, "42"),
    filed: await filingClient.file({ berth: 7, ticket, label: "007" }, "stand-in"),
  };
  const routes = [...wharfClientServiceHttpRoutes, ...wharfFilingClientServiceHttpRoutes].map(
    (route) => [route.method, route.path, route.operation],
  );
  console.log(JSON.stringify({ sent, kinds, answered, routes }, (_, value) => typeof value === "bigint" ? `${value}n` : value));
  process.exit(0);
}
main().catch((error) => { console.error(error); process.exit(1); });
"#;

/// What each client's driver prints: the request line, the headers and the text parts of each call
/// its transport carried, then the `header_out` the second call read back.
#[cfg(any(feature = "dart", feature = "kotlin", feature = "swift"))]
const CARRIED: &str = "\
GET /wharf/7?ticket=9007199254740993&label=007&since=9&berths=1%2C2 [] []
POST /wharf? [x-berth=7;x-ticket=9007199254740993;x-label=42] []
POST /filed/7? [] [ticket=9007199254740993;label=007;file]
next 9007199254740994
";

#[cfg(feature = "dart")]
const DART_DRIVER: &str = "
class _Recorder implements WharfClientServiceHttpTransport, WharfFilingClientServiceHttpTransport {
  final List<String> sent = <String>[];

  @override
  Future<({int status, List<(String, String)> headers, List<int> body, Stream<List<int>> bodyStream})> send(
    ({String method, String path, String query, List<(String, String)> headers, List<int> body, List<(String, dynamic)> parts}) request,
  ) async {
    final headers = request.headers.map((header) => '${header.$1}=${header.$2}').join(';');
    final parts = request.parts.map((part) => part.$2 is String ? '${part.$1}=${part.$2}' : part.$1).join(';');
    sent.add('${request.method} ${request.path}?${request.query} [$headers] [$parts]');
    return (
      status: 200,
      headers: request.path == '/wharf' ? <(String, String)>[('x-next', '9007199254740994')] : <(String, String)>[],
      body: utf8.encode(jsonEncode('ok')),
      bodyStream: const Stream<List<int>>.empty(),
    );
  }
}

void main() async {
  final recorder = _Recorder();
  final ticket = WharfTicketNumber(9007199254740993);
  await WharfClientServiceHttpClient(recorder).find(FindRequest(
    berth: WharfBerthNumber(7),
    ticket: ticket,
    label: WharfBerthLabel('007'),
    since: WharfBerthAliasType(WharfBerthNumber(9)),
    berths: [WharfBerthNumber(1), WharfBerthNumber(2)],
  ));
  final moored = await WharfClientServiceHttpClient(recorder).moor('n', WharfBerthNumber(7), ticket, WharfBerthLabel('42'));
  await WharfFilingClientServiceHttpClient(recorder).file(
    FileRequest(berth: WharfBerthNumber(7), ticket: ticket, label: WharfBerthLabel('007')),
    <int>[1],
  );
  for (final sent in recorder.sent) {
    print(sent);
  }
  print(switch (moored) { WharfClientServiceMoorResultOk(value: (_, final next)) => 'next ${next.value}', _ => 'refused' });
}
";

#[cfg(feature = "swift")]
const SWIFT_DRIVER: &str = r#"
actor Recorder: WharfClientServiceHttpTransport, WharfFilingClientServiceHttpTransport {
  private(set) var sent: [String] = []

  private func record(_ method: String, _ path: String, _ query: String, _ headers: [(String, String)], _ parts: [(String, any Sendable)]) {
    let shownHeaders = headers.map { "\($0.0)=\($0.1)" }.joined(separator: ";")
    let shownParts = parts.map { $0.1 is String ? "\($0.0)=\($0.1)" : $0.0 }.joined(separator: ";")
    sent.append("\(method) \(path)?\(query) [\(shownHeaders)] [\(shownParts)]")
  }

  func send(_ request: WharfClientServiceHttpRequest) async throws -> WharfClientServiceHttpResponse {
    record(request.method, request.path, request.query, request.headers, [])
    let headers: [(String, String)] = request.path == "/wharf" ? [("x-next", "9007199254740994")] : []
    return WharfClientServiceHttpResponse(status: 200, headers: headers, body: Data("\"ok\"".utf8))
  }

  func send(_ request: WharfFilingClientServiceHttpRequest) async throws -> WharfFilingClientServiceHttpResponse {
    record(request.method, request.path, request.query, request.headers, request.parts)
    return WharfFilingClientServiceHttpResponse(status: 200, headers: [], body: Data("\"ok\"".utf8))
  }
}

let recorder = Recorder()
let ticket = WharfTicketNumber(value: 9007199254740993)
_ = await WharfClientServiceHttpClient(transport: recorder).find(FindRequest(
  berth: WharfBerthNumber(value: 7),
  ticket: ticket,
  label: WharfBerthLabel(value: "007"),
  since: WharfBerthNumber(value: 9),
  berths: [WharfBerthNumber(value: 1), WharfBerthNumber(value: 2)]
))
let moored = await WharfClientServiceHttpClient(transport: recorder).moor("n", berth: WharfBerthNumber(value: 7), ticket: ticket, label: WharfBerthLabel(value: "42"))
_ = await WharfFilingClientServiceHttpClient(transport: recorder).file(
  FileRequest(berth: WharfBerthNumber(value: 7), ticket: ticket, label: WharfBerthLabel(value: "007")),
  attachment: Data([1])
)
for sent in await recorder.sent {
  print(sent)
}
switch moored {
case .success((_, let next)): print("next \(next.value)")
case .failure: print("refused")
}
"#;

#[cfg(feature = "kotlin")]
const KOTLIN_DRIVER: &str = r#"
class Recorder : WharfClientServiceHttpTransport, WharfFilingClientServiceHttpTransport {
    val sent = mutableListOf<String>()

    private fun record(method: String, path: String, query: String, headers: List<Pair<String, String>>, parts: List<Pair<String, Any?>>) {
        val shownHeaders = headers.joinToString(";") { it.first + "=" + it.second }
        val shownParts = parts.joinToString(";") { if (it.second is String) it.first + "=" + it.second else it.first }
        sent.add("$method $path?$query [$shownHeaders] [$shownParts]")
    }

    override suspend fun send(request: WharfClientServiceHttpRequest): WharfClientServiceHttpResponse {
        record(request.method, request.path, request.query, request.headers, emptyList())
        val headers = if (request.path == "/wharf") listOf("x-next" to "9007199254740994") else emptyList()
        return WharfClientServiceHttpResponse(200, headers, "\"ok\"".encodeToByteArray())
    }

    override suspend fun send(request: WharfFilingClientServiceHttpRequest): WharfFilingClientServiceHttpResponse {
        record(request.method, request.path, request.query, request.headers, request.parts)
        return WharfFilingClientServiceHttpResponse(200, emptyList(), "\"ok\"".encodeToByteArray())
    }
}

fun main() = runBlocking {
    val recorder = Recorder()
    val ticket = WharfTicketNumber(9007199254740993L)
    WharfClientServiceHttpClient(recorder).find(FindRequest(
        berth = WharfBerthNumber(7u),
        ticket = ticket,
        label = WharfBerthLabel("007"),
        since = WharfBerthNumber(9u),
        berths = listOf(WharfBerthNumber(1u), WharfBerthNumber(2u)),
    ))
    val moored = WharfClientServiceHttpClient(recorder).moor("n", WharfBerthNumber(7u), ticket, WharfBerthLabel("42"))
    WharfFilingClientServiceHttpClient(recorder).file(
        FileRequest(berth = WharfBerthNumber(7u), ticket = ticket, label = WharfBerthLabel("007")),
        JsonPrimitive(1),
    )
    for (sent in recorder.sent) println(sent)
    println(when (moored) { is WharfClientServiceMoorResult.Ok -> "next " + moored.value.headerOut0.value; else -> "refused" })
}
"#;

#[cfg(feature = "kotlin")]
const KOTLIN_IMPORTS: &str = "import kotlinx.coroutines.*\n\
     import kotlinx.coroutines.flow.*\n\
     import kotlinx.serialization.*\n\
     import kotlinx.serialization.json.*\n\
     import kotlinx.serialization.descriptors.*\n\
     import kotlinx.serialization.encoding.*\n\
     import kotlinx.serialization.builtins.*";

#[model_schema()]
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(transparent)]
pub struct WharfBerthNumber(pub u32);

#[model_schema()]
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(transparent)]
pub struct WharfTicketNumber(pub i64);

#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(transparent)]
pub struct WharfBerthLabel(pub String);

#[model_schema()]
pub type WharfBerthAlias = WharfBerthNumber;

#[model_schema()]
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case", tag = "errorCode")]
pub enum WharfError {
    Refused,
}

#[service_schema(transports = ["http_rest"])]
pub trait WharfClientService<Ctx> {
    #[service_schema_op(http(method = "GET", path = "/wharf/{berth}"))]
    async fn find(
        &self,
        ctx: &Ctx,
        berth: WharfBerthNumber,
        ticket: Option<WharfTicketNumber>,
        label: Option<WharfBerthLabel>,
        since: Option<WharfBerthAlias>,
        berths: Option<Vec<WharfBerthNumber>>,
    ) -> Result<String, WharfError>;

    #[service_schema_op(http(
        method = "POST",
        path = "/wharf",
        header_in("x-berth" = berth),
        header_in("x-ticket" = ticket),
        header_in("x-label" = label),
        header_out("x-next"),
    ))]
    async fn moor(
        &self,
        ctx: &Ctx,
        note: String,
        berth: WharfBerthNumber,
        ticket: WharfTicketNumber,
        label: WharfBerthLabel,
    ) -> Result<(String, WharfTicketNumber), WharfError>;
}

#[service_schema(transports = ["http_rest"])]
pub trait WharfFilingClientService<Ctx> {
    #[service_schema_op(http(
        method = "POST",
        path = "/filed/{berth}",
        body = "multipart",
        part("file" = attachment),
    ))]
    async fn file(
        &self,
        ctx: &Ctx,
        berth: WharfBerthNumber,
        ticket: WharfTicketNumber,
        label: WharfBerthLabel,
        attachment: Box<dyn wharf_filing_client_service_schema::BodySource + Send>,
    ) -> Result<String, WharfError>;
}

/// Answers as the driver's implementations do.
pub struct WharfBackEnd;

impl WharfClientService<()> for WharfBackEnd {
    async fn find(
        &self,
        _ctx: &(),
        berth: WharfBerthNumber,
        ticket: Option<WharfTicketNumber>,
        label: Option<WharfBerthLabel>,
        since: Option<WharfBerthAlias>,
        berths: Option<Vec<WharfBerthNumber>>,
    ) -> Result<String, WharfError> {
        ready(()).await;
        let joined: Vec<String> = berths
            .unwrap_or_default()
            .iter()
            .map(|each| each.0.to_string())
            .collect();
        Ok(format!(
            "{}|{}|{}|{}|{}",
            berth.0,
            ticket.map_or(0, |each| each.0),
            label.map_or_else(String::new, |each| each.0),
            since.map_or(0, |each| each.0),
            joined.join(",")
        ))
    }

    async fn moor(
        &self,
        _ctx: &(),
        note: String,
        berth: WharfBerthNumber,
        ticket: WharfTicketNumber,
        label: WharfBerthLabel,
    ) -> Result<(String, WharfTicketNumber), WharfError> {
        ready(()).await;
        Ok((
            format!("{note}|{}|{}|{}", berth.0, ticket.0, label.0),
            WharfTicketNumber(ticket.0 + 1),
        ))
    }
}

impl WharfFilingClientService<()> for WharfBackEnd {
    async fn file(
        &self,
        _ctx: &(),
        berth: WharfBerthNumber,
        ticket: WharfTicketNumber,
        label: WharfBerthLabel,
        mut attachment: Box<dyn wharf_filing_client_service_schema::BodySource + Send>,
    ) -> Result<String, WharfError> {
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
        Ok(format!(
            "{}|{}|{}|{}",
            berth.0,
            ticket.0,
            label.0,
            String::from_utf8(drained).unwrap()
        ))
    }
}

/// Answers every fault `499`.
struct RefusedAs499;

impl wharf_http_rest_transport::FaultHandler for RefusedAs499 {
    fn on_fault(
        &self,
        _fault: &wharf_client_service_schema::ServiceFault,
    ) -> wharf_http_rest_transport::OutgoingResponse {
        wharf_http_rest_transport::OutgoingResponse::new(499, Vec::new(), b"refused".to_vec())
    }
}

impl wharf_filing_http_rest_transport::FaultHandler for RefusedAs499 {
    fn on_fault(
        &self,
        _fault: &wharf_filing_client_service_schema::ServiceFault,
    ) -> wharf_filing_http_rest_transport::OutgoingResponse {
        wharf_filing_http_rest_transport::OutgoingResponse::new(499, Vec::new(), Vec::new())
    }
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

/// The `[name, value]` pairs `value` holds, sorted.
fn pairs(value: &Value) -> Vec<(String, String)> {
    let mut held: Vec<(String, String)> = value
        .as_array()
        .unwrap()
        .iter()
        .map(|pair| {
            (
                pair[0].as_str().unwrap().to_owned(),
                pair[1].as_str().unwrap().to_owned(),
            )
        })
        .collect();
    held.sort();
    held
}

/// The status, the sorted headers and the body the Rust dispatcher answers the request the
/// TypeScript client wrote with.
fn rust_answered(request: &Value) -> (u16, Vec<(String, String)>, String) {
    let text = |key: &str| request[key].as_str().unwrap().to_owned();
    let method = text("method");
    let path = text("path");
    let query = text("query");
    let headers = pairs(&request["headers"]);
    let body = text("body").into_bytes();
    if path.starts_with("/filed/") {
        let parts = request["parts"]
            .as_array()
            .unwrap()
            .iter()
            .map(|part| {
                let (name, content) = (
                    part[0].as_str().unwrap().to_owned(),
                    part[1].as_str().unwrap().to_owned(),
                );
                let carried = if name == "file" {
                    wharf_filing_http_rest_transport::IncomingPart::File(Box::new(io::Cursor::new(
                        content.into_bytes(),
                    )))
                } else {
                    wharf_filing_http_rest_transport::IncomingPart::Text(content)
                };
                (name, carried)
            })
            .collect();
        let incoming = wharf_filing_http_rest_transport::IncomingRequest::new(
            method, path, query, headers, body,
        );
        let response = poll_once(wharf_filing_http_rest_transport::dispatch(
            &WharfBackEnd,
            &(),
            &incoming,
            parts,
            &wharf_filing_http_rest_transport::DefaultFaultHandler,
        ))
        .unwrap();
        let mut answered = response.headers().to_vec();
        answered.sort();
        return (
            response.status(),
            answered,
            String::from_utf8(response.body().to_vec()).unwrap(),
        );
    }
    let incoming =
        wharf_http_rest_transport::IncomingRequest::new(method, path, query, headers, body);
    let response = poll_once(wharf_http_rest_transport::dispatch(
        &WharfBackEnd,
        &(),
        &incoming,
        &wharf_http_rest_transport::DefaultFaultHandler,
    ))
    .unwrap();
    let mut answered = response.headers().to_vec();
    answered.sort();
    (
        response.status(),
        answered,
        String::from_utf8(response.body().to_vec()).unwrap(),
    )
}

fn driven() -> Option<Value> {
    let Some(modules) = node_modules(REQUIRED_PACKAGES) else {
        stand_down_modules(
            REQUIRED_PACKAGES,
            "the emitted TypeScript services that take a brand over a number",
        );
        return None;
    };
    let module = [
        "import { z, type ZodType, type $brand } from \"zod\";\nimport type { SomeType } from \"zod/v4/core\";"
            .to_owned(),
        WharfBerthNumber::ts_definition(),
        WharfBerthNumber::zod_schema(),
        WharfTicketNumber::ts_definition(),
        WharfTicketNumber::zod_schema(),
        WharfBerthLabel::ts_definition(),
        WharfBerthLabel::zod_schema(),
        wharf_berth_alias_schema::Schema::ts_definition(),
        wharf_berth_alias_schema::Schema::zod_schema(),
        WharfError::ts_definition(),
        WharfError::zod_schema(),
        WharfClientServiceSchema::ts_definition(),
        WharfClientServiceSchema::ts_service(),
        WharfClientServiceSchema::ts_http_client(),
        WharfClientServiceSchema::ts_http_service(),
        WharfFilingClientServiceSchema::ts_definition(),
        WharfFilingClientServiceSchema::ts_service(),
        WharfFilingClientServiceSchema::ts_http_client(),
        WharfFilingClientServiceSchema::ts_http_service(),
        DRIVER.to_owned(),
    ]
    .join("\n\n");
    let printed = ran_with_modules(
        "number-brands",
        RUNTIME_VAR,
        "node",
        "number_brands.mts",
        &module,
        &modules,
    )?;
    Some(serde_json::from_str(printed.trim()).unwrap())
}

/// The client writes each number brand as the text of the number, the dispatcher reads it back as
/// the number (a `bigint` for the `i64` brand), and the brand over text stays text.
#[test]
fn a_brand_over_a_number_is_written_and_read_as_the_text_of_the_number_by_the_typescript_clients() {
    let Some(ran) = driven() else {
        return;
    };
    let sent = ran["sent"].as_array().unwrap();
    assert_eq!(sent[0]["path"], "/wharf/7");
    for text in [
        "ticket=9007199254740993",
        "label=007",
        "since=9",
        "berths=1%2C2",
    ] {
        assert!(
            sent[0]["query"].as_str().unwrap().contains(text),
            "{}",
            sent[0]
        );
    }
    assert_eq!(
        sent[1]["headers"],
        serde_json::json!([
            ["x-berth", "7"],
            ["x-ticket", "9007199254740993"],
            ["x-label", "42"]
        ]),
        "{ran}"
    );
    assert_eq!(
        sent[2]["parts"],
        serde_json::json!([
            ["ticket", "9007199254740993"],
            ["label", "007"],
            ["file", "stand-in"]
        ])
    );
    assert_eq!(
        ran["kinds"],
        serde_json::json!([
            "number", "bigint", "number", "number", "number", "bigint", "number", "bigint"
        ]),
        "{ran}"
    );
    for carried in sent {
        let node = (
            u16::try_from(carried["status"].as_u64().unwrap()).unwrap(),
            pairs(&carried["answeredHeaders"]),
            carried["answeredBody"].as_str().unwrap().to_owned(),
        );
        assert_eq!(node, rust_answered(carried), "{carried}");
    }
    assert_eq!(
        sent[0]["answeredBody"], "\"7|9007199254740993|007|9|1,2\"",
        "{ran}"
    );
    assert_eq!(
        sent[1]["answeredBody"], "\"n|7|9007199254740993|42\"",
        "{ran}"
    );
    assert_eq!(
        pairs(&sent[1]["answeredHeaders"]),
        [
            ("content-type".to_owned(), "application/json".to_owned()),
            ("x-next".to_owned(), "9007199254740994".to_owned())
        ],
        "{ran}"
    );
    assert_eq!(
        sent[2]["answeredBody"], "\"7|9007199254740993|007|stand-in\"",
        "{ran}"
    );
}

/// The route tables name each path and operation as the Rust ones do, and a request no route
/// matches reaches the fault handler it is given.
#[test]
fn the_route_tables_and_the_rust_twins_read_back_what_they_were_built_with() {
    let Some(ran) = driven() else {
        return;
    };
    let rust_routes: Vec<Value> = wharf_http_rest_transport::ROUTES
        .iter()
        .map(|route| serde_json::json!([route.method(), route.path(), route.operation()]))
        .chain(
            wharf_filing_http_rest_transport::ROUTES
                .iter()
                .map(|route| serde_json::json!([route.method(), route.path(), route.operation()])),
        )
        .collect();
    assert_eq!(ran["routes"], Value::Array(rust_routes));
    let statuses: Vec<(u16, Vec<u16>)> = wharf_http_rest_transport::ROUTES
        .iter()
        .map(|route| (route.ok_status(), route.error_statuses().to_vec()))
        .chain(
            wharf_filing_http_rest_transport::ROUTES
                .iter()
                .map(|route| (route.ok_status(), route.error_statuses().to_vec())),
        )
        .collect();
    assert_eq!(
        statuses,
        [(200, vec![422]), (200, vec![422]), (200, vec![422])]
    );
    let headers = vec![("x-trace".to_owned(), "abc".to_owned())];
    let plain = wharf_http_rest_transport::IncomingRequest::new(
        "GET".to_owned(),
        "/nowhere".to_owned(),
        "unused=1".to_owned(),
        headers.clone(),
        b"ignored".to_vec(),
    );
    let filing = wharf_filing_http_rest_transport::IncomingRequest::new(
        "GET".to_owned(),
        "/nowhere".to_owned(),
        "unused=1".to_owned(),
        headers.clone(),
        b"ignored".to_vec(),
    );
    for read_back in [
        (
            plain.body(),
            plain.query(),
            plain.headers(),
            plain.header("X-Trace"),
        ),
        (
            filing.body(),
            filing.query(),
            filing.headers(),
            filing.header("X-Trace"),
        ),
    ] {
        assert_eq!(
            read_back,
            (&b"ignored"[..], "unused=1", headers.as_slice(), Some("abc"))
        );
    }
    let refused = poll_once(wharf_http_rest_transport::dispatch(
        &WharfBackEnd,
        &(),
        &plain,
        &RefusedAs499,
    ))
    .unwrap();
    let refused_filing = poll_once(wharf_filing_http_rest_transport::dispatch(
        &WharfBackEnd,
        &(),
        &filing,
        Vec::new(),
        &RefusedAs499,
    ))
    .unwrap();
    assert_eq!((refused.status(), refused.body()), (499, &b"refused"[..]));
    assert_eq!(refused.headers(), &[] as &[(String, String)]);
    assert_eq!(refused_filing.status(), 499);
}

#[cfg(feature = "dart")]
#[test]
fn the_dart_client_carries_a_brand_over_a_number_as_the_text_of_the_number() {
    let module = [
        "import 'dart:async';".to_owned(),
        "import 'dart:convert';".to_owned(),
        wharf_error_dart::dart_definition(),
        wharf_berth_number_dart::dart_definition(),
        wharf_ticket_number_dart::dart_definition(),
        wharf_berth_label_dart::dart_definition(),
        wharf_berth_alias_dart::dart_definition(),
        WharfClientServiceSchema::dart_definition(),
        WharfClientServiceSchema::dart_http_client(),
        WharfFilingClientServiceSchema::dart_definition(),
        WharfFilingClientServiceSchema::dart_http_client(),
        DART_DRIVER.to_owned(),
    ]
    .join("\n\n");
    let Some(printed) = ran("dart", "TIXSCHEMA_DART", "dart", "client.dart", &module) else {
        return;
    };
    assert_eq!(printed, CARRIED, "the Dart client carries other text");
    let Some((clean, said)) = dart("number-brands-analyze", &["analyze"], &module) else {
        return;
    };
    assert!(clean && said.contains("No issues found!"), "{said}");
}

#[cfg(feature = "swift")]
#[test]
fn the_swift_client_carries_a_brand_over_a_number_as_the_text_of_the_number() {
    use super::runtime::judged;
    use wharf_client_service_schema::{
        wharf_client_service_fault_fields_swift, wharf_client_service_fault_kind_swift,
    };
    use wharf_filing_client_service_schema::{
        wharf_filing_client_service_fault_fields_swift,
        wharf_filing_client_service_fault_kind_swift,
    };

    let module = [
        "import Foundation".to_owned(),
        wharf_error_swift::swift_definition(),
        wharf_berth_number_swift::swift_definition(),
        wharf_ticket_number_swift::swift_definition(),
        wharf_berth_label_swift::swift_definition(),
        wharf_berth_alias_swift::swift_definition(),
        find_request_swift::swift_definition(),
        file_request_swift::swift_definition(),
        wharf_client_service_fault_fields_swift::swift_definition(),
        wharf_client_service_fault_kind_swift::swift_definition(),
        wharf_filing_client_service_fault_fields_swift::swift_definition(),
        wharf_filing_client_service_fault_kind_swift::swift_definition(),
        WharfClientServiceSchema::swift_http_client(),
        WharfFilingClientServiceSchema::swift_http_client(),
        SWIFT_DRIVER.to_owned(),
    ]
    .join("\n\n");
    let Some(run) = judged(
        "swift-number-brands",
        "TIXSCHEMA_SWIFT",
        "swift",
        &["-warnings-as-errors"],
        "main.swift",
        &module,
    ) else {
        return;
    };
    assert!(
        run.succeeded,
        "`swift -warnings-as-errors main.swift` failed.\n--- stdout ---\n{}\n--- stderr ---\n{}",
        run.stdout, run.stderr
    );
    assert_eq!(run.stdout, CARRIED, "the Swift client carries other text");
}

#[cfg(feature = "kotlin")]
#[test]
fn the_kotlin_client_carries_a_brand_over_a_number_as_the_text_of_the_number() {
    use wharf_client_service_schema::{
        wharf_client_service_fault_fields_kotlin, wharf_client_service_fault_kind_kotlin,
    };
    use wharf_filing_client_service_schema::{
        wharf_filing_client_service_fault_fields_kotlin,
        wharf_filing_client_service_fault_kind_kotlin,
    };

    let module = [
        KOTLIN_IMPORTS.to_owned(),
        wharf_error_kotlin::kotlin_definition(),
        wharf_berth_number_kotlin::kotlin_definition(),
        wharf_ticket_number_kotlin::kotlin_definition(),
        wharf_berth_label_kotlin::kotlin_definition(),
        wharf_berth_alias_kotlin::kotlin_definition(),
        find_request_kotlin::kotlin_definition(),
        file_request_kotlin::kotlin_definition(),
        wharf_client_service_fault_fields_kotlin::kotlin_definition(),
        wharf_client_service_fault_kind_kotlin::kotlin_definition(),
        wharf_filing_client_service_fault_fields_kotlin::kotlin_definition(),
        wharf_filing_client_service_fault_kind_kotlin::kotlin_definition(),
        WharfClientServiceSchema::kotlin_http_client(),
        WharfFilingClientServiceSchema::kotlin_http_client(),
        KOTLIN_DRIVER.to_owned(),
    ]
    .join("\n\n");
    let Some(printed) = ran_kotlin(&module) else {
        return;
    };
    assert_eq!(printed, CARRIED, "the Kotlin client carries other text");
}
