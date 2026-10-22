//! A service whose header names, part names, message names and paths hold a `$`, a quote, a
//! double quote and a backslash, each a character a Dart, a Swift or a Kotlin string literal reads
//! apart from text, and a message name that holds a line break, which ends a line comment, called
//! through each emitted `http_rest` client: each name reaches the transport as it was declared, and
//! the `ws_rpc` client and dispatcher beside it compile. The Rust dispatcher of a service with the
//! same operation routes the path the clients carry for the line break.

#![cfg(any(feature = "dart", feature = "swift", feature = "kotlin"))]

use core::future::{Future, ready};
use core::pin::pin;
use core::task::{Context as PollContext, Poll, Waker};
use std::io::Cursor;
use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use tixschema::{model_schema, service_schema};

use super::run_node_quoted::rust_noted;

#[cfg(feature = "dart")]
use super::run_dart_validate::dart;
#[cfg(feature = "dart")]
use super::runtime::ran;
#[cfg(feature = "kotlin")]
use super::runtime::ran_kotlin;

/// The message name `note` declares, which names its path too.
const NOTE_WIRE: &str = "no*/te\n\\";

/// What each client's driver prints: the header names, the part names and the path of each call
/// its transport carried, under keys in their order, the header the first call answered, and the
/// operation the fault of the second call names, which is the message name.
const CARRIED: &str = r#"{"headers":["x-co$t","x-it's"],"parts":[],"path":"/co$t/it's/l"}
{"headers":[],"parts":[],"path":"/le$vy'it\"q\\x/l"}
{"headers":[],"parts":["title","note","fi$le'x\"q\\"],"path":"/deposit"}
{"headers":[],"parts":[],"path":"/no*/te\n\\"}
answered aged
fault in le$vy'it"q\x
"#;

/// Records each call and answers it, the second by throwing.
#[cfg(feature = "dart")]
const DART_DRIVER: &str = r"
class _Recorder implements TollingClientServiceHttpTransport, TollingUploadClientServiceHttpTransport {
  final List<String> sent = <String>[];

  @override
  Future<({int status, List<(String, String)> headers, List<int> body, Stream<List<int>> bodyStream})> send(
    ({String method, String path, String query, List<(String, String)> headers, List<int> body, List<(String, dynamic)> parts}) request,
  ) async {
    sent.add(jsonEncode(<String, dynamic>{
      'headers': [for (final (name, _) in request.headers) name],
      'parts': [for (final (name, _) in request.parts) name],
      'path': request.path,
    }));
    if (request.path.startsWith('/le')) {
      throw StateError('uncarried');
    }
    if (request.path.startsWith('/no')) {
      return (status: 204, headers: <(String, String)>[], body: <int>[], bodyStream: const Stream<List<int>>.empty());
    }
    return (
      status: 200,
      headers: <(String, String)>[('x-ag\$e', 'aged')],
      body: utf8.encode(jsonEncode(<String, dynamic>{'label': 'l'})),
      bodyStream: const Stream<List<int>>.empty(),
    );
  }
}

void main() async {
  final recorder = _Recorder();
  final tolled = await TollingClientServiceHttpClient(recorder).toll('l', 't', 7);
  final levied = await TollingClientServiceHttpClient(recorder).levy(LevyRequest(label: 'l', note: 'n'));
  await TollingUploadClientServiceHttpClient(recorder).deposit(DepositRequest(title: 't', note: 'n'), <int>[1]);
  await TollingClientServiceHttpClient(recorder).note('l');
  for (final sent in recorder.sent) {
    print(sent);
  }
  print(switch (tolled) { TollingClientServiceTollResultOk(value: (_, final aged)) => 'answered $aged', _ => 'refused' });
  print(switch (levied) { TollingClientServiceLevyResultFault(:final fault) => 'fault in ${fault.operation}', _ => 'answered' });
}
";

/// The Swift twin of [`DART_DRIVER`].
#[cfg(feature = "swift")]
const SWIFT_DRIVER: &str = r##"
struct Sent: Codable {
  let path: String
  let headers: [String]
  let parts: [String]
}

actor Recorder: TollingClientServiceHttpTransport, TollingUploadClientServiceHttpTransport {
  private(set) var sent: [Sent] = []

  func send(_ request: TollingClientServiceHttpRequest) async throws -> TollingClientServiceHttpResponse {
    sent.append(Sent(path: request.path, headers: request.headers.map { $0.0 }, parts: []))
    if request.path.hasPrefix("/le") {
      throw CancellationError()
    }
    if request.path.hasPrefix("/no") {
      return TollingClientServiceHttpResponse(status: 204, headers: [], body: Data())
    }
    return TollingClientServiceHttpResponse(status: 200, headers: [("x-ag$e", "aged")], body: Data(#"{"label":"l"}"#.utf8))
  }

  func send(_ request: TollingUploadClientServiceHttpRequest) async throws -> TollingUploadClientServiceHttpResponse {
    sent.append(Sent(path: request.path, headers: request.headers.map { $0.0 }, parts: request.parts.map { $0.0 }))
    return TollingUploadClientServiceHttpResponse(status: 200, headers: [], body: Data(#"{"label":"l"}"#.utf8))
  }
}

let recorder = Recorder()
let tolled = await TollingClientServiceHttpClient(transport: recorder).toll("l", tenant: "t", trace: 7)
let levied = await TollingClientServiceHttpClient(transport: recorder).levy(LevyRequest(label: "l", note: "n"))
_ = await TollingUploadClientServiceHttpClient(transport: recorder).deposit(DepositRequest(title: "t", note: "n"), attachment: Data([1]))
try await TollingClientServiceHttpClient(transport: recorder).note("l")
let encoder = JSONEncoder()
encoder.outputFormatting = [.sortedKeys, .withoutEscapingSlashes]
for sent in await recorder.sent {
  print(String(data: try! encoder.encode(sent), encoding: .utf8)!)
}
switch tolled {
case .success((_, let aged)): print("answered \(aged)")
case .failure: print("refused")
}
switch levied {
case .failure(.fault(let fault)): print("fault in \(fault.operation)")
default: print("answered")
}
"##;

/// The Kotlin twin of [`DART_DRIVER`].
#[cfg(feature = "kotlin")]
const KOTLIN_DRIVER: &str = r#"
class Recorder : TollingClientServiceHttpTransport, TollingUploadClientServiceHttpTransport {
    val sent = mutableListOf<JsonObject>()

    override suspend fun send(request: TollingClientServiceHttpRequest): TollingClientServiceHttpResponse {
        sent.add(recorded(request.path, request.headers.map { it.first }, emptyList()))
        if (request.path.startsWith("/le")) throw IllegalStateException("uncarried")
        if (request.path.startsWith("/no")) return TollingClientServiceHttpResponse(204, emptyList(), ByteArray(0))
        return TollingClientServiceHttpResponse(200, listOf("x-ag${'$'}e" to "aged"), """{"label":"l"}""".encodeToByteArray())
    }

    override suspend fun send(request: TollingUploadClientServiceHttpRequest): TollingUploadClientServiceHttpResponse {
        sent.add(recorded(request.path, request.headers.map { it.first }, request.parts.map { it.first }))
        return TollingUploadClientServiceHttpResponse(200, emptyList(), """{"label":"l"}""".encodeToByteArray())
    }

    private fun recorded(path: String, headers: List<String>, parts: List<String>): JsonObject = buildJsonObject {
        put("headers", JsonArray(headers.map { JsonPrimitive(it) }))
        put("parts", JsonArray(parts.map { JsonPrimitive(it) }))
        put("path", path)
    }
}

fun main() = runBlocking {
    val recorder = Recorder()
    val tolled = TollingClientServiceHttpClient(recorder).toll("l", "t", 7u)
    val levied = TollingClientServiceHttpClient(recorder).levy(LevyRequest(label = "l", note = "n"))
    TollingUploadClientServiceHttpClient(recorder).deposit(DepositRequest(title = "t", note = "n"), JsonPrimitive(1))
    TollingClientServiceHttpClient(recorder).note("l")
    for (sent in recorder.sent) println(sent.toString())
    println(when (tolled) { is TollingClientServiceTollResult.Ok -> "answered " + tolled.value.headerOut0; else -> "refused" })
    println(when (levied) { is TollingClientServiceLevyResult.Fault -> "fault in " + levied.fault.operation; else -> "answered" })
}
"#;

/// The imports the emitted Kotlin and its driver reach for.
#[cfg(feature = "kotlin")]
const KOTLIN_IMPORTS: &str = "import kotlinx.coroutines.*\n\
     import kotlinx.coroutines.flow.*\n\
     import kotlinx.serialization.*\n\
     import kotlinx.serialization.json.*\n\
     import kotlinx.serialization.descriptors.*\n\
     import kotlinx.serialization.encoding.*\n\
     import kotlinx.serialization.builtins.*";

/// The answer a call reads.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct TollingReceipt {
    pub label: String,
}

/// The error a call declares.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "errorCode")]
pub enum TollingError {
    Refused,
}

/// Header names and a path holding a `$` and a quote, a message name and a path holding a double
/// quote and a backslash as well, and a one-way operation whose message name, and so its path,
/// holds a line break.
#[service_schema(transports = ["http_rest", "ws_rpc"])]
pub trait TollingClientService<Ctx> {
    #[service_schema_op(
        message = "le$vy'it\"q\\x",
        http(method = "POST", path = "/le$vy'it\"q\\x/{label}")
    )]
    async fn levy(
        &self,
        ctx: &Ctx,
        label: String,
        note: String,
    ) -> Result<TollingReceipt, TollingError>;

    #[service_schema_op(message = "no*/te\n\\", one_way)]
    async fn note(&self, ctx: &Ctx, label: String);

    #[service_schema_op(http(
        method = "POST",
        path = "/co$t/it's/{label}",
        header_in("x-co$t" = tenant),
        header_in("x-it's" = trace),
        header_out("x-ag$e"),
        error_status(Refused = 409),
        error_header_out("x-rea$on"),
    ))]
    async fn toll(
        &self,
        ctx: &Ctx,
        label: String,
        tenant: String,
        trace: Option<u32>,
    ) -> Result<(TollingReceipt, String), (TollingError, Option<String>)>;
}

/// A part name holding a `$`, a quote, a double quote and a backslash.
#[service_schema(transports = ["http_rest"])]
pub trait TollingUploadClientService<Ctx> {
    #[service_schema_op(http(
        method = "POST",
        path = "/deposit",
        body = "multipart",
        part("fi$le'x\"q\\" = attachment),
    ))]
    async fn deposit(
        &self,
        ctx: &Ctx,
        title: String,
        note: String,
        attachment: Box<dyn tolling_upload_client_service_schema::BodySource + Send>,
    ) -> Result<TollingReceipt, TollingError>;
}

/// A back end that answers each operation from what it is handed, and holds the label of each
/// one-way call.
#[derive(Default)]
pub struct Tollbooth {
    noted: Mutex<Vec<String>>,
}

impl TollingClientService<()> for Tollbooth {
    async fn levy(
        &self,
        _ctx: &(),
        label: String,
        note: String,
    ) -> Result<TollingReceipt, TollingError> {
        ready(()).await;
        if note.is_empty() {
            return Err(TollingError::Refused);
        }
        Ok(TollingReceipt { label })
    }

    async fn note(&self, _ctx: &(), label: String) {
        ready(()).await;
        self.noted.lock().unwrap().push(label);
    }

    async fn toll(
        &self,
        _ctx: &(),
        label: String,
        tenant: String,
        trace: Option<u32>,
    ) -> Result<(TollingReceipt, String), (TollingError, Option<String>)> {
        ready(()).await;
        Ok((
            TollingReceipt { label },
            format!("{tenant}{}", trace.unwrap_or_default()),
        ))
    }
}

impl TollingUploadClientService<()> for Tollbooth {
    async fn deposit(
        &self,
        _ctx: &(),
        title: String,
        note: String,
        mut attachment: Box<dyn tolling_upload_client_service_schema::BodySource + Send>,
    ) -> Result<TollingReceipt, TollingError> {
        ready(()).await;
        let mut held = vec![0_u8; 8];
        let pulled = attachment.pull(&mut held).unwrap_or_default();
        Ok(TollingReceipt {
            label: format!("{title}{note}{pulled}"),
        })
    }
}

/// The back end never suspends, so one poll answers it.
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

#[cfg(feature = "dart")]
#[test]
fn the_dart_client_carries_each_name_as_it_was_declared() {
    let module = [
        "import 'dart:async';".to_owned(),
        "import 'dart:convert';".to_owned(),
        tolling_receipt_dart::dart_definition(),
        tolling_error_dart::dart_definition(),
        TollingClientServiceSchema::dart_definition(),
        TollingClientServiceSchema::dart_http_client(),
        TollingClientServiceSchema::dart_ws_client(),
        TollingUploadClientServiceSchema::dart_definition(),
        TollingUploadClientServiceSchema::dart_http_client(),
        DART_DRIVER.to_owned(),
    ]
    .join("\n\n");
    let Some(printed) = ran("dart", "TIXSCHEMA_DART", "dart", "client.dart", &module) else {
        return;
    };
    assert_eq!(printed, CARRIED, "the Dart client carries other names");
    let Some((clean, said)) = dart("quoted-service-analyze", &["analyze"], &module) else {
        return;
    };
    assert!(clean && said.contains("No issues found!"), "{said}");
}

#[cfg(feature = "swift")]
#[test]
fn the_swift_client_carries_each_name_as_it_was_declared() {
    use super::runtime::judged;
    use tolling_client_service_schema::{
        tolling_client_service_fault_fields_swift, tolling_client_service_fault_kind_swift,
    };
    use tolling_upload_client_service_schema::{
        tolling_upload_client_service_fault_fields_swift,
        tolling_upload_client_service_fault_kind_swift,
    };

    let module = [
        "import Foundation".to_owned(),
        tolling_receipt_swift::swift_definition(),
        tolling_error_swift::swift_definition(),
        levy_request_swift::swift_definition(),
        deposit_request_swift::swift_definition(),
        tolling_client_service_fault_fields_swift::swift_definition(),
        tolling_client_service_fault_kind_swift::swift_definition(),
        tolling_upload_client_service_fault_fields_swift::swift_definition(),
        tolling_upload_client_service_fault_kind_swift::swift_definition(),
        TollingClientServiceSchema::swift_http_client(),
        TollingClientServiceSchema::swift_ws_client(),
        TollingUploadClientServiceSchema::swift_http_client(),
        SWIFT_DRIVER.to_owned(),
    ]
    .join("\n\n");
    let Some(run) = judged(
        "swift-quoted-service",
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
    assert_eq!(run.stdout, CARRIED, "the Swift client carries other names");
}

#[cfg(feature = "kotlin")]
#[test]
fn the_kotlin_client_carries_each_name_as_it_was_declared() {
    use tolling_client_service_schema::{
        tolling_client_service_fault_fields_kotlin, tolling_client_service_fault_kind_kotlin,
    };
    use tolling_upload_client_service_schema::{
        tolling_upload_client_service_fault_fields_kotlin,
        tolling_upload_client_service_fault_kind_kotlin,
    };

    let module = [
        KOTLIN_IMPORTS.to_owned(),
        tolling_receipt_kotlin::kotlin_definition(),
        tolling_error_kotlin::kotlin_definition(),
        levy_request_kotlin::kotlin_definition(),
        deposit_request_kotlin::kotlin_definition(),
        tolling_client_service_fault_fields_kotlin::kotlin_definition(),
        tolling_client_service_fault_kind_kotlin::kotlin_definition(),
        tolling_upload_client_service_fault_fields_kotlin::kotlin_definition(),
        tolling_upload_client_service_fault_kind_kotlin::kotlin_definition(),
        TollingClientServiceSchema::kotlin_http_client(),
        TollingClientServiceSchema::kotlin_ws_client(),
        TollingUploadClientServiceSchema::kotlin_http_client(),
        KOTLIN_DRIVER.to_owned(),
    ]
    .join("\n\n");
    let Some(printed) = ran_kotlin(&module) else {
        return;
    };
    assert_eq!(printed, CARRIED, "the Kotlin client carries other names");
}

/// The Rust dispatcher of a service that declares the same one-way operation under the same
/// message name routes the path each client carried for it, and hands its back end the label.
#[test]
fn the_rust_dispatcher_routes_the_path_the_clients_carry_for_a_line_break() {
    let carried: Value = serde_json::from_str(CARRIED.lines().nth(3).unwrap()).unwrap();
    let path = carried["path"].as_str().unwrap();
    assert_eq!(
        path,
        format!("/{NOTE_WIRE}"),
        "the clients carry another path"
    );
    assert_eq!(
        rust_noted(path, serde_json::to_vec("l").unwrap()),
        (204, vec!["l".to_owned()]),
        "the Rust dispatcher answers the path otherwise"
    );
}

/// The back end the clients' calls stand for answers each operation from what it is handed.
#[test]
fn the_back_end_answers_each_operation_from_what_it_is_handed() {
    let tolled = poll_once(Tollbooth::default().toll(&(), "l".to_owned(), "t".to_owned(), Some(7)));
    assert_eq!(
        tolled,
        Some(Ok((
            TollingReceipt {
                label: "l".to_owned()
            },
            "t7".to_owned()
        ))),
        "toll answers otherwise"
    );
    let levied = poll_once(Tollbooth::default().levy(&(), "l".to_owned(), String::new()));
    assert_eq!(
        levied,
        Some(Err(TollingError::Refused)),
        "levy answers otherwise"
    );
    let tollbooth = Tollbooth::default();
    assert_eq!(
        poll_once(tollbooth.note(&(), "l".to_owned())),
        Some(()),
        "note answers otherwise"
    );
    assert_eq!(
        *tollbooth.noted.lock().unwrap(),
        ["l"],
        "note holds another label"
    );
    let deposited = poll_once(Tollbooth::default().deposit(
        &(),
        "t".to_owned(),
        "n".to_owned(),
        Box::new(Cursor::new(vec![1_u8])),
    ));
    assert_eq!(
        deposited,
        Some(Ok(TollingReceipt {
            label: "tn1".to_owned()
        })),
        "deposit answers otherwise"
    );
}
